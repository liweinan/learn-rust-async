use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::thread;
use std::time::Duration;

/// 自定义 Future：演示 Waker 的实际用途
/// 
/// 这个 Future 模拟一个异步操作：
/// 1. 第一次 poll 时，保存 waker 并返回 Pending
/// 2. 在后台线程中，模拟异步操作（等待一段时间）
/// 3. 操作完成后，调用 waker 通知 executor
/// 4. Executor 收到通知后，再次 poll，这次返回 Ready
pub struct AsyncTimerFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

struct SharedState {
    completed: bool,
    waker: Option<Waker>,
}

impl AsyncTimerFuture {
    pub fn new(duration: Duration) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
        }));

        // 在后台线程中模拟异步操作
        let state_clone = shared_state.clone();
        thread::spawn(move || {
            // 模拟异步操作：等待指定时间
            thread::sleep(duration);
            
            // 操作完成，设置标志并唤醒任务
            let mut state = state_clone.lock().unwrap();
            state.completed = true;
            
            // 关键：调用 waker 通知 executor 可以重新 poll 了
            if let Some(waker) = state.waker.take() {
                println!("[后台线程] 异步操作完成，唤醒 executor...");
                waker.wake();
            }
        });

        Self { shared_state }
    }
}

impl Future for AsyncTimerFuture {
    type Output = &'static str;
    
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.shared_state.lock().unwrap();
        
        if state.completed {
            // 操作已完成，返回结果
            Poll::Ready("异步操作完成！")
        } else {
            // 操作未完成，保存 waker 以便后续唤醒
            // 注意：这里只保存一次，避免重复保存
            state.waker = Some(cx.waker().clone());
            println!("[poll] Future 未就绪，保存 waker 并返回 Pending");
            Poll::Pending
        }
    }
}

/// 测试自定义 Waker 的实际用途（使用 tokio 运行时）
pub async fn test_custom_waker() {
    println!("\n=== 自定义 Waker 示例：展示实际用途 ===");
    
    println!("\n场景：模拟一个异步定时器 Future");
    println!("1. Future 在后台线程中等待 1 秒");
    println!("2. 第一次 poll 返回 Pending，并保存 waker");
    println!("3. 1 秒后，后台线程调用 waker.wake()");
    println!("4. Tokio executor 收到通知，重新 poll future");
    println!("5. 这次 poll 返回 Ready\n");
    
    let future = AsyncTimerFuture::new(Duration::from_secs(1));
    
    let start = std::time::Instant::now();
    let result = future.await;
    let elapsed = start.elapsed();
    
    println!("\n结果: {}", result);
    println!("总耗时: {:?} (包含等待时间)", elapsed);
    
    println!("\n关键点：");
    println!("- Waker 是连接异步操作完成和 executor 重新 poll 的桥梁");
    println!("- 当异步操作（I/O、定时器等）完成时，调用 waker.wake()");
    println!("- Executor 收到通知后，知道可以重新 poll 这个 future 了");
    println!("- 这样避免了轮询，提高了效率");
    println!("\n注意：在实际使用中，你不需要手动创建 waker，");
    println!("tokio 等运行时会自动处理。这个例子展示了底层机制。");
}



// Waker vtable 的回调函数（模块级别）
// 这些函数用于 SimpleExecutor，展示了如何手动创建 waker
#[allow(dead_code)]
unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
    let arc = unsafe { Arc::from_raw(ptr as *const Arc<(Mutex<bool>, Condvar)>) };
    let clone = Arc::clone(&arc);
    std::mem::forget(arc);
    RawWaker::new(
        Arc::into_raw(clone) as *const (),
        &WAKE_VTABLE,
    )
}

#[allow(dead_code)]
unsafe fn wake_waker(ptr: *const ()) {
    let arc = unsafe { Arc::from_raw(ptr as *const Arc<(Mutex<bool>, Condvar)>) };
    let (lock, cvar) = &**arc;
    *lock.lock().unwrap() = true;
    cvar.notify_one();
    std::mem::forget(arc);
}

#[allow(dead_code)]
unsafe fn wake_by_ref_waker(ptr: *const ()) {
    let arc = unsafe { Arc::from_raw(ptr as *const Arc<(Mutex<bool>, Condvar)>) };
    let (lock, cvar) = &**arc;
    *lock.lock().unwrap() = true;
    cvar.notify_one();
    std::mem::forget(arc);
}

#[allow(dead_code)]
unsafe fn drop_waker(ptr: *const ()) {
    drop(unsafe { Arc::from_raw(ptr as *const Arc<(Mutex<bool>, Condvar)>) });
}

#[allow(dead_code)]
const WAKE_VTABLE: RawWakerVTable = RawWakerVTable::new(
    clone_waker,
    wake_waker,
    wake_by_ref_waker,
    drop_waker,
);

/// 简单的 executor：演示如何使用 waker
///
/// 这是一个极简的 executor，实际运行时（如 tokio）会更复杂
/// 保留此代码作为参考，展示如何手动创建 executor
///
/// # 重要限制：阻塞问题
///
/// **这个实现存在严重的局限性**：
///
/// 1. **单任务阻塞**：当 future 返回 `Poll::Pending` 时，executor 会在 `cvar.wait()` 上阻塞
///    - 如果这是单线程 executor，整个线程被占用
///    - 无法执行其他 future，无法并发处理多个任务
///
/// 2. **为什么还能工作**：
///    - `block_on` 只运行一个 future，是单任务场景
///    - 这是一个教学示例，用于展示 waker 的基本机制
///
/// # 实际运行时的设计（如 Tokio）
///
/// 实际运行时不会这样阻塞：
///
/// 1. **事件驱动架构**：
///    - Future 返回 `Pending` 时，不阻塞线程
///    - 将 future 放入等待队列，继续执行其他 ready 的 future
///    - 使用事件循环（如 epoll/kqueue）监听 I/O 事件
///
/// 2. **Waker 的作用**：
///    - 当异步操作完成时，`waker.wake()` 被调用
///    - 将对应的 future 标记为 ready，加入就绪队列
///    - Executor 在下次事件循环中会重新 poll 这些 future
///
/// 3. **多任务支持**：
///    - 可以同时管理数千个 future
///    - 通过任务调度器在 ready 的 future 之间切换
///    - 不会因为一个 future 等待而阻塞其他 future
///
/// # 总结
///
/// 这个 `SimpleExecutor` 是教学示例，展示了 waker 如何通知 executor 重新 poll，
/// 但实际运行时需要非阻塞的事件驱动架构来支持并发执行多个 future。
#[allow(dead_code)]
pub struct SimpleExecutor {
    pair: Arc<(Mutex<bool>, Condvar)>,
}

#[allow(dead_code)]
impl SimpleExecutor {
    pub fn new() -> Self {
        Self {
            pair: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    /// 创建一个 waker，当被唤醒时会设置标志位并通知条件变量
    fn create_waker(&self) -> Waker {
        unsafe {
            Waker::from_raw(RawWaker::new(
                Arc::into_raw(self.pair.clone()) as *const (),
                &WAKE_VTABLE,
            ))
        }
    }

    /// 运行 future 直到完成
    ///
    /// # 工作流程
    ///
    /// 1. 循环 poll future
    /// 2. 如果返回 `Ready`，返回结果
    /// 3. 如果返回 `Pending`，阻塞等待 waker 唤醒
    /// 4. 被唤醒后，继续循环重新 poll
    ///
    /// # 阻塞问题
    ///
    /// **注意**：当 future 返回 `Pending` 时，这里会阻塞整个线程：
    ///
    /// ```rust
    /// while !*woken {
    ///     woken = cvar.wait(woken).unwrap();  // 线程在这里阻塞！
    /// }
    /// ```
    ///
    /// 这意味着：
    /// - 如果这是单线程 executor，无法执行其他 future
    /// - 实际运行时不会这样设计，而是使用非阻塞的事件循环
    ///
    /// # Waker 如何触发重新 poll
    ///
    /// 1. Future 在 `poll` 中保存 waker（通过 `cx.waker().clone()`）
    /// 2. 异步操作完成后，调用 `waker.wake()`
    /// 3. `wake()` 会执行 `wake_waker` 回调：
    ///    - 设置 `woken = true`
    ///    - 调用 `cvar.notify_one()` 唤醒等待的线程
    /// 4. Executor 被唤醒，退出 `while` 循环
    /// 5. 继续 `loop`，再次调用 `poll`，这次返回 `Ready`
    pub fn block_on<F: Future>(&self, mut future: F) -> F::Output {
        let waker = self.create_waker();
        let mut cx = Context::from_waker(&waker);
        let mut future = unsafe { Pin::new_unchecked(&mut future) };

        loop {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(result) => return result,
                Poll::Pending => {
                    // 等待被唤醒
                    println!("[executor] Future 返回 Pending，等待唤醒...");

                    let (lock, cvar) = &*self.pair;
                    let mut woken = lock.lock().unwrap();
                    // 注意：这里会阻塞整个线程，无法执行其他 future
                    // 实际运行时不会这样设计
                    while !*woken {
                        woken = cvar.wait(woken).unwrap();
                    }
                    *woken = false;

                    println!("[executor] 收到唤醒信号，重新 poll future");
                }
            }
        }
    }
}

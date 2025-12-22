use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::Duration;

// 导入 AsyncTimerFuture 用于演示
use super::custom_waker::AsyncTimerFuture;

// Waker vtable 的回调函数（模块级别）
// 这些函数用于 SimpleExecutor，展示了如何手动创建 waker
// 注意：这些函数虽然看起来"未使用"，但实际上被 WAKE_VTABLE 引用
unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
    let arc = unsafe { Arc::from_raw(ptr as *const (Mutex<bool>, Condvar)) };
    let clone = Arc::clone(&arc);
    std::mem::forget(arc);
    RawWaker::new(
        Arc::into_raw(clone) as *const (),
        &WAKE_VTABLE,
    )
}

unsafe fn wake_waker(ptr: *const ()) {
    // 从原始指针恢复 Arc
    let arc = unsafe { Arc::from_raw(ptr as *const (Mutex<bool>, Condvar)) };
    let (lock, cvar) = &*arc;
    
    // 设置唤醒标志并通知等待的线程
    {
        let mut woken = lock.lock().unwrap();
        *woken = true;
    }
    cvar.notify_one();
    
    // 不要 drop arc，因为它是从 into_raw 创建的
    std::mem::forget(arc);
}

unsafe fn wake_by_ref_waker(ptr: *const ()) {
    let arc = unsafe { Arc::from_raw(ptr as *const (Mutex<bool>, Condvar)) };
    let (lock, cvar) = &*arc;
    {
        let mut woken = lock.lock().unwrap();
        *woken = true;
    }
    cvar.notify_one();
    std::mem::forget(arc);
}

unsafe fn drop_waker(ptr: *const ()) {
    drop(unsafe { Arc::from_raw(ptr as *const (Mutex<bool>, Condvar)) });
}

const WAKE_VTABLE: RawWakerVTable = RawWakerVTable::new(
    clone_waker,
    wake_waker,
    wake_by_ref_waker,
    drop_waker,
);

/// 简单的 executor：演示如何使用 waker
///
/// 这是一个极简的 executor，实际运行时（如 tokio）会更复杂
/// 这个例子展示了如何手动创建 executor 和 waker
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
pub struct SimpleExecutor {
    pair: Arc<(Mutex<bool>, Condvar)>,
}

impl SimpleExecutor {
    pub fn new() -> Self {
        Self {
            pair: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    /// 创建一个 waker，当被唤醒时会设置标志位并通知条件变量
    fn create_waker(&self) -> Waker {
        // 克隆 Arc，然后转换为原始指针
        // 注意：self.pair 是 Arc<(Mutex<bool>, Condvar)>，clone() 后得到新的 Arc
        let arc_clone = self.pair.clone();
        unsafe {
            Waker::from_raw(RawWaker::new(
                Arc::into_raw(arc_clone) as *const (),
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
                    // 注意：这里会阻塞整个线程，无法执行其他 future
                    // 实际运行时不会这样设计
                    let mut woken = lock.lock().unwrap();
                    // 检查是否已经被唤醒（可能在获取锁之前就已经被唤醒了）
                    while !*woken {
                        // wait 会释放锁并等待，被唤醒后会重新获取锁
                        woken = cvar.wait(woken).unwrap();
                    }
                    // 重置标志，为下次等待做准备
                    *woken = false;

                    println!("[executor] 收到唤醒信号，重新 poll future");
                }
            }
        }
    }
}

/// 测试 SimpleExecutor：展示如何手动创建 executor
///
/// 这个例子展示了：
/// 1. 如何手动创建 executor 和 waker
/// 2. Waker 如何通知 executor 重新 poll future
/// 3. 阻塞式 executor 的局限性
pub fn test_simple_executor() {
    println!("\n=== SimpleExecutor 示例：手动创建 Executor ===");
    
    println!("\n场景：使用 SimpleExecutor 运行 AsyncTimerFuture");
    println!("1. 创建 SimpleExecutor");
    println!("2. 创建 AsyncTimerFuture（在后台线程等待 1 秒）");
    println!("3. 使用 block_on 运行 future");
    println!("4. Executor 在 Pending 时阻塞等待");
    println!("5. 后台线程完成后唤醒 executor");
    println!("6. Executor 重新 poll，返回 Ready\n");
    
    let executor = SimpleExecutor::new();
    let future = AsyncTimerFuture::new(Duration::from_secs(1));
    
    let start = std::time::Instant::now();
    let result = executor.block_on(future);
    let elapsed = start.elapsed();
    
    println!("\n结果: {}", result);
    println!("总耗时: {:?} (包含等待时间)", elapsed);
    
    println!("\n关键点：");
    println!("- SimpleExecutor 展示了如何手动创建 executor");
    println!("- 展示了 waker 如何通过 Condvar 唤醒 executor");
    println!("- 注意：这个实现会阻塞线程，无法并发执行多个 future");
    println!("- 实际运行时（如 tokio）使用非阻塞的事件驱动架构");
}


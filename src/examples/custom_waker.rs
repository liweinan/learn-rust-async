use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
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
            // 
            // 重要：每次 poll 都应该更新 waker，即使 waker 已经存在
            // 原因：
            // 1. Future 可能在 executor 的任务之间移动
            // 2. 每次 poll 时的 cx.waker() 可能指向不同的任务
            // 3. 如果不更新，后台线程唤醒的可能是旧任务，而不是当前任务
            // 4. 这会导致 executor 运行错误的任务，或者任务永远不会被唤醒
            //
            // 注意：虽然 waker.clone() 看起来有开销，但实际上：
            // - Waker 的 clone 是轻量级的（通常是引用计数增加）
            // - 相比任务调度错误的风险，这个开销是可以接受的
            // - 大多数情况下，poll 只会被调用几次，不会频繁 clone
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

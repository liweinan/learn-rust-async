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
            // 注意：waker 初始化为 None，此时还没有 Context，无法获取 waker
            // waker 会在第一次 poll() 时被注入（见 poll() 方法的注释）
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
            // 注意：此时 waker 应该已经被 poll() 方法注入（见 poll() 方法的注释）
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
            // # Waker 注入时机和流程
            //
            // ## 1. Waker 注入时机
            // - **不是在 new() 时注入**：new() 时 waker 初始化为 None，此时还没有 Context
            // - **在第一次 poll() 时注入**：当 executor（如 tokio）第一次调用 poll() 时注入
            // - **每次 poll() 都会更新**：确保唤醒的是当前任务（见下面的说明）
            //
            // ## 2. 谁负责注入
            // - **Future 的 poll() 方法负责注入**：这里执行 `state.waker = Some(cx.waker().clone())`
            // - **tokio 负责提供 waker**：通过 Context 传入 `cx.waker()`
            // - **tokio 负责调用 poll()**：executor 调用 poll() 时传入 Context
            //
            // ## 3. 完整流程
            // ```
            // 1. new() 创建 future
            //    └─> shared_state.waker = None
            //    └─> 启动后台线程（等待中）
            //
            // 2. tokio executor 第一次调用 poll()
            //    └─> 传入 Context（包含 waker）
            //    └─> poll() 中：state.waker = Some(cx.waker().clone())  ← 注入时机
            //    └─> 返回 Poll::Pending
            //
            // 3. 后台线程完成等待
            //    └─> state.waker.take() 获取 waker
            //    └─> waker.wake() 通知 executor
            //
            // 4. tokio executor 收到通知，再次调用 poll()
            //    └─> 此时 completed = true
            //    └─> 返回 Poll::Ready
            // ```
            //
            // ## 4. 为什么每次 poll 都要更新 waker
            // - Future 可能在 executor 的任务之间移动
            // - 每次 poll 时的 cx.waker() 可能指向不同的任务
            // - 如果不更新，后台线程唤醒的可能是旧任务，而不是当前任务
            // - 这会导致 executor 运行错误的任务，或者任务永远不会被唤醒
            //
            // ## 5. 性能考虑
            // - Waker 的 clone 是轻量级的（通常是引用计数增加）
            // - 相比任务调度错误的风险，这个开销是可以接受的
            // - 大多数情况下，poll 只会被调用几次，不会频繁 clone
            state.waker = Some(cx.waker().clone());
            println!("[poll] Future 未就绪，保存 waker 并返回 Pending");
            Poll::Pending
        }
    }
}

/// 测试自定义 Waker 的实际用途（使用 tokio 运行时，通过 await）
pub async fn test_custom_waker() {
    println!("\n=== 自定义 Waker 示例：展示实际用途（使用 await） ===");
    
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

/// 测试自定义 Waker 的实际用途（使用 tokio 的 block_on）
///
/// 这个例子展示了如何显式使用 `tokio::runtime::Runtime::block_on()` 来运行 future。
/// 与 `test_custom_waker()` 的区别是：
/// - `test_custom_waker()` 使用 `await`，需要在 async 上下文中运行
/// - `test_custom_waker_with_block_on()` 使用 `block_on`，可以在非 async 函数中运行
///
/// 注意：这个函数需要在独立线程中运行，因为不能在运行时内部再创建运行时。
/// 如果已经在 tokio 运行时中，可以使用 `Handle::current().block_on()`。
pub fn test_custom_waker_with_block_on() {
    println!("\n=== 自定义 Waker 示例：使用 tokio::runtime::Runtime::block_on() ===");
    
    println!("\n场景：显式使用 tokio 的 Runtime::block_on() 运行 future");
    println!("1. 创建 tokio Runtime");
    println!("2. 使用 block_on 运行 AsyncTimerFuture");
    println!("3. block_on 会阻塞当前线程，直到 future 完成");
    println!("4. 这与 SimpleExecutor 的 block_on 类似，但使用的是 tokio 的运行时\n");
    
    // 创建 tokio 运行时
    // 注意：如果在 tokio 运行时内部调用此函数，会报错
    // 此时应该使用 Handle::current().block_on() 或 spawn_blocking
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let future = AsyncTimerFuture::new(Duration::from_secs(1));
    
    let start = std::time::Instant::now();
    // 使用 block_on 运行 future
    let result = rt.block_on(future);
    let elapsed = start.elapsed();
    
    println!("\n结果: {}", result);
    println!("总耗时: {:?} (包含等待时间)", elapsed);
    
    println!("\n关键点：");
    println!("- Runtime::block_on() 会阻塞当前线程，直到 future 完成");
    println!("- 这与 SimpleExecutor::block_on() 的行为类似");
    println!("- 但 tokio 的 Runtime 支持多线程和事件驱动，可以并发执行多个任务");
    println!("- 在实际应用中，通常使用 #[tokio::main] 或 Runtime::new().unwrap().block_on()");
    println!("- 在 async 函数中，使用 await 更常见，不需要显式创建 Runtime");
    println!("\n注意：如果在 tokio 运行时内部，应该使用 Handle::current().block_on()");
}

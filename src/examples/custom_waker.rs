use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

/// 自定义 Future：演示 Waker 的使用
pub struct CustomMyFuture {
    is_ready: bool,
}

impl CustomMyFuture {
    pub fn new() -> Self {
        Self { is_ready: false }
    }
    
    pub fn make_ready(&mut self) {
        self.is_ready = true;
    }
}

impl Future for CustomMyFuture {
    type Output = &'static str;
    
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_ready {
            Poll::Ready("Future is now ready!")
        } else {
            // 注册 waker 以便在任务准备好时唤醒
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

/// 创建自定义 Waker
fn create_custom_waker(callback: Arc<Mutex<bool>>) -> Waker {
    // 从指针和 vtable 创建 RawWaker
    let raw_waker = RawWaker::new(
        Arc::into_raw(callback) as *const (),
        &RawWakerVTable::new(
            clone_callback,
            wake_callback,
            wake_by_ref_callback,
            drop_callback,
        ),
    );
    // 将 RawWaker 转换为 Waker
    unsafe { Waker::from_raw(raw_waker) }
}

// Waker vtable 的回调函数
unsafe fn clone_callback(ptr: *const ()) -> RawWaker {
    let arc = unsafe { Arc::from_raw(ptr as *const Mutex<bool>) };
    let clone = Arc::clone(&arc);
    std::mem::forget(arc);
    RawWaker::new(
        Arc::into_raw(clone) as *const (),
        &RawWakerVTable::new(
            clone_callback,
            wake_callback,
            wake_by_ref_callback,
            drop_callback,
        ),
    )
}

unsafe fn wake_callback(ptr: *const ()) {
    let arc = unsafe { Arc::from_raw(ptr as *const Mutex<bool>) };
    *arc.lock().unwrap() = true;
    std::mem::forget(arc);
}

unsafe fn wake_by_ref_callback(ptr: *const ()) {
    let arc = unsafe { Arc::from_raw(ptr as *const Mutex<bool>) };
    *arc.lock().unwrap() = true;
    std::mem::forget(arc);
}

unsafe fn drop_callback(ptr: *const ()) {
    drop(unsafe { Arc::from_raw(ptr as *const Mutex<bool>) });
}

/// 测试自定义 Waker
pub fn test_custom_waker() {
    println!("\n=== 自定义 Waker 示例 ===");
    
    // 自定义 Waker 的共享状态
    let ready_state = Arc::new(Mutex::new(false));
    let waker = create_custom_waker(ready_state.clone());
    let mut my_future = CustomMyFuture::new();
    let mut cx = Context::from_waker(&waker);
    
    // 第一次 poll future
    match Pin::new(&mut my_future).poll(&mut cx) {
        Poll::Ready(result) => println!("{}", result),
        Poll::Pending => {
            println!("Future is not ready. Waking the task...");
        }
    }
    
    // 检查 waker 是否被调用
    if *ready_state.lock().unwrap() {
        println!("Waker was called! (ready_state = true)");
    }
    
    // 模拟让 future 准备好
    my_future.make_ready();
    
    // 再次 poll future
    match Pin::new(&mut my_future).poll(&mut cx) {
        Poll::Ready(result) => println!("{}", result),
        Poll::Pending => println!("Future is still not ready."),
    }
}

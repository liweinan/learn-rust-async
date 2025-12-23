use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

/// SimpleCoroutine: 编译器生成的等价代码（无 await 的 async 函数）
pub enum SimpleCoroutine {
    Unresumed,
    Returned,
    #[allow(dead_code)]
    Panicked,  // 这个 variant 确实不会被使用，保留 allow(dead_code)
}

impl Future for SimpleCoroutine {
    type Output = i32;
    
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        match this {
            SimpleCoroutine::Unresumed => {
                *this = SimpleCoroutine::Returned;
                Poll::Ready(42)
            }
            SimpleCoroutine::Returned => panic!("cannot poll after completion"),
            SimpleCoroutine::Panicked => panic!("cannot poll after panic"),
        }
    }
}

fn simple() -> impl Future<Output = i32> {
    SimpleCoroutine::Unresumed
}

/// 测试 SimpleCoroutine
pub fn test_simple_coroutine() {
    fn create_waker() -> Waker {
        unsafe fn clone(data: *const ()) -> RawWaker {
            RawWaker::new(data, &VTABLE)
        }
        unsafe fn wake(_data: *const ()) {}
        unsafe fn wake_by_ref(_data: *const ()) {}
        unsafe fn drop(_data: *const ()) {}
        
        const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
        unsafe { Waker::from_raw(RawWaker::new(&(), &VTABLE)) }
    }
    
    let waker = create_waker();
    let mut cx = Context::from_waker(&waker);
    let mut fut = Box::pin(simple());
    
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(result) => println!("SimpleCoroutine result: {}", result),
        Poll::Pending => println!("Still pending..."),
    }
}

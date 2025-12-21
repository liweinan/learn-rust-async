use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// 简单的 Future 实现示例
pub struct MyFuture {}

impl MyFuture {
    pub fn new() -> Self {
        Self {}
    }
}

impl Future for MyFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

/// 测试基本的 Future
pub async fn test_basic_future() {
    let fut = MyFuture::new();
    println!("Awaiting fut...");
    fut.await;
    println!("Awaiting fut... done!");
}

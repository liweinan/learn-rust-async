use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let fut = MyFuture {};
    println!("Awaiting fut...");
    fut.await;
    println!("Awaiting fut... done!");
}

struct MyFuture {}

impl Future for MyFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 👇
        Poll::Ready(())
    }
}

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

    let msg = hello().await; // 像写同步代码一样等待
    println!("{msg}");

}

/// 异步函数 `hello` 在编译期会被**展开成一个匿名结构体**，
/// 而不是面向语言里的 “class”。
///
/// 生成的伪代码大致如下：
/// ```
/// struct HelloFuture {
///     state: u8,          // 当前状态（每个 await 点一个编号）
///     // …局部变量也会变成字段，保证跨 poll 存活
/// }
///
/// impl Future for HelloFuture {
///     type Output = &'static str;
///     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>)
///         -> Poll<Self::Output>
///     {
///         match self.state { /* 状态机主体 */ }
///     }
/// }
/// ```
///
/// 调用 `hello()` 只是**构造并返回**这个状态机实例；
/// 真正驱动它跑完的是运行时反复执行的 `Future::poll`。
async fn hello() -> &'static str {
    "hello, tokio!"
}

struct MyFuture {}

impl Future for MyFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 👇
        Poll::Ready(())
    }
}

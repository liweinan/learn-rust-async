use tokio::time::sleep;
use std::time::Duration;
use tokio::join;

/// 基本的 async 函数示例
pub async fn greet() {
    println!("Hello!");
    sleep(Duration::from_millis(500)).await;
    println!("Goodbye!");
}

/// 简单的 async 函数示例
/// 
/// 异步函数 `hello` 在编译期会被**展开成一个匿名结构体**，
/// 而不是面向语言里的 "class"。
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
pub async fn hello() -> &'static str {
    "hello, tokio!"
}

/// 测试顺序执行：直接 await
pub async fn test_sequential() {
    greet().await;
    greet().await;
}

/// 测试并发执行（即使使用 current_thread，spawn 创建的任务也会并发执行）
pub async fn test_concurrent() {
    let one = tokio::spawn(greet());
    let two = tokio::spawn(greet());
    let (_, _) = join!(one, two);
}

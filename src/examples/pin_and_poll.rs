use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// HelloFuture：使用 get_mut() 修改字段
/// 
/// 这个例子展示了在 poll 方法内部使用 get_mut() 的方式
/// 适用于 Future 实现了 Unpin 的情况
pub struct HelloFuture {
    // 可修改的字段：用于演示 get_mut() 如何修改字段
    count: u32,
}

impl HelloFuture {
    pub fn new() -> Self {
        Self {
            count: 0,
        }
    }
}

impl Future for HelloFuture {
    type Output = &'static str;

    /// 使用 get_mut() 修改字段
    /// 
    /// 适用场景：Future 实现了 Unpin
    /// 优点：代码最简洁，直接获取 &mut Self，可以修改字段
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 使用 get_mut() 获取 &mut Self
        // 因为 HelloFuture 实现了 Unpin，可以直接使用 get_mut()
        let this = self.get_mut();
        
        // 修改字段：演示 get_mut() 可以修改字段
        this.count += 1;
        
        // 当 count 达到 2 时返回 Ready
        if this.count >= 2 {
            Poll::Ready("Hello")
        } else {
            Poll::Pending
        }
    }
}

/// 演示在 poll 方法内部访问和修改 self 的不同方式
pub fn test_pin_and_poll_unpin() {
    // 使用 Rust 标准库提供的 noop waker（Rust 1.85.0+）
    // 这是一个不执行任何操作的 waker，非常适合测试和演示
    // 不需要手动创建 RawWaker 和 RawWakerVTable，也不需要 unsafe
    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);
    
    // 测试 HelloFuture：使用 get_mut()
    println!("\n=== 测试 HelloFuture：使用 get_mut() ===");
    let mut future = HelloFuture::new();
    
    // 注意：poll 方法的签名是 fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>)
    // 这里有两个参数：
    // 1. self: Pin<&mut Self> - 这是方法的接收者（receiver）
    // 2. cx: &mut Context<'_> - 这是显式传入的参数
    // 
    // 关于 receiver 的说明：
    // - Rust 支持标准的 receiver：self（所有权）、&self（不可变引用）、&mut self（可变引用）
    // - self: Pin<&mut Self> 是"arbitrary self types"特性（Rust 1.33+），允许自定义 receiver 类型
    // - 这是 Rust 的语言特性，允许使用点号调用语法，如 pinned.as_mut().poll(&mut cx)
    // - 如果没有这个特性，就需要写成 Future::poll(pinned.as_mut(), &mut cx) 这样的形式
    //
    // 当我们调用 pinned.as_mut().poll(&mut cx) 时：
    // - pinned.as_mut() 返回 Pin<&mut HelloFuture>，作为 self 传入
    // - &mut cx 作为第二个参数传入
    //
    // as_mut() 并没有 deref，它返回的还是 Pin<&mut T>，只是获取了一个可变的 Pin
    // 这是为了满足 poll 方法对 self 类型的要求
    let mut pinned = Pin::new(&mut future);
    
    // 多次 poll 直到完成
    loop {
        // pinned.as_mut() 返回 Pin<&mut HelloFuture>，作为 poll 方法的 self 参数
        // &mut cx 作为 poll 方法的第二个参数（cx 参数）
        match pinned.as_mut().poll(&mut cx) {
            Poll::Ready(value) => {
                println!("结果: {}", value);
                break;
            }
            Poll::Pending => {
                println!("未完成，继续 poll...");
            }
        }
    }
    
    println!("\n=== 总结 ===");
    println!("在 poll 方法内部访问和修改 self 的方式：");
    println!("get_mut() - 最简单，要求 Future 实现 Unpin");
}

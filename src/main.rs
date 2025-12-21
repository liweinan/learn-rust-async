mod examples;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // 示例 1: 基本的 Future 实现
    examples::basic_future::test_basic_future().await;
    
    // 示例 2: 简单的 async 函数
    let msg = examples::greet::hello().await;
    println!("{msg}");
    
    // 示例 3: 顺序执行
    examples::greet::test_sequential().await;
    
    // 示例 4: 并发执行（即使使用 current_thread，spawn 创建的任务也会并发执行）
    examples::greet::test_concurrent().await;
    
    // 示例 5: SimpleCoroutine（编译器生成的等价代码）
    examples::simple_coroutine::test_simple_coroutine();
    
    // 示例 6: 自定义 Waker 示例
    examples::custom_waker::test_custom_waker();
}

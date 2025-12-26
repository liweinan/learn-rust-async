mod examples;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // // 示例 1: 基本的 Future 实现
    // examples::basic_future::test_basic_future().await;
    //
    // // 示例 2: 简单的 async 函数
    // let msg = examples::greet::hello().await;
    // println!("{msg}");
    //
    // // 示例 3: 顺序执行
    // examples::greet::test_sequential().await;
    //
    // // 示例 4: 并发执行（即使使用 current_thread，spawn 创建的任务也会并发执行）
    // examples::greet::test_concurrent().await;
    //
    // // 示例 5: SimpleCoroutine（编译器生成的等价代码）
    // examples::simple_coroutine::test_simple_coroutine();
    
    // 示例 6: 自定义 Waker 示例（使用 await）
    examples::custom_waker::test_custom_waker().await;
    
    // // 示例 6b: 自定义 Waker 示例（使用 tokio 的 block_on）
    // // 注意：这个函数需要在独立线程中运行，因为不能在运行时内部再创建运行时
    // let handle = std::thread::spawn(|| {
    //     examples::custom_waker::test_custom_waker_with_block_on();
    // });
    // handle.join().unwrap();
    //
    // // 示例 7: SimpleExecutor 示例（手动创建 executor）
    // // 注意：SimpleExecutor 是阻塞的，使用 std::thread 在独立线程中运行
    // // 避免阻塞 tokio 运行时，也避免 spawn_blocking 可能带来的线程问题
    // let handle = std::thread::spawn(|| {
    //     examples::simple_executor::test_simple_executor();
    // });
    // handle.join().unwrap();
}

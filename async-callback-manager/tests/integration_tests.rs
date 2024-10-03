#[tokio::test]
async fn test_create() {
    async_callback_manager::AsyncCallbackManager::<String>::new(50);
}

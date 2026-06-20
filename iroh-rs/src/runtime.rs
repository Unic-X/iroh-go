use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub static GLOBAL_RUNTIME: Lazy<Arc<Runtime>> = Lazy::new(|| {
    Arc::new(Runtime::new().expect("failed to create runtime"))
});

pub fn iroh_executor<F: std::future::Future>(future: F) -> F::Output {
    GLOBAL_RUNTIME.block_on(future)
}

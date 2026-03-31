#[cfg(test)]
use std::future::Future;
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[cfg(test)]
pub(crate) fn with_env_lock<F: FnOnce()>(f: F) {
    let lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    f();
}

#[cfg(test)]
pub(crate) async fn with_env_lock_async<F, Fut>(f: F) -> Fut::Output
where
    F: FnOnce() -> Fut,
    Fut: Future,
{
    let lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    f().await
}

use tokio::sync::{RwLockReadGuard, RwLockWriteGuard};

#[async_trait]
pub trait RWLockable<T> {
    async fn read(&self) -> RwLockReadGuard<'_, T>;
    async fn write(&self) -> RwLockWriteGuard<'_, T>;
}

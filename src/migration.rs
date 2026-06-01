use crate::error::SurrealStoreError;
use crate::storage::{SqliteWorkspaceStore, WorkspaceStore};
use crate::surreal::SurrealWorkspaceStore;
use std::path::Path;

pub async fn migrate_sqlite_to_surreal(
    sqlite_path: impl AsRef<Path>,
    surreal_store: &SurrealWorkspaceStore,
) -> Result<(), SurrealStoreError> {
    let sqlite_store = SqliteWorkspaceStore::new(sqlite_path.as_ref().to_path_buf())?;
    if let Some(snapshot) = sqlite_store.load_snapshot()? {
        surreal_store.save_snapshot(&snapshot).await?;
    }
    Ok(())
}

pub async fn migrate_sqlite_to_surreal_path(
    sqlite_path: impl AsRef<Path>,
    surreal_path: impl AsRef<Path>,
) -> Result<(), SurrealStoreError> {
    let surreal_store = SurrealWorkspaceStore::new_local(surreal_path).await?;
    migrate_sqlite_to_surreal(sqlite_path, &surreal_store).await
}

use async_trait::async_trait;
use bytes::Bytes;
use parking_lot::RwLock;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::{fs, io::AsyncWriteExt};

use crate::infrastructure::config::StorageConfig;

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn put(&self, key: &str, data: Bytes, content_type: &str) -> anyhow::Result<()>;
    async fn delete(&self, key: &str) -> anyhow::Result<()>;
    async fn presigned_url(&self, key: &str) -> anyhow::Result<Option<String>>;
}

pub fn build_storage(config: &StorageConfig) -> anyhow::Result<Arc<dyn StorageBackend>> {
    match config.provider.as_str() {
        "local" => Ok(Arc::new(LocalStorage::new(config.local_path.clone())?)),
        "memory" => Ok(Arc::new(MemoryStorage::default())),
        other => anyhow::bail!("unsupported storage provider: {other}"),
    }
}

pub fn local_storage_root(path: Option<&str>) -> PathBuf {
    PathBuf::from(path.unwrap_or("./uploads"))
}

struct LocalStorage {
    root: PathBuf,
}

impl LocalStorage {
    fn new(path: Option<String>) -> anyhow::Result<Self> {
        let root = local_storage_root(path.as_deref());
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn put(&self, key: &str, data: Bytes, _content_type: &str) -> anyhow::Result<()> {
        let mut path = self.root.clone();
        path.push(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = fs::File::create(path).await?;
        file.write_all(&data).await?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        let mut path = self.root.clone();
        path.push(key);
        if fs::try_exists(&path).await? {
            fs::remove_file(path).await?;
        }
        Ok(())
    }

    async fn presigned_url(&self, key: &str) -> anyhow::Result<Option<String>> {
        let mut path = PathBuf::from("/receipts");
        path.push(key);
        Ok(Some(path.to_string_lossy().to_string()))
    }
}

#[derive(Default)]
struct MemoryStorage {
    objects: RwLock<HashMap<String, Bytes>>,
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn put(&self, key: &str, data: Bytes, _content_type: &str) -> anyhow::Result<()> {
        self.objects.write().insert(key.to_string(), data);
        Ok(())
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.objects.write().remove(key);
        Ok(())
    }

    async fn presigned_url(&self, key: &str) -> anyhow::Result<Option<String>> {
        Ok(Some(format!("memory://{key}")))
    }
}

use async_trait::async_trait;
use bytes::Bytes;
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    sync::Arc,
};
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

    fn validate_key(&self, key: &str) -> anyhow::Result<PathBuf> {
        if key.trim().is_empty() {
            anyhow::bail!("invalid storage key: {key}");
        }

        let path = Path::new(key);
        if path.is_absolute() {
            anyhow::bail!("invalid storage key: {key}");
        }

        let mut sanitized = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Normal(part) => sanitized.push(part),
                Component::CurDir => continue,
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    anyhow::bail!("invalid storage key: {key}")
                }
            }
        }

        if sanitized.as_os_str().is_empty() {
            anyhow::bail!("invalid storage key: {key}");
        }

        let resolved = self.root.join(&sanitized);
        if !resolved.starts_with(&self.root) {
            anyhow::bail!("invalid storage key: {key}");
        }

        Ok(sanitized)
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn put(&self, key: &str, data: Bytes, _content_type: &str) -> anyhow::Result<()> {
        let sanitized = self.validate_key(key)?;
        let path = self.root.join(sanitized);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = fs::File::create(path).await?;
        file.write_all(&data).await?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        let sanitized = self.validate_key(key)?;
        let path = self.root.join(sanitized);
        if fs::try_exists(&path).await? {
            fs::remove_file(path).await?;
        }
        Ok(())
    }

    async fn presigned_url(&self, key: &str) -> anyhow::Result<Option<String>> {
        let sanitized = self.validate_key(key)?;
        let mut path = PathBuf::from("/receipts");
        path.push(sanitized);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_key_accepts_relative_paths() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let storage = LocalStorage {
            root: tmp_dir.path().to_path_buf(),
        };

        let sanitized = storage.validate_key("receipts/user1.png").unwrap();
        assert_eq!(sanitized, PathBuf::from("receipts/user1.png"));
    }

    #[test]
    fn validate_key_rejects_parent_directory_components() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let storage = LocalStorage {
            root: tmp_dir.path().to_path_buf(),
        };

        assert!(storage.validate_key("../secrets.txt").is_err());
        assert!(storage.validate_key("receipts/../../secrets.txt").is_err());
    }

    #[test]
    fn validate_key_rejects_absolute_paths() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let storage = LocalStorage {
            root: tmp_dir.path().to_path_buf(),
        };

        assert!(storage.validate_key("/etc/passwd").is_err());
    }
}

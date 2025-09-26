use anyhow::Result;
use camino::Utf8Path;
use tokio::fs as async_fs;

pub async fn sync_artifact(path: &Utf8Path, contents: Option<&[u8]>) -> Result<()> {
    if let Some(bytes) = contents {
        if let Some(parent) = path.parent() {
            async_fs::create_dir_all(parent.as_std_path()).await?;
        }
        async_fs::write(path.as_std_path(), bytes).await?;
    } else if async_fs::metadata(path.as_std_path()).await.is_ok() {
        let _ = async_fs::remove_file(path.as_std_path()).await;
    }
    Ok(())
}

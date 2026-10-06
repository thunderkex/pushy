use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    pub name: &'static str,
    pub description: &'static str,
    pub default_enabled: bool,
    pub max_size_bytes: Option<u64>,
    pub retention: &'static str,
    pub supports_expiry: bool,
}

#[derive(Debug, Clone)]
pub struct UploadContext {
    pub file_path: PathBuf,
    pub file_name: String,
    pub file_size: u64,
    pub expire: Option<String>,
}

pub trait Provider: Send + Sync {
    fn metadata(&self) -> ProviderMetadata;

    fn check_availability(&self, is_explicit: bool, config: &crate::config::Config) -> Result<()>;

    fn upload<'a>(
        &'a self,
        client: &'a reqwest::Client,
        ctx: &'a UploadContext,
        config: &'a Config,
        progress_bar: Option<indicatif::ProgressBar>,
        base_url_override: Option<&'a str>,
    ) -> BoxFuture<'a, Result<String>>;
}

use crate::config::Config;

pub async fn create_stream_body(
    path: &Path,
    progress_bar: Option<indicatif::ProgressBar>,
) -> Result<reqwest::Body> {
    let file = File::open(path)
        .await
        .with_context(|| format!("Failed to open file: {}", path.display()))?;

    let stream = ReaderStream::new(file);
    if let Some(pb) = progress_bar {
        let watched = stream.inspect_ok(move |chunk| {
            pb.inc(chunk.len() as u64);
        });
        Ok(reqwest::Body::wrap_stream(watched))
    } else {
        Ok(reqwest::Body::wrap_stream(stream))
    }
}

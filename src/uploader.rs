use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::Duration;
use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;

use crate::config::Config;
use crate::provider::{Provider, UploadContext};
use crate::providers;

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(Debug, Clone)]
pub enum UploadResult {
    Single(String),
    All(Vec<(String, String)>),
}

pub struct Uploader {
    client: Client,
    config: Config,
    quiet: bool,
}

impl Uploader {
    pub fn new(config: Config, quiet: bool) -> Self {
        let ua = ua_generator::ua::spoof_ua();
        Self::with_user_agent(config, quiet, ua)
    }

    pub fn with_user_agent(config: Config, quiet: bool, user_agent: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .user_agent(user_agent)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            config,
            quiet,
        }
    }

    pub async fn upload_file(
        &self,
        file_path: &Path,
        override_name: Option<&str>,
        override_provider: Option<&str>,
        expire: Option<&str>,
        force_all: bool,
    ) -> Result<UploadResult> {
        let file_name = if let Some(n) = override_name {
            n.to_string()
        } else {
            file_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "file.bin".to_string())
        };

        let metadata = tokio::fs::metadata(file_path)
            .await
            .with_context(|| format!("File not found or inaccessible: {}", file_path.display()))?;

        if !metadata.is_file() {
            bail!("Not a regular file: {}", file_path.display());
        }

        let file_size = metadata.len();
        let ctx = UploadContext {
            file_path: file_path.to_path_buf(),
            file_name,
            file_size,
            expire: expire.map(|s| s.to_string()),
        };

        let should_upload_all = force_all
            || override_provider == Some("all")
            || (override_provider.is_none() && (self.config.all || self.config.provider == "all"));

        if should_upload_all {
            let results = self.upload_to_all(&ctx).await?;
            Ok(UploadResult::All(results))
        } else if let Some(prov_name) = override_provider {
            let link = self.upload_single_provider(prov_name, &ctx).await?;
            Ok(UploadResult::Single(link))
        } else {
            let link = self.upload_with_fallback(&ctx).await?;
            Ok(UploadResult::Single(link))
        }
    }

    pub async fn upload_to_all(&self, ctx: &UploadContext) -> Result<Vec<(String, String)>> {
        let mut candidates = Vec::new();

        for p in providers::all_providers() {
            let meta = p.metadata();

            // Skip providers where file exceeds limit
            if let Some(max) = meta.max_size_bytes {
                if ctx.file_size > max {
                    continue;
                }
            }

            // Skip disabled providers unless enabled in config
            if p.check_availability(false, &self.config).is_err() {
                continue;
            }

            // If expiry is not requested, skip temporary-only litterbox
            if meta.name == "litterbox" && ctx.expire.is_none() {
                continue;
            }

            candidates.push(p);
        }

        if candidates.is_empty() {
            bail!("No available providers found for upload.");
        }

        let mut futures = Vec::new();
        for p in &candidates {
            let p_ref = p.as_ref();
            let name = p.metadata().name;
            futures.push(async move {
                let res = self.attempt_upload_with_retry(p_ref, ctx, None).await;
                (name.to_string(), res)
            });
        }

        let results = futures_util::future::join_all(futures).await;
        let mut successful = Vec::new();
        let mut failed = Vec::new();

        for (name, res) in results {
            match res {
                Ok(url) => successful.push((name, url)),
                Err(e) => failed.push(format!("{}: {}", name, e)),
            }
        }

        if successful.is_empty() {
            bail!("All providers failed:\n  {}", failed.join("\n  "));
        }

        if !failed.is_empty() && !self.quiet {
            eprintln!("pushy (all): some providers failed:\n  {}", failed.join("\n  "));
        }

        Ok(successful)
    }

    async fn upload_single_provider(
        &self,
        prov_name: &str,
        ctx: &UploadContext,
    ) -> Result<String> {
        let provider = providers::get_provider(prov_name)
            .with_context(|| format!("Unknown provider '{}'. Run 'pushy providers' for a list.", prov_name))?;

        let meta = provider.metadata();
        provider.check_availability(true, &self.config)?;

        if let Some(max) = meta.max_size_bytes {
            if ctx.file_size > max {
                bail!(
                    "File '{}' (size: {}) exceeds {}'s limit of {}",
                    ctx.file_name,
                    format_bytes(ctx.file_size),
                    meta.name,
                    format_bytes(max)
                );
            }
        }

        self.attempt_upload_with_retry(provider.as_ref(), ctx, None)
            .await
    }

    async fn upload_with_fallback(&self, ctx: &UploadContext) -> Result<String> {
        let mut candidates = Vec::new();

        if ctx.expire.is_some() {
            for p in providers::all_providers() {
                if p.metadata().supports_expiry {
                    let name = p.metadata().name.to_string();
                    if !candidates.contains(&name) {
                        candidates.push(name);
                    }
                }
            }
        }

        if !candidates.contains(&self.config.provider) {
            candidates.push(self.config.provider.clone());
        }

        for fb in &self.config.fallback {
            if !candidates.contains(fb) {
                candidates.push(fb.clone());
            }
        }

        let mut failed_providers = Vec::new();

        for name in candidates {
            let provider = match providers::get_provider(&name) {
                Some(p) => p,
                None => continue,
            };

            let meta = provider.metadata();

            if let Some(max) = meta.max_size_bytes {
                if ctx.file_size > max {
                    if !self.quiet {
                        eprintln!(
                            "pushy: '{}' exceeds {} limit ({} > {}), skipping...",
                            ctx.file_name,
                            meta.name,
                            format_bytes(ctx.file_size),
                            format_bytes(max)
                        );
                    }
                    continue;
                }
            }

            if provider.check_availability(false, &self.config).is_err() {
                continue;
            }

            match self.attempt_upload_with_retry(provider.as_ref(), ctx, None).await {
                Ok(url) => {
                    if !failed_providers.is_empty() && !self.quiet {
                        eprintln!(
                            "pushy: {} failed, tried {}: ok",
                            failed_providers.join(", "),
                            meta.name
                        );
                    }
                    return Ok(url);
                }
                Err(err) => {
                    failed_providers.push(format!("{}: {}", meta.name, err));
                }
            }
        }

        bail!("All providers failed:\n  {}", failed_providers.join("\n  "))
    }

    pub async fn attempt_upload_with_retry(
        &self,
        provider: &dyn Provider,
        ctx: &UploadContext,
        base_url_override: Option<&str>,
    ) -> Result<String> {
        let meta = provider.metadata();
        let max_retries = 2;
        let mut last_err = anyhow::anyhow!("Unknown error");

        for attempt in 0..=max_retries {
            let pb = if !self.quiet && std::io::stderr().is_terminal() {
                let p = ProgressBar::new(ctx.file_size);
                p.set_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
                        .unwrap_or_else(|_| ProgressStyle::default_bar())
                        .progress_chars("#>-"),
                );
                p.set_message(format!("Uploading to {}", meta.name));
                Some(p)
            } else {
                None
            };

            let result = provider
                .upload(&self.client, ctx, &self.config, pb.clone(), base_url_override)
                .await;

            if let Some(p) = pb {
                p.finish_and_clear();
            }

            match result {
                Ok(url) => return Ok(url),
                Err(e) => {
                    let err_str = e.to_string();
                    let is_4xx = err_str.contains("HTTP 4");
                    last_err = e;

                    if is_4xx || attempt == max_retries {
                        break;
                    }

                    let backoff = Duration::from_millis(500 * (1 << attempt));
                    tokio::time::sleep(backoff).await;
                }
            }
        }

        Err(last_err)
    }

    pub async fn buffer_stdin_to_temp() -> Result<(tempfile::NamedTempFile, PathBuf)> {
        use tokio::io::AsyncWriteExt;

        let temp_file = tempfile::Builder::new()
            .prefix("pushy_stdin_")
            .tempfile()
            .context("Failed to create temporary file for stdin")?;

        let temp_path = temp_file.path().to_path_buf();
        let mut async_file = tokio::fs::File::create(&temp_path).await?;
        let mut stdin = tokio::io::stdin();

        tokio::io::copy(&mut stdin, &mut async_file)
            .await
            .context("Failed to read from stdin")?;

        async_file.flush().await?;
        Ok((temp_file, temp_path))
    }
}

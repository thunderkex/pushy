use anyhow::{bail, Context, Result};
use reqwest::header::{CONTENT_LENGTH};
use serde::Deserialize;

use crate::config::Config;
use crate::provider::{create_stream_body, BoxFuture, Provider, ProviderMetadata, UploadContext};

#[derive(Deserialize)]
struct PixeldrainResponse {
    id: Option<String>,
    message: Option<String>,
}

pub struct PixeldrainProvider;

fn api_key(config: &Config) -> Option<&str> {
    config
        .pixeldrain_key
        .as_deref()
        .or_else(|| config.providers.get("pixeldrain").and_then(|p| p.key.as_deref()))
}

impl Provider for PixeldrainProvider {
    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "pixeldrain",
            description: "Pixeldrain file sharing (needs API key; 20 GB limit)",
            default_enabled: false,
            max_size_bytes: Some(20 * 1024 * 1024 * 1024),
            retention: "Kept while downloaded (~120 days idle)",
            supports_expiry: false,
        }
    }

    fn check_availability(&self, is_explicit: bool, config: &Config) -> Result<()> {
        let has_key = api_key(config).is_some();
        if !has_key {
            if is_explicit {
                bail!("pixeldrain needs an API key: set pixeldrain_key = \"...\" in config (pixeldrain.com/user/api_keys)");
            } else {
                bail!("pixeldrain is disabled by default (no API key configured)");
            }
        }
        Ok(())
    }

    fn upload<'a>(
        &'a self,
        client: &'a reqwest::Client,
        ctx: &'a UploadContext,
        config: &'a Config,
        progress_bar: Option<indicatif::ProgressBar>,
        base_url_override: Option<&'a str>,
    ) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            let body = create_stream_body(&ctx.file_path, progress_bar).await?;

            let base = base_url_override.unwrap_or("https://pixeldrain.com");
            let mut url = reqwest::Url::parse(base).context("Invalid pixeldrain base URL")?;
            url.path_segments_mut()
                .map_err(|_| anyhow::anyhow!("Invalid pixeldrain base URL"))?
                .pop_if_empty()
                .extend(["api", "file", ctx.file_name.as_str()]);

            let mut req = client
                .put(url)
                .header(CONTENT_LENGTH, ctx.file_size)
                .body(body);

            if let Some(key) = api_key(config) {
                req = req.basic_auth("", Some(key));
            }

            let resp = req.send().await?;
            let status = resp.status();
            let body_text = resp.text().await?;

            if !status.is_success() {
                // JSON errors look like {"success":false,"value":"...","message":"..."}
                let detail = serde_json::from_str::<PixeldrainResponse>(&body_text)
                    .ok()
                    .and_then(|r| r.message)
                    .unwrap_or_else(|| crate::provider::sanitize_error_body(&body_text));
                if detail.is_empty() {
                    bail!("pixeldrain error (HTTP {})", status);
                } else {
                    bail!("pixeldrain error (HTTP {}): {}", status, detail);
                }
            }

            let parsed: PixeldrainResponse = serde_json::from_str(&body_text)
                .with_context(|| format!("Failed to parse pixeldrain JSON: {}", body_text))?;
            let id = parsed.id.context("Missing id in pixeldrain response")?;

            Ok(format!("https://pixeldrain.com/u/{}", id))
        })
    }
}

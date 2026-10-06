use anyhow::{bail, Context, Result};
use reqwest::multipart::{Form, Part};
use serde::Deserialize;

use crate::config::Config;
use crate::provider::{create_stream_body, BoxFuture, Provider, ProviderMetadata, UploadContext};

#[derive(Deserialize)]
struct TmpfilesResponse {
    status: String,
    data: Option<TmpfilesData>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct TmpfilesData {
    url: Option<String>,
}

pub struct TmpfilesProvider;

impl Provider for TmpfilesProvider {
    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "tmpfiles",
            description: "Temporary direct file sharing (~1h retention, 100 MB limit)",
            default_enabled: true,
            max_size_bytes: Some(100 * 1024 * 1024),
            retention: "~1 hour",
            supports_expiry: false,
        }
    }

    fn check_availability(&self, _is_explicit: bool, _config: &Config) -> Result<()> {
        Ok(())
    }

    fn upload<'a>(
        &'a self,
        client: &'a reqwest::Client,
        ctx: &'a UploadContext,
        _config: &'a Config,
        progress_bar: Option<indicatif::ProgressBar>,
        base_url_override: Option<&'a str>,
    ) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            let body = create_stream_body(&ctx.file_path, progress_bar).await?;

            let safe_name = if ctx.file_name.len() < 5 {
                format!("upload_{}", ctx.file_name)
            } else {
                ctx.file_name.clone()
            };

            let part = Part::stream_with_length(body, ctx.file_size).file_name(safe_name);
            let form = Form::new().part("file", part);

            let endpoint = base_url_override.unwrap_or("https://tmpfiles.org/api/v1/upload");
            let resp = client.post(endpoint).multipart(form).send().await?;

            let status = resp.status();
            let body_text = resp.text().await?;

            if !status.is_success() {
                let sanitized = crate::provider::sanitize_error_body(&body_text);
                if sanitized.is_empty() {
                    bail!("tmpfiles error (HTTP {})", status);
                } else {
                    bail!("tmpfiles error (HTTP {}): {}", status, sanitized);
                }
            }

            let tmp_resp: TmpfilesResponse = serde_json::from_str(&body_text)
                .with_context(|| format!("Failed to parse tmpfiles JSON: {}", body_text))?;

            if tmp_resp.status != "success" {
                let msg = tmp_resp.message.unwrap_or_else(|| "unknown error".into());
                bail!("tmpfiles error: {}", msg);
            }

            let data = tmp_resp.data.context("Missing tmpfiles data")?;
            let raw_url = data.url.context("Missing url in tmpfiles response")?;

            let dl_url = if raw_url.contains("/dl/") {
                raw_url
            } else if let Some(idx) = raw_url.find("tmpfiles.org/") {
                let (prefix, suffix) = raw_url.split_at(idx + "tmpfiles.org/".len());
                format!("{}dl/{}", prefix, suffix)
            } else {
                raw_url
            };

            Ok(dl_url)
        })
    }
}

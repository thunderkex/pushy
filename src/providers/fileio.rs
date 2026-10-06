use anyhow::{bail, Context, Result};
use reqwest::header::AUTHORIZATION;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;

use crate::config::Config;
use crate::provider::{create_stream_body, BoxFuture, Provider, ProviderMetadata, UploadContext};

#[derive(Deserialize)]
struct FileioResponse {
    success: Option<bool>,
    link: Option<String>,
    message: Option<String>,
}

pub struct FileioProvider;

impl Provider for FileioProvider {
    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "fileio",
            description: "Ephemeral file sharing (Requires API key; anonymous disabled)",
            default_enabled: false,
            max_size_bytes: Some(2 * 1024 * 1024 * 1024),
            retention: "Single download / 14 days",
            supports_expiry: true,
        }
    }

    fn check_availability(&self, is_explicit: bool, config: &Config) -> Result<()> {
        let has_key = config
            .providers
            .get("fileio")
            .and_then(|p| p.key.as_ref())
            .is_some();
        if !has_key {
            if is_explicit {
                bail!("file.io anonymous uploads are currently broken upstream; an API key is required in config.");
            } else {
                bail!("fileio is disabled by default (no API key configured)");
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
            let part = Part::stream_with_length(body, ctx.file_size)
                .file_name(ctx.file_name.clone());

            let mut form = Form::new().part("file", part);
            if let Some(exp) = &ctx.expire {
                form = form.text("expires", exp.clone());
            }

            let key = config
                .providers
                .get("fileio")
                .and_then(|p| p.key.as_deref());

            let endpoint = base_url_override.unwrap_or("https://file.io");
            let mut req = client.post(endpoint).multipart(form);

            if let Some(k) = key {
                req = req.header(AUTHORIZATION, format!("Bearer {}", k));
            }

            let resp = req.send().await?;
            let status = resp.status();
            let body_text = resp.text().await?;

            if !status.is_success() {
                bail!("file.io error (HTTP {}): {}", status, body_text);
            }

            let fileio_resp: FileioResponse = serde_json::from_str(&body_text)
                .with_context(|| format!("Failed to parse file.io JSON: {}", body_text))?;

            if let Some(success) = fileio_resp.success {
                if !success {
                    let msg = fileio_resp.message.unwrap_or_else(|| "Upload failed".into());
                    bail!("file.io error: {}", msg);
                }
            }

            let link = fileio_resp.link.context("Missing link in file.io response")?;
            Ok(link)
        })
    }
}

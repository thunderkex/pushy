use anyhow::{bail, Result};
use reqwest::multipart::{Form, Part};

use crate::config::Config;
use crate::provider::{create_stream_body, BoxFuture, Provider, ProviderMetadata, UploadContext};

pub struct TempshProvider;

impl Provider for TempshProvider {
    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "tempsh",
            description: "Temporary file hosting (3 days, 4 GB limit)",
            default_enabled: true,
            max_size_bytes: Some(4 * 1024 * 1024 * 1024),
            retention: "3 days",
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
            let part = Part::stream_with_length(body, ctx.file_size)
                .file_name(ctx.file_name.clone());

            let form = Form::new().part("file", part);

            let endpoint = base_url_override.unwrap_or("https://temp.sh/upload");
            let resp = client.post(endpoint).multipart(form).send().await?;

            let status = resp.status();
            let body_text = resp.text().await?;

            if !status.is_success() {
                let sanitized = crate::provider::sanitize_error_body(&body_text);
                if sanitized.is_empty() {
                    bail!("temp.sh error (HTTP {})", status);
                } else {
                    bail!("temp.sh error (HTTP {}): {}", status, sanitized);
                }
            }

            let link = body_text.trim();
            if !link.starts_with("http") {
                bail!("temp.sh returned invalid URL: {}", link);
            }

            Ok(link.to_string())
        })
    }
}

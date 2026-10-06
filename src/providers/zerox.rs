use anyhow::{bail, Result};
use reqwest::header::USER_AGENT;
use reqwest::multipart::{Form, Part};

use crate::config::Config;
use crate::provider::{create_stream_body, BoxFuture, Provider, ProviderMetadata, UploadContext};

pub struct ZeroxProvider;

impl Provider for ZeroxProvider {
    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "zerox",
            description: "0x0.st (Disabled upstream due to bot spam)",
            default_enabled: false,
            max_size_bytes: Some(512 * 1024 * 1024),
            retention: "30-365 days",
            supports_expiry: false,
        }
    }

    fn check_availability(&self, is_explicit: bool, config: &Config) -> Result<()> {
        let has_custom = config.providers.get("zerox").map(|p| p.base_url.is_some()).unwrap_or(false);
        if !has_custom {
            if is_explicit {
                bail!("0x0.st disabled uploads upstream due to botnet spam; uploads are currently unavailable.");
            } else {
                bail!("zerox is disabled by default");
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

            let form = Form::new().part("file", part);

            let custom_base = config
                .providers
                .get("zerox")
                .and_then(|p| p.base_url.as_deref());
            let endpoint = base_url_override
                .or(custom_base)
                .unwrap_or("https://0x0.st");

            let resp = client
                .post(endpoint)
                .header(USER_AGENT, "pushy/0.1")
                .multipart(form)
                .send()
                .await?;

            let status = resp.status();
            let body_text = resp.text().await?;

            if body_text.contains("uploads disabled") {
                bail!("0x0.st rejected upload: uploads disabled upstream due to botnet spam");
            }

            if !status.is_success() {
                let sanitized = crate::provider::sanitize_error_body(&body_text);
                if sanitized.is_empty() {
                    bail!("0x0.st error (HTTP {})", status);
                } else {
                    bail!("0x0.st error (HTTP {}): {}", status, sanitized);
                }
            }

            let link = body_text.trim();
            if !link.starts_with("http") {
                bail!("0x0.st returned invalid URL: {}", link);
            }

            Ok(link.to_string())
        })
    }
}

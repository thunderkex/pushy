use anyhow::{bail, Result};

use crate::config::Config;
use crate::provider::{create_stream_body, BoxFuture, Provider, ProviderMetadata, UploadContext};

pub struct TransfershProvider;

impl Provider for TransfershProvider {
    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "transfersh",
            description: "Transfer.sh (Requires custom base_url; public host unreliable)",
            default_enabled: false,
            max_size_bytes: Some(10 * 1024 * 1024 * 1024),
            retention: "14 days",
            supports_expiry: false,
        }
    }

    fn check_availability(&self, is_explicit: bool, config: &Config) -> Result<()> {
        let has_custom = config
            .providers
            .get("transfersh")
            .and_then(|p| p.base_url.as_ref())
            .is_some();
        if !has_custom {
            if is_explicit {
                bail!("transfer.sh public instance is unreachable; configure a custom base_url in config.");
            } else {
                bail!("transfersh is disabled by default (no base_url configured)");
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

            let custom_base = config
                .providers
                .get("transfersh")
                .and_then(|p| p.base_url.as_deref());

            let base = base_url_override
                .or(custom_base)
                .unwrap_or("https://transfer.sh");

            let url = format!("{}/{}", base.trim_end_matches('/'), ctx.file_name);
            let resp = client.put(&url).body(body).send().await?;

            let status = resp.status();
            let body_text = resp.text().await?;

            if !status.is_success() {
                bail!("transfer.sh error (HTTP {}): {}", status, body_text);
            }

            let link = body_text.trim();
            if !link.starts_with("http") {
                bail!("transfer.sh returned invalid URL: {}", link);
            }

            Ok(link.to_string())
        })
    }
}

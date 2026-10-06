use anyhow::{bail, Result};
use reqwest::multipart::{Form, Part};

use crate::config::Config;
use crate::provider::{create_stream_body, BoxFuture, Provider, ProviderMetadata, UploadContext};

pub struct LitterboxProvider;

fn normalize_time(time_str: Option<&str>) -> &'static str {
    match time_str {
        Some("1h") | Some("1") => "1h",
        Some("12h") | Some("12") => "12h",
        Some("24h") | Some("1d") | Some("24") => "24h",
        Some("72h") | Some("3d") | Some("72") => "72h",
        _ => "24h",
    }
}

impl Provider for LitterboxProvider {
    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "litterbox",
            description: "Temporary file hosting (1h, 12h, 24h, 72h; 1 GB limit)",
            default_enabled: true,
            max_size_bytes: Some(1024 * 1024 * 1024),
            retention: "1h / 12h / 24h / 72h",
            supports_expiry: true,
        }
    }

    fn check_availability(&self, _is_explicit: bool, _config: &Config) -> Result<()> {
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

            let cfg_time = config
                .providers
                .get("litterbox")
                .and_then(|p| p.time.as_deref());
            let time_val = normalize_time(ctx.expire.as_deref().or(cfg_time));

            let form = Form::new()
                .text("reqtype", "fileupload")
                .text("time", time_val)
                .part("fileToUpload", part);

            let endpoint = base_url_override
                .unwrap_or("https://litterbox.catbox.moe/resources/internals/api.php");
            let resp = client.post(endpoint).multipart(form).send().await?;

            let status = resp.status();
            let body_text = resp.text().await?;

            if !status.is_success() {
                bail!("litterbox error (HTTP {}): {}", status, body_text);
            }

            let link = body_text.trim();
            if !link.starts_with("http") {
                bail!("litterbox returned invalid URL: {}", link);
            }

            Ok(link.to_string())
        })
    }
}

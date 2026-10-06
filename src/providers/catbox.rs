use anyhow::{bail, Result};
use reqwest::multipart::{Form, Part};

use crate::config::Config;
use crate::provider::{create_stream_body, BoxFuture, Provider, ProviderMetadata, UploadContext};

pub struct CatboxProvider;

impl Provider for CatboxProvider {
    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "catbox",
            description: "Permanent file hosting (~200 MB limit)",
            default_enabled: true,
            max_size_bytes: Some(200 * 1024 * 1024),
            retention: "Permanent",
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
        config: &'a Config,
        progress_bar: Option<indicatif::ProgressBar>,
        base_url_override: Option<&'a str>,
    ) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            let body = create_stream_body(&ctx.file_path, progress_bar).await?;
            let part = Part::stream_with_length(body, ctx.file_size)
                .file_name(ctx.file_name.clone());

            let mut form = Form::new()
                .text("reqtype", "fileupload")
                .part("fileToUpload", part);

            let userhash = config
                .catbox_userhash
                .as_ref()
                .or_else(|| config.providers.get("catbox").and_then(|p| p.userhash.as_ref()));

            if let Some(uh) = userhash {
                form = form.text("userhash", uh.clone());
            }

            let endpoint = base_url_override.unwrap_or("https://catbox.moe/user/api.php");
            let resp = client.post(endpoint).multipart(form).send().await?;

            let status = resp.status();
            let body_text = resp.text().await?;

            if !status.is_success() {
                bail!("catbox error (HTTP {}): {}", status, body_text);
            }

            let link = body_text.trim();
            if !link.starts_with("http") {
                bail!("catbox returned invalid URL: {}", link);
            }

            Ok(link.to_string())
        })
    }
}

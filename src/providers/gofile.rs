use std::fs;
use anyhow::{bail, Context, Result};
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::provider::{create_stream_body, BoxFuture, Provider, ProviderMetadata, UploadContext};

#[derive(Serialize, Deserialize)]
struct GofileState {
    token: String,
}

#[derive(Deserialize)]
struct GofileResponse {
    status: String,
    data: Option<GofileData>,
}

#[derive(Deserialize)]
struct GofileData {
    #[serde(rename = "downloadPage")]
    download_page: Option<String>,
    #[serde(rename = "guestToken")]
    guest_token: Option<String>,
}

pub struct GofileProvider;

impl Provider for GofileProvider {
    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "gofile",
            description: "Fast cloud file sharing (Unlimited size)",
            default_enabled: true,
            max_size_bytes: None,
            retention: "Active (removed after inactivity)",
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

            let mut form = Form::new().part("file", part);

            let mut active_token = config
                .gofile_token
                .clone()
                .or_else(|| config.providers.get("gofile").and_then(|p| p.token.clone()));

            if active_token.is_none() {
                let state_file = Config::state_dir().join("gofile.json");
                if state_file.exists() {
                    if let Ok(content) = fs::read_to_string(&state_file) {
                        if let Ok(state) = serde_json::from_str::<GofileState>(&content) {
                            active_token = Some(state.token);
                        }
                    }
                }
            }

            if let Some(tok) = &active_token {
                form = form.text("token", tok.clone());
            }

            let endpoint = base_url_override.unwrap_or("https://upload.gofile.io/uploadfile");
            let resp = client.post(endpoint).multipart(form).send().await?;

            let status = resp.status();
            let body_text = resp.text().await?;

            if !status.is_success() {
                bail!("gofile error (HTTP {}): {}", status, body_text);
            }

            let gofile_resp: GofileResponse = serde_json::from_str(&body_text)
                .with_context(|| format!("Failed to parse gofile JSON: {}", body_text))?;

            if gofile_resp.status != "ok" {
                bail!("gofile upload failed: status is {}", gofile_resp.status);
            }

            let data = gofile_resp.data.context("Missing gofile response data")?;
            let download_page = data.download_page.context("Missing downloadPage in gofile response")?;

            if let Some(new_token) = data.guest_token {
                if config.gofile_token.is_none() {
                    let state_dir = Config::state_dir();
                    let _ = fs::create_dir_all(&state_dir);
                    let state_file = state_dir.join("gofile.json");
                    let state_data = GofileState { token: new_token };
                    if let Ok(json) = serde_json::to_string(&state_data) {
                        let _ = fs::write(&state_file, json);
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = fs::set_permissions(&state_file, fs::Permissions::from_mode(0o600));
                        }
                    }
                }
            }

            Ok(download_page)
        })
    }
}

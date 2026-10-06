use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_provider")]
    pub provider: String,

    #[serde(default = "default_fallback")]
    pub fallback: Vec<String>,

    #[serde(default)]
    pub copy: bool,

    #[serde(default)]
    pub all: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub catbox_userhash: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub gofile_token: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixeldrain_key: Option<String>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub userhash: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
}

fn default_provider() -> String {
    "catbox".to_string()
}

fn default_fallback() -> Vec<String> {
    vec![
        "catbox".to_string(),
        "tempsh".to_string(),
        "gofile".to_string(),
        "tmpfiles".to_string(),
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            fallback: default_fallback(),
            copy: false,
            all: false,
            catbox_userhash: None,
            gofile_token: None,
            pixeldrain_key: None,
            providers: HashMap::new(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        if let Ok(val) = std::env::var("PUSHY_CONFIG") {
            return PathBuf::from(val);
        }
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("pushy").join("config.toml");
        }
        if let Some(mut base) = dirs::config_dir() {
            base.push("pushy");
            base.push("config.toml");
            return base;
        }
        PathBuf::from("pushy.toml")
    }

    pub fn state_dir() -> PathBuf {
        if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
            return PathBuf::from(xdg).join("pushy");
        }
        if let Some(state) = dirs::state_dir() {
            return state.join("pushy");
        }
        if let Some(data) = dirs::data_local_dir() {
            return data.join("pushy");
        }
        PathBuf::from(".pushy_state")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(parsed) = toml::from_str::<Config>(&content) {
                    return parsed;
                }
            }
        }
        Config::default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config to TOML")?;
        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            let _ = fs::set_permissions(path, permissions);
        }

        Ok(())
    }
}

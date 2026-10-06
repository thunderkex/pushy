use std::path::Path;
use std::process::ExitCode;
use clap::Parser;

use pushy::cli::{Cli, Commands};
use pushy::config::Config;
use pushy::providers;
use pushy::setup;
use pushy::uploader::{self, UploadResult, Uploader};

#[cfg(feature = "clipboard")]
fn copy_to_clipboard(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text);
    }
}

#[cfg(not(feature = "clipboard"))]
fn copy_to_clipboard(_text: &str) {}

fn open_editor(path: &Path) -> anyhow::Result<()> {
    println!("Config path: {}", path.display());
    if !path.exists() {
        let default_cfg = Config::default();
        default_cfg.save(path)?;
    }

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "notepad".to_string()
            } else {
                "nano".to_string()
            }
        });

    let parts = shlex::split(&editor).unwrap_or_else(|| vec![editor.clone()]);
    let (cmd, args) = parts
        .split_first()
        .map(|(c, a)| (c.as_str(), a))
        .unwrap_or(("notepad", &[][..]));

    let mut command = std::process::Command::new(cmd);
    command.args(args);
    command.arg(path);

    let status = command.status();

    match status {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("pushy: failed to launch editor '{}': {}", editor, e);
            Ok(())
        }
    }
}

fn print_providers(config: &Config) {
    println!("{:<12} {:<8} {:<15} {:<25} DESCRIPTION", "PROVIDER", "STATUS", "LIMIT", "RETENTION");
    println!("{}", "-".repeat(80));

    for p in providers::all_providers() {
        let meta = p.metadata();
        let is_configured_on = p.check_availability(false, config).is_ok()
            || config.provider == meta.name
            || config.fallback.contains(&meta.name.to_string());

        let status = if is_configured_on { "ON" } else { "OFF" };
        let limit = meta
            .max_size_bytes
            .map(uploader::format_bytes)
            .unwrap_or_else(|| "Unlimited".to_string());

        println!(
            "{:<12} {:<8} {:<15} {:<25} {}",
            meta.name, status, limit, meta.retention, meta.description
        );
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let config = Config::load();

    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Setup => {
                if let Err(e) = setup::run_setup() {
                    eprintln!("pushy setup error: {}", e);
                    return ExitCode::from(1);
                }
                return ExitCode::SUCCESS;
            }
            Commands::Providers => {
                print_providers(&config);
                return ExitCode::SUCCESS;
            }
            Commands::Config => {
                let path = Config::config_path();
                if let Err(e) = open_editor(&path) {
                    eprintln!("pushy config error: {}", e);
                    return ExitCode::from(1);
                }
                return ExitCode::SUCCESS;
            }
        }
    }

    if cli.files.is_empty() {
        eprintln!("pushy: missing file operand");
        eprintln!("Try 'pushy --help' for more information.");
        return ExitCode::from(2);
    }

    let uploader = Uploader::new(config.clone(), cli.quiet);
    let mut any_failed = false;

    for target in &cli.files {
        let result = if target == "-" {
            if cli.name.is_none() {
                eprintln!("pushy: stdin upload requires --name <filename>");
                return ExitCode::from(2);
            }

            match Uploader::buffer_stdin_to_temp().await {
                Ok((_temp_guard, temp_path)) => {
                    uploader
                        .upload_file(
                            &temp_path,
                            cli.name.as_deref(),
                            cli.provider.as_deref(),
                            cli.expire.as_deref(),
                            cli.all,
                        )
                        .await
                }
                Err(e) => Err(e),
            }
        } else {
            let path = Path::new(target);
            if !path.exists() {
                eprintln!("pushy: cannot access '{}': No such file or directory", target);
                return ExitCode::from(2);
            }

            uploader
                .upload_file(
                    path,
                    cli.name.as_deref(),
                    cli.provider.as_deref(),
                    cli.expire.as_deref(),
                    cli.all,
                )
                .await
        };

        match result {
            Ok(UploadResult::Single(link)) => {
                println!("{}", link);
                if config.copy {
                    copy_to_clipboard(&link);
                }
            }
            Ok(UploadResult::All(links)) => {
                let mut copied = Vec::new();
                for (prov, link) in &links {
                    if cli.quiet {
                        println!("{}", link);
                    } else {
                        println!("{:<10} {}", format!("{}:", prov), link);
                    }
                    copied.push(link.clone());
                }
                if config.copy && !copied.is_empty() {
                    copy_to_clipboard(&copied.join("\n"));
                }
            }
            Err(e) => {
                eprintln!("pushy error: {}", e);
                any_failed = true;
            }
        }
    }

    if any_failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

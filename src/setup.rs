use anyhow::Result;
use dialoguer::{Confirm, Input, Select};

use crate::config::Config;

pub fn run_setup() -> Result<()> {
    println!("=== pushy setup wizard ===\n");
    println!("Press Enter to accept defaults.\n");

    let provider_options = [
        "catbox    — Permanent file hosting (~200 MB limit) [Recommended]",
        "all       — Upload to ALL providers simultaneously by default",
        "tempsh    — Temporary file hosting (3 days, 4 GB limit)",
        "gofile    — Fast cloud file sharing (Unlimited size)",
        "tmpfiles  — Temporary direct file sharing (~1h, 100 MB limit)",
        "litterbox — Temporary file hosting (1h-72h, 1 GB limit)",
    ];

    let provider_names = ["catbox", "all", "tempsh", "gofile", "tmpfiles", "litterbox"];

    let default_idx = 0;
    let selected_idx = Select::new()
        .with_prompt("1. Default provider?")
        .items(&provider_options)
        .default(default_idx)
        .interact()?;

    let chosen_provider = provider_names[selected_idx].to_string();

    let copy_clipboard = Confirm::new()
        .with_prompt("2. Copy the link to clipboard after upload?")
        .default(false)
        .interact()?;

    println!("\n3. Account tokens (optional):");
    let catbox_userhash: String = Input::new()
        .with_prompt("   Catbox userhash")
        .allow_empty(true)
        .interact_text()?;

    let gofile_token: String = Input::new()
        .with_prompt("   Gofile token")
        .allow_empty(true)
        .interact_text()?;

    let config_path = Config::config_path();
    let mut config = Config::load();

    config.all = chosen_provider == "all";
    config.provider = chosen_provider.clone();
    config.copy = copy_clipboard;
    config.catbox_userhash = if catbox_userhash.trim().is_empty() {
        None
    } else {
        Some(catbox_userhash.trim().to_string())
    };
    config.gofile_token = if gofile_token.trim().is_empty() {
        None
    } else {
        Some(gofile_token.trim().to_string())
    };

    config.save(&config_path)?;

    println!("\nSuccess! Config written to: {}", config_path.display());
    println!("\nExample usage:");
    println!("  pushy photo.png");
    println!("  pushy -p {} document.pdf", chosen_provider);

    Ok(())
}

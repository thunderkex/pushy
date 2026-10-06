# pushy

Simple file uploader with zero configuration and automatic provider fallback.

```bash
# 1. Download the binary for your OS and put it in your PATH
# 2. (Optional) Run the 20-second setup wizard:
pushy setup

# 3. Upload a file and get a link:
pushy photo.png
```

Works with zero config out of the box: no account or config file required. If the primary host is down, `pushy` silently falls back to the next available provider.

---

## Installation (Linux)

To make `pushy` available globally from anywhere in your terminal:

```bash
# Option A: System-wide (recommended)
sudo mv pushy /usr/local/bin/
sudo chmod +x /usr/local/bin/pushy

# Option B: User-only (no sudo required)
mkdir -p ~/.local/bin
mv pushy ~/.local/bin/
chmod +x ~/.local/bin/pushy
# Ensure ~/.local/bin is in your PATH (e.g. in ~/.bashrc or ~/.zshrc)
```

Verify installation:
```bash
pushy --version
```

## Usage

```bash
pushy <FILE>...            # Upload one or more files (prints one link per line on stdout)
pushy - --name notes.txt   # Upload from stdin
pushy setup                # Interactive setup wizard
pushy providers            # List all providers, status, limits, and retention
pushy config               # Print config path and open in $EDITOR

# Flags
-a, --all                # Upload to all providers simultaneously
-p, --provider <name>    # Use a specific provider (e.g. -p tempsh)
-e, --expire <time>      # Expiry duration where supported (1h, 12h, 24h, 72h)
    --name <name>        # Rename file on upload (required for stdin '-')
-q, --quiet              # Print only the uploaded link(s)

## Providers

| Provider | Status | Max File Size | Retention | Notes |
|---|---|---|---|---|
| `catbox` | Default (ON) | 200 MB | Permanent | Primary default host |
| `tempsh` | Fallback (ON) | 4 GB | 3 days | High capacity fallback |
| `gofile` | Fallback (ON) | Unlimited | Inactivity-based | Reuses saved guest token |
| `tmpfiles` | Fallback (ON) | 100 MB | ~1 hour | Direct link (`/dl/`) rewrite |
| `litterbox` | Expiry (ON) | 1 GB | 1h, 12h, 24h, 72h | Temporary file host, used with `-e` |
| `zerox` (0x0.st) | OFF | 512 MB | 30-365 days | Disabled upstream due to spam |
| `fileio` | OFF | 2 GB | 14 days / 1 dl | Requires API key in config |
| `transfersh` | OFF | 10 GB | 14 days | Requires custom `base_url` |
| `pixeldrain` | OFF | 20 GB | Kept while downloaded (~120 days idle) | Requires `pixeldrain_key` in config |

---

## Building from Source

### Prerequisites

Install the Rust toolchain (Rust 1.74+):
- **Linux / macOS:**
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Windows:** Download and run [rustup-init.exe](https://rustup.rs/).

> **Note (Linux only):** If building with the default clipboard support, ensure X11/xcb development libraries are installed:
> ```bash
> # Debian / Ubuntu
> sudo apt install -y pkg-config libx11-dev libxcb-shape0-dev libxcb-xfixes0-dev
> 
> # Fedora / RHEL
> sudo dnf install -y pkgconf-pkg-config libX11-devel libxcb-devel
> 
> # Arch Linux
> sudo pacman -S --needed pkgconf libx11 libxcb
> ```
> *(Or compile without clipboard support using `cargo build --release --no-default-features`)*

---

### 1. Quick Install (All Platforms)

Clone the repository and install directly into your Cargo binary directory:

```bash
git clone https://github.com/thunderkex/pushy.git
cd pushy
cargo install --path .
```

`pushy` will automatically be installed into `~/.cargo/bin/` (or `%USERPROFILE%\.cargo\bin` on Windows), which is usually already in your system `PATH`.

---

### 2. Manual Build & Install by Platform

#### Linux

```bash
git clone https://github.com/thunderkex/pushy.git
cd pushy
cargo build --release

# Install globally:
sudo install -m 755 target/release/pushy /usr/local/bin/

# Or install for current user only:
mkdir -p ~/.local/bin
cp target/release/pushy ~/.local/bin/
```

#### macOS

```bash
git clone https://github.com/thunderkex/pushy.git
cd pushy
cargo build --release

# Install globally:
sudo cp target/release/pushy /usr/local/bin/
```

#### Windows (PowerShell)

```powershell
git clone https://github.com/thunderkex/pushy.git
cd pushy
cargo build --release

# The compiled binary is located at:
# target\release\pushy.exe

# To copy to a directory in your PATH (e.g. C:\Windows or custom tools folder):
Copy-Item target\release\pushy.exe "$env:USERPROFILE\.cargo\bin\pushy.exe"
```


## Configuration

Config location: `$XDG_CONFIG_HOME/pushy/config.toml` (or standard OS config directory). Override using `PUSHY_CONFIG=/path/to/config.toml`.

```toml
# Set to "all" to upload to all active providers simultaneously by default:
provider = "all"             # Or "catbox", "tempsh", etc.
all = true                   # When true, uploads to all providers by default
fallback = ["catbox", "tempsh", "gofile", "tmpfiles"]
copy = false

# Optional account tokens
# catbox_userhash = "your_catbox_hash"
# gofile_token = "your_gofile_token"
# pixeldrain_key = "your_pixeldrain_api_key"   # required for pixeldrain

# Advanced provider tables
[providers.litterbox]
time = "24h" # Default expiration time (1h, 12h, 24h, 72h)

[providers.fileio]
key = "your_api_key_here"

[providers.transfersh]
base_url = "https://transfer.example.com"

[providers.zerox]
base_url = "https://0x0.st"
```

## Exit Codes

- `0`: Success
- `1`: Upload failure across all candidates
- `2`: Invalid usage / bad arguments

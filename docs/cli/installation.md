# CLI Installation

## Pre-built Binaries (Recommended)

Download the latest release from GitHub for your platform:

### macOS

```bash
# Apple Silicon (M1/M2/M3)
curl -fsSL https://github.com/loonghao/agentverse/releases/latest/download/agentverse-aarch64-apple-darwin.tar.gz | tar -xz
sudo mv agentverse /usr/local/bin/

# Intel
curl -fsSL https://github.com/loonghao/agentverse/releases/latest/download/agentverse-x86_64-apple-darwin.tar.gz | tar -xz
sudo mv agentverse /usr/local/bin/
```

### Linux

```bash
# x86_64 (most servers and desktops)
curl -fsSL https://github.com/loonghao/agentverse/releases/latest/download/agentverse-x86_64-unknown-linux-gnu.tar.gz | tar -xz
sudo mv agentverse /usr/local/bin/

# Verify
agentverse --version
```

### Windows (PowerShell)

```powershell
# Download and extract
irm https://github.com/loonghao/agentverse/releases/latest/download/agentverse-x86_64-pc-windows-msvc.zip -OutFile agentverse.zip
Expand-Archive agentverse.zip -DestinationPath "$env:LOCALAPPDATA\agentverse"

# Add to PATH (run once)
$env:PATH += ";$env:LOCALAPPDATA\agentverse"
[Environment]::SetEnvironmentVariable("PATH", $env:PATH, "User")

# Verify
agentverse --version
```

## Install Script

```bash
# macOS / Linux one-liner (auto-detects architecture)
curl -fsSL https://github.com/loonghao/agentverse/raw/main/install.sh | bash
```

```powershell
# Windows PowerShell one-liner
irm https://github.com/loonghao/agentverse/raw/main/install.ps1 | iex
```

## Build from Source

Requires Rust 1.75+:

```bash
git clone https://github.com/loonghao/agentverse.git
cd agentverse
cargo build --release -p agentverse

# Copy binary
sudo cp target/release/agentverse /usr/local/bin/
```

Or install directly from git:

```bash
cargo install --git https://github.com/loonghao/agentverse agentverse
```

## Verify Installation

```bash
agentverse --version
# agentverse x.y.z

agentverse --help
```


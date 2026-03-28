#!/usr/bin/env bash
# agentverse installer script for Linux and macOS
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/loonghao/agentverse/main/install.sh | bash
#
# With specific version:
#   AGENTVERSE_VERSION="0.1.3" curl -fsSL https://raw.githubusercontent.com/loonghao/agentverse/main/install.sh | bash
#
# With custom install directory:
#   AGENTVERSE_INSTALL_DIR="$HOME/bin" curl -fsSL https://raw.githubusercontent.com/loonghao/agentverse/main/install.sh | bash
#
# With GitHub token (to avoid rate limits):
#   GITHUB_TOKEN="your_token" curl -fsSL https://raw.githubusercontent.com/loonghao/agentverse/main/install.sh | bash

set -euo pipefail

_TEMP_DIR=""

REPO_OWNER="loonghao"
REPO_NAME="agentverse"
BASE_URL="https://github.com/$REPO_OWNER/$REPO_NAME/releases"

AGENTVERSE_VERSION="${AGENTVERSE_VERSION:-}"
AGENTVERSE_INSTALL_DIR="${AGENTVERSE_INSTALL_DIR:-$HOME/.local/bin}"

# ── Logging ───────────────────────────────────────────────────────────────────

step() { printf "  \033[36magentverse\033[0m %s\n" "$1" >&2; }
ok()   { printf "  \033[32magentverse\033[0m %s\n" "$1" >&2; }
fail() { printf "  \033[31magentverse\033[0m %s\n" "$1" >&2; exit 1; }

# ── Platform detection ────────────────────────────────────────────────────────

detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="unknown-linux-gnu" ;;
        Darwin*) os="apple-darwin"      ;;
        *)       fail "Unsupported OS: $(uname -s)" ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)  arch="x86_64"  ;;
        aarch64|arm64) arch="aarch64" ;;
        *)             fail "Unsupported architecture: $(uname -m)" ;;
    esac

    echo "$arch-$os"
}

# ── Download helper ───────────────────────────────────────────────────────────

download() {
    local url="$1" dest="$2"
    local auth_opts=""
    [[ -n "${GITHUB_TOKEN:-}" ]] && auth_opts="-H \"Authorization: Bearer $GITHUB_TOKEN\""

    local max_retries=3
    for i in $(seq 1 $max_retries); do
        if command -v curl >/dev/null 2>&1; then
            if eval curl -fsSL --connect-timeout 15 --max-time 120 $auth_opts "\"$url\"" -o "\"$dest\"" 2>/dev/null; then
                local size
                size=$(stat -f%z "$dest" 2>/dev/null || stat -c%s "$dest" 2>/dev/null || echo 0)
                [[ "$size" -gt 1024 ]] && return 0
            fi
        elif command -v wget >/dev/null 2>&1; then
            if wget -q --timeout=120 "$url" -O "$dest" 2>/dev/null; then
                local size
                size=$(stat -f%z "$dest" 2>/dev/null || stat -c%s "$dest" 2>/dev/null || echo 0)
                [[ "$size" -gt 1024 ]] && return 0
            fi
        else
            fail "Neither curl nor wget is available"
        fi
        rm -f "$dest"
        [[ $i -lt $max_retries ]] && sleep 2
    done
    return 1
}

# ── Resolve latest GitHub release version ────────────────────────────────────

resolve_latest_version() {
    local auth_opts=""
    [[ -n "${GITHUB_TOKEN:-}" ]] && auth_opts="-H \"Authorization: Bearer $GITHUB_TOKEN\""

    local api_url="https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases?per_page=10"
    local json=""

    if command -v curl >/dev/null 2>&1; then
        json=$(eval curl -fsSL --connect-timeout 15 --max-time 30 $auth_opts "\"$api_url\"" 2>/dev/null || true)
    elif command -v wget >/dev/null 2>&1; then
        json=$(wget -qO- --timeout=30 "$api_url" 2>/dev/null || true)
    fi

    [[ -z "$json" ]] && return 1

    local tag=""
    tag=$(printf '%s' "$json" | awk '
        BEGIN { found=0; in_r=0; cur=""; is_draft=0; is_pre=0; has_assets=0 }
        /"tag_name"/ {
            if (in_r && cur != "" && !is_draft && !is_pre && has_assets) { print cur; found=1; exit }
            in_r=1; is_draft=0; is_pre=0; has_assets=0
            s=$0; gsub(/.*"tag_name"[[:space:]]*:[[:space:]]*"/, "", s); gsub(/".*/, "", s); cur=s
        }
        /"draft"[[:space:]]*:[[:space:]]*true/      { is_draft=1 }
        /"prerelease"[[:space:]]*:[[:space:]]*true/ { is_pre=1 }
        /"browser_download_url"/                    { has_assets=1 }
        END { if (!found && in_r && cur!="" && !is_draft && !is_pre && has_assets) print cur }
    ' | head -1)

    [[ -z "$tag" ]] && return 1
    tag="${tag#v}"
    printf '%s\n' "$tag"
}

# ── Main ──────────────────────────────────────────────────────────────────────

main() {
    local platform
    platform=$(detect_platform)

    step "Installing agentverse CLI for $(uname -s)..."
    step "Detected: $(uname -s) $(uname -m) -> $platform"

    _TEMP_DIR=$(mktemp -d)
    trap 'rm -rf "$_TEMP_DIR"' EXIT

    local ver=""
    if [[ -z "$AGENTVERSE_VERSION" || "$AGENTVERSE_VERSION" == "latest" ]]; then
        ver=$(resolve_latest_version || true)
        [[ -z "$ver" ]] && fail "Could not resolve latest version. Set AGENTVERSE_VERSION explicitly."
        step "Resolved latest version: $ver"
    else
        ver="${AGENTVERSE_VERSION#v}"
    fi

    # Build download URL candidates (try versioned name then unversioned)
    local candidates=(
        "$BASE_URL/download/v$ver/agentverse-$ver-$platform.tar.gz"
        "$BASE_URL/download/v$ver/agentverse-$platform.tar.gz"
        "$BASE_URL/latest/download/agentverse-$platform.tar.gz"
    )

    local archive_path=""
    for url in "${candidates[@]}"; do
        local dest="$_TEMP_DIR/${url##*/}"
        step "Trying: $url"
        if download "$url" "$dest"; then
            archive_path="$dest"
            break
        fi
    done

    [[ -z "$archive_path" ]] && fail "Download failed. Try: AGENTVERSE_VERSION='$ver' $0"

    step "Extracting..."
    mkdir -p "$AGENTVERSE_INSTALL_DIR"
    tar -xzf "$archive_path" -C "$_TEMP_DIR"

    local binary
    binary=$(find "$_TEMP_DIR" -name "agentverse" -type f | head -n1)
    [[ -z "$binary" ]] && fail "agentverse binary not found in archive"

    cp "$binary" "$AGENTVERSE_INSTALL_DIR/agentverse"
    chmod +x "$AGENTVERSE_INSTALL_DIR/agentverse"

    local installed_version
    installed_version=$("$AGENTVERSE_INSTALL_DIR/agentverse" --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || echo "unknown")
    ok "Installed: agentverse $installed_version"

    # Update PATH
    if [[ ":$PATH:" != *":$AGENTVERSE_INSTALL_DIR:"* ]]; then
        local shell_config
        case "${SHELL:-bash}" in
            */zsh)  shell_config="$HOME/.zshrc"  ;;
            */fish) shell_config="$HOME/.config/fish/config.fish" ;;
            *)      shell_config="$HOME/.bashrc" ;;
        esac

        if [[ -w "$(dirname "$shell_config")" ]]; then
            { echo ""; echo "# Added by agentverse installer"; echo "export PATH=\"$AGENTVERSE_INSTALL_DIR:\$PATH\""; } >> "$shell_config"
            ok "Added to PATH in $shell_config"
        fi
        export PATH="$AGENTVERSE_INSTALL_DIR:$PATH"
    fi

    [[ -n "${GITHUB_PATH:-}" ]] && echo "$AGENTVERSE_INSTALL_DIR" >> "$GITHUB_PATH"

    echo "" >&2
    ok "agentverse installed successfully!"
    echo "" >&2
    printf "  Run: agentverse --help\n" >&2
    printf "  Self-update: agentverse self-update\n" >&2
    printf "  Docs: https://github.com/%s/%s\n" "$REPO_OWNER" "$REPO_NAME" >&2
    echo "" >&2
}

main "$@"


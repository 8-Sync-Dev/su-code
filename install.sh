#!/bin/sh
#
# 8sync standalone installer.
#
# Downloads the prebuilt `8sync` binary from GitHub Releases — no git clone,
# no Rust toolchain, no cargo build. Ideal for a fresh machine or quick upgrade.
#
#   curl -fsSL https://raw.githubusercontent.com/8-Sync-Dev/su-code/main/install.sh | sh
#
# Upgrade:   re-run the same command (atomically replaces the old binary).
# Uninstall: curl -fsSL .../install.sh | sh -s -- --uninstall
#
# Environment:
#   SUSYNC_VERSION   release tag to install (default: latest, e.g. v0.12.1)
#   SUSYNC_BIN_DIR   install location (default: ~/.local/bin)
set -eu

REPO="8-Sync-Dev/su-code"
BIN_DIR="${SUSYNC_BIN_DIR:-$HOME/.local/bin}"
BIN="$BIN_DIR/8sync"

if [ "${1:-}" = "--uninstall" ]; then
  rm -f "$BIN"
  echo "8sync uninstalled (removed $BIN)."
  exit 0
fi

# 1. Platform check — resolve os first, then arch (arm64 naming differs per-os).
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
  Linux) os="linux" ;;
  Darwin) os="darwin" ;;
  *) echo "8sync: no prebuilt binary for '$os' yet — build from source: https://github.com/$REPO (scripts/bootstrap.sh)" >&2; exit 1 ;;
esac
case "$arch" in
  x86_64|amd64) arch="x86_64" ;;
  aarch64|arm64)
    # Apple Silicon reports/uses arm64; Linux uses aarch64.
    case "$os" in
      linux) arch="aarch64" ;;
      darwin) arch="arm64" ;;
    esac
    ;;
  *) echo "8sync: no prebuilt binary for '$arch' yet — build from source: https://github.com/$REPO (scripts/bootstrap.sh)" >&2; exit 1 ;;
esac

# 2. Resolve the version (latest unless SUSYNC_VERSION is pinned).
#
# Prefer the releases/latest *web* redirect over the GitHub API: the
# unauthenticated API is rate-limited to 60 req/hour per IP (403 once
# exhausted — common on shared/cloud hosts and CI). The redirect
# (github.com/<repo>/releases/latest -> .../releases/tag/vX.Y.Z) is not.
version="${SUSYNC_VERSION:-}"
if [ -z "$version" ]; then
  version="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest" \
    | sed -n 's#.*/releases/tag/##p' | tr -d '\r')"
fi
if [ -z "$version" ]; then
  version="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
fi
[ -n "$version" ] || { echo "8sync: could not resolve latest version; set SUSYNC_VERSION (e.g. SUSYNC_VERSION=v0.12.1)." >&2; exit 1; }
# Release tags are vX.Y.Z; accept a bare X.Y.Z in SUSYNC_VERSION too.
case "$version" in v*) ;; *) version="v$version" ;; esac

# 3. Download the release asset to a temp file, then atomically replace.
asset="8sync-${version}-${os}-${arch}"
url="https://github.com/$REPO/releases/download/$version/$asset"
echo "Installing 8sync $version ($os-$arch)..."
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
curl -fSL --proto '=https' --tlsv1.2 "$url" -o "$tmp" 2>/dev/null \
  || { echo "8sync: download failed: $url" >&2; exit 1; }
[ -s "$tmp" ] || { echo "8sync: downloaded an empty file from $url" >&2; exit 1; }
chmod 0755 "$tmp"
mkdir -p "$BIN_DIR"
mv -f "$tmp" "$BIN"
trap - EXIT

echo "Installed → $BIN"
"$BIN" --version 2>/dev/null || true

# 4. PATH hint if ~/.local/bin is not yet on PATH (bash/zsh/fish).
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *)
    echo ""
    echo "$BIN_DIR is not on your PATH. Add it:"
    echo "  bash/zsh: echo 'export PATH=\"$BIN_DIR:\$PATH\"' >> ~/.bashrc   # or ~/.zshrc"
    echo "  fish:     fish_add_path -aP $BIN_DIR"
    echo "  (\`8sync setup\` also wires PATH for bash/zsh/fish automatically.)"
    ;;
esac
echo ""
echo "Done. Next steps:"
echo "  8sync setup        # full stack + config"
echo "  8sync doctor       # verify"
echo "  8sync up           # upgrade later (or re-run this installer)"

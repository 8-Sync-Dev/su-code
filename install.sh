#!/bin/sh
#
# 8sync standalone installer (Linux x86_64).
#
# Downloads the prebuilt `8sync` binary from the GitHub Releases of the PRIVATE
# repo 8syncdev/kilop — no git clone, no Rust toolchain, no cargo build. The repo
# is private, so a GitHub token with read access to it is required: GH_TOKEN,
# else GITHUB_TOKEN, else `gh auth token` (run `gh auth login` first).
#
#   gh api repos/8syncdev/kilop/contents/install.sh -H "Accept: application/vnd.github.raw" | sh
#
# Upgrade:   re-run the same command (atomically replaces the old binary), or `8sync up`.
# Uninstall: ... | sh -s -- --uninstall
#
# After the binary lands, the installer runs `8sync setup --no-profile`: the AI
# harness (gh, omp, forge + zsh, configs, skills, `harness global`).
#
# Environment:
#   GH_TOKEN / GITHUB_TOKEN  GitHub token with read access to 8syncdev/kilop
#   SUSYNC_VERSION   release tag to install (default: latest, e.g. v0.141.0)
#   SUSYNC_BIN_DIR   install location (default: ~/.local/bin)
#   SUSYNC_NO_SETUP  set to 1 to install the binary only (skip `8sync setup`)
set -eu

REPO="8syncdev/kilop"
BIN_DIR="${SUSYNC_BIN_DIR:-$HOME/.local/bin}"
BIN="$BIN_DIR/8sync"

if [ "${1:-}" = "--uninstall" ]; then
  rm -f "$BIN"
  echo "8sync uninstalled (removed $BIN)."
  exit 0
fi

# 1. Platform check — releases publish Linux x86_64 and Windows x86_64 only
# (Windows: install.ps1). Anything else builds from source.
os="$(uname -s)"
arch="$(uname -m)"
case "$os/$arch" in
  Linux/x86_64|Linux/amd64) os="linux"; arch="x86_64" ;;
  *) echo "8sync: no prebuilt binary for $os/$arch (releases ship Linux x86_64 and Windows x86_64) — build from source: https://github.com/$REPO (scripts/bootstrap.sh)" >&2; exit 1 ;;
esac

# 2. GitHub token, in the order `8sync up` uses (crates/cli/src/verbs/selfup.rs github_token):
# GH_TOKEN, GITHUB_TOKEN, then `gh auth token`. Anything that is not one plain token is ignored.
clean_token() {
  case "$1" in
    ''|*[!A-Za-z0-9_.-]*) return 1 ;;
  esac
  printf '%s' "$1"
}
TOKEN="$(clean_token "${GH_TOKEN:-}" || clean_token "${GITHUB_TOKEN:-}" \
  || { command -v gh >/dev/null 2>&1 && clean_token "$(gh auth token 2>/dev/null | tr -d '\r\n')"; } || true)"
[ -n "$TOKEN" ] || {
  echo "8sync: $REPO is private — set GH_TOKEN (or GITHUB_TOKEN), or run \`gh auth login\`, with read access to it, then re-run." >&2
  exit 1
}

# curl with the token passed through a config on stdin, so it never shows in any argv. curl drops
# a custom Authorization header on a redirect to another host, so the signed storage URL the asset
# endpoint redirects to is fetched without the token; --proto-redir keeps every hop on https.
gh_curl() {
  printf 'header = "Authorization: Bearer %s"\n' "$TOKEN" \
    | curl --config - --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL \
        -H "X-GitHub-Api-Version: 2022-11-28" -H "User-Agent: 8sync-install" "$@"
}

# 3. The release payload: latest, or the tag pinned in SUSYNC_VERSION.
version="${SUSYNC_VERSION:-}"
if [ -n "$version" ]; then
  # Release tags are vX.Y.Z; accept a bare X.Y.Z in SUSYNC_VERSION too.
  case "$version" in v*) ;; *) version="v$version" ;; esac
  rel_path="tags/$version"
else
  rel_path="latest"
fi
release="$(gh_curl --max-time 30 -H "Accept: application/vnd.github+json" \
  "https://api.github.com/repos/$REPO/releases/$rel_path" 2>/dev/null)" || {
  echo "8sync: could not read release '$rel_path' of $REPO — check the token can read the repo (gh auth status) and the tag exists." >&2
  exit 1
}
if [ -z "${SUSYNC_VERSION:-}" ]; then
  version="$(printf '%s' "$release" | tr ',' '\n' \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"
fi
case "$version" in v[0-9]*) ;; *) echo "8sync: release '$rel_path' of $REPO has no version tag." >&2; exit 1 ;; esac

# The id and sha256 digest of asset $1 in $release, printed as "<id> <hex|->". Within an asset object
# the API emits "id" before "name", then "digest", and ends with "browser_download_url"; the digest
# only counts until that closing field, so a missing digest never borrows the next asset's.
asset_info() {
  printf '%s' "$release" | tr ',' '\n' | awk -v a="$1" '
    /"id"[[:space:]]*:/ && !hit { id = $0; sub(/.*"id"[[:space:]]*:[[:space:]]*/, "", id); sub(/[^0-9].*/, "", id); next }
    /"name"[[:space:]]*:/ {
      n = $0; sub(/.*"name"[[:space:]]*:[[:space:]]*"/, "", n); sub(/".*/, "", n)
      if (n == a) { hit = 1; got = id }
      next
    }
    hit && /"digest"[[:space:]]*:[[:space:]]*"sha256:/ { d = $0; sub(/.*"sha256:/, "", d); sub(/".*/, "", d) }
    hit && /"browser_download_url"/ { exit }
    END { if (got != "") print got, (length(d) == 64 ? d : "-") }'
}

asset="8sync-${version}-${os}-${arch}"
info="$(asset_info "$asset")"
[ -n "$info" ] || { echo "8sync: release $version of $REPO has no $asset." >&2; exit 1; }
asset_id="${info%% *}"
want="${info#* }"

# 4. Download the release asset to a temp file, then atomically replace.
#
# The temp file must be a *sibling* of the destination, not $TMPDIR: on most
# distros /tmp is tmpfs while $BIN_DIR (~/.local/bin) is not, so `mv` across
# that boundary is an EXDEV copy, not a rename — leaving a partially written
# binary observable at $BIN. A same-directory rename is atomic. This mirrors
# download_and_replace() in crates/cli/src/verbs/selfup.rs.
download_asset() {
  gh_curl --max-time 300 -H "Accept: application/octet-stream" \
    "https://api.github.com/repos/$REPO/releases/assets/$1" -o "$2" 2>/dev/null
}
echo "Installing 8sync $version ($os-$arch)..."
mkdir -p "$BIN_DIR" || { echo "8sync: could not create $BIN_DIR" >&2; exit 1; }
tmp="$BIN_DIR/.8sync.new.$$"
side="$BIN_DIR/.8sync.sha256.$$"
trap 'rm -f "$tmp" "$side"' EXIT HUP INT TERM
download_asset "$asset_id" "$tmp" || { echo "8sync: download failed: $asset ($REPO $version)" >&2; exit 1; }
[ -s "$tmp" ] || { echo "8sync: downloaded an empty file for $asset" >&2; exit 1; }

# 5. Integrity: the asset's API `digest`, else the release's `<asset>.sha256` file (written by
# scripts/ci/publish-release.sh). A digest that does not match is fatal; no digest anywhere or no
# sha256 tool is a loud notice, the same policy as `8sync up`.
if [ "$want" = "-" ]; then
  side_info="$(asset_info "$asset.sha256")"
  if [ -n "$side_info" ] && download_asset "${side_info%% *}" "$side"; then
    want="$(tr -d '\r' <"$side" | awk 'NR == 1 { print $1 }')"
  fi
fi
if command -v sha256sum >/dev/null 2>&1; then
  got="$(sha256sum "$tmp" | cut -d' ' -f1)"
elif command -v shasum >/dev/null 2>&1; then
  got="$(shasum -a 256 "$tmp" | cut -d' ' -f1)"
else
  got=""
fi
if [ "${#want}" -ne 64 ]; then
  echo "  checksum: NOT VERIFIED (no sha256 digest published for $asset)"
elif [ -z "$got" ]; then
  echo "  checksum: NOT VERIFIED (no sha256sum or shasum on PATH)"
elif [ "$got" != "$(printf '%s' "$want" | tr 'A-F' 'a-f')" ]; then
  echo "8sync: checksum mismatch for $asset — refusing to install." >&2
  echo "  expected sha256:$want" >&2
  echo "  actual   sha256:$got" >&2
  exit 1
else
  echo "  checksum: ok (sha256:$got)"
fi

chmod 0755 "$tmp"
mv -f "$tmp" "$BIN"
rm -f "$side"
trap - EXIT HUP INT TERM

echo "Installed → $BIN"
"$BIN" --version 2>/dev/null || true

# 6. PATH hint if ~/.local/bin is not yet on PATH (bash/zsh/fish).
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

# 7. Harness setup. Under `curl | sh` our stdin IS this script: a child that
# reads stdin would swallow the rest of it, so give setup the terminal when
# there is one and /dev/null otherwise (setup never prompts with --no-profile;
# sudo reads the tty directly).
if [ "${SUSYNC_NO_SETUP:-0}" = "1" ]; then
  echo "SUSYNC_NO_SETUP=1 — skipped setup. Run it any time: 8sync setup"
else
  echo "Running 8sync setup --no-profile (AI harness: omp, forge, skills, MCP)..."
  if (exec </dev/tty) 2>/dev/null; then setup_in=/dev/tty; else setup_in=/dev/null; fi
  # Tools installed during setup land in these dirs; later steps must see them
  # even before the user's shell rc picks them up.
  if PATH="$BIN_DIR:$HOME/.local/bin:$HOME/.bun/bin:$PATH" "$BIN" setup --no-profile <"$setup_in"; then
    echo "Setup finished — the summary above lists any step to retry."
  else
    echo "8sync: setup reported failures — fix them and re-run: 8sync setup --no-profile" >&2
  fi
fi
echo ""
echo "Next steps:"
echo "  8sync doctor             # verify"
echo "  8sync setup              # optional Linux profiles (dev-stack, nvidia, terminal, ...)"
echo "  cd <project> && 8sync .  # omp session  ·  8sync forge <name>  # Forge zsh session"
echo "  8sync up                 # upgrade later (8sync also notifies when a release is out)"

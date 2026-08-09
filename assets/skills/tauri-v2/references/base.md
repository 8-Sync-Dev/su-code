# Tauri 2.x — canonical base (chuẩn base)

> Upstream: **Tauri 2.x** (current stable `tauri` crate **v2.11.5**, `tauri-build`
> **v2.6.3** as of 2026‑08). Docs: <https://v2.tauri.app/>. Repo:
> `tauri-apps/tauri` (v2 / `v2` branch). Plugins: `tauri-apps/plugins-workspace`.
> Config schema: <https://schema.tauri.app/config/2>.
>
> This is the load-bearing distillation — the concepts that break everything when
> wrong. It is NOT a vendored README. Follow the links for the full surface.

## 1. v2 vs v1 — the deltas that bite migrations

v2 is **not** a drop-in bump. The CLI ships `tauri migrate` to automate most of
it, but these structural changes are what a human (or agent) must verify by hand:

- **Allowlist → Capabilities/ACL.** v1's `tauri.allowlist` (a boolean map baked
  into config) is **gone**. v2 gates every privileged operation behind an ACL
  permission set declared in `src-tauri/capabilities/*.json`. This is the #1
  migration shock: a command "just stops working" until you grant it permission.
- **Monolith → plugins.** `tauri::api::{dialog,http,shell,fs,…}` and the matching
  `@tauri-apps/api/*` JS modules were extracted into `tauri-plugin-*` (Rust) /
  `@tauri-apps/plugin-*` (JS). Updater, shell, dialog, http, fs, clipboard,
  global-shortcut, notification, os, process — all plugins now. You add a crate
  dep, register `.plugin(...)`, and grant permission.
- **Config reshape.** `tauri` key → `app`; `package` removed (`productName` /
  `version` top-level); `build.distDir`→`frontendDist`, `build.devPath`→`devUrl`;
  `bundle` top-level; per-OS bundle moved (`bundle.dmg`→`bundle.macOS.dmg`,
  `bundle.deb`→`bundle.linux.deb`, `bundle.appimage`→`bundle.linux.appimage`).
- **Updater is a plugin + artifact flag.** `tauri.updater` → `plugins.updater`,
  and you **must** set `bundle.createUpdaterArtifacts: true` (or `"v1Compatible"`)
  or updates silently never ship. Signing env vars renamed:
  `TAURI_PRIVATE_KEY`→`TAURI_SIGNING_PRIVATE_KEY`.
- **lib/main split for (future) mobile.** Desktop logic moves to `lib.rs` with
  `#[cfg_attr(mobile, tauri::mobile_entry_point)] pub fn run()`; `main.rs` just
  calls `app_lib::run()`. `[lib] crate-type = ["staticlib","cdylib","rlib"]` for
  mobile; desktop-only can stay `["rlib"]`.
- **Window → WebviewWindow.** `Window` renamed `WebviewWindow` (multi-webview
  support). `@tauri-apps/api/window` → `@tauri-apps/api/webviewWindow`.
- **Events redesigned around targets.** `emit` now fans to all listeners;
  `emit_to` targets; `listen_global`→`listen_any`. JS `listen()` hears all
  unless a target is set.
- **Windows origin scheme.** Production frontends moved `https://tauri.localhost`
  → `http://tauri.localhost`, **resetting IndexedDB/LocalStorage/Cookies**. Set
  `app.windows[].useHttpsScheme: true` to preserve them.
- **Core module rename.** `@tauri-apps/api/tauri` → `@tauri-apps/api/core` (this
  is where `invoke` lives).

Full list: <https://v2.tauri.app/start/migrate/from-tauri-1/>.

## 2. The capabilities / ACL permission system (the v2 security model)

This replaces the allowlist and is the most common source of "command not
allowed" / "plugin not allowed" runtime errors.

- **Permission set files** live in `src-tauri/capabilities/*.json`. Each file has
  an `identifier`, the `windows` it applies to, and a `permissions` array.
- **Every plugin and core command exposes named permissions.** A plugin's
  `default` set (e.g. `dialog:default`) is a curated bundle; granular permissions
  are `plugin:allow-<command>` / `plugin:deny-<command>` (e.g.
  `dialog:allow-open`, `opener:allow-reveal-item-in-dir`).
- **`core:default`** grants the safe core set. Add `core:window:allow-*` etc. for
  specific window operations (needed for custom titlebars / frameless windows).
- **Scope** narrows file/network access for scoped commands (e.g. fs globs,
  `assetProtocol.scope`). Scope is a deny-by-default allowlist.
- **Debugging "command not allowed":** the error names the missing permission
  string verbatim — add exactly that string to the relevant capability file's
  `permissions` array, on the window that emits the call. Schema reference:
  `src-tauri/gen/schemas/desktop-schema.json`.

## 3. IPC model — commands, events, state

- **Commands** are `#[tauri::command]` async/sync Rust fns. Arguments are
  **flat**, not a single struct (a struct arg forces the frontend into
  `invoke('cmd', { args: {...} })` — an extra nesting the contract doesn't
  want). Special injected args (`AppHandle`, `State<'_, T>`, `Window`) are
  resolved by type, not passed by the caller. Return `Result<T, E>` where `E:
  Serialize` — `Result<T, String>` is the common idiom.
- **Registration:** list every command in
  `tauri::generate_handler![mod::cmd_a, mod::cmd_b]` passed to
  `.invoke_handler(...)`. Forgetting a command here → "command not found".
- **State:** `app.manage(your_state)` in `.setup(...)`, then read it in a command
  via `State<'_, YourState>`. Clone owned data out of a locked `State` **before**
  an `.await` — a borrow of interior state must not cross the await.
- **Events** (`.emit("name", &payload)` / frontend `listen("name", …)`): use for
  server-pushed progress/streams. **Channels** (`tauri::ipc::Channel`): a typed,
  one-way pipe created per-invoke, faster and better-typed, but it **dies with
  the invoke** that created it — pick events when state must survive a webview
  reload.
- **Frontend:** `invoke('cmd_name', { argA, argB })` from
  `@tauri-apps/api/core`; `listen('event', cb)` from `@tauri-apps/api/event`;
  `convertFileSrc(absPath)` builds a webview-playable `asset:` URL.

## 4. Plugin system

Add Rust dep `tauri-plugin-<name> = "2"`, JS dep `@tauri-apps/plugin-<name>`,
register `.plugin(tauri_plugin_name::init())` (or `::Builder`), grant the
permission set in capabilities. Official plugins live in
`tauri-apps/plugins-workspace`. The shell plugin also exposes the **sidecar**
mechanism (see §7).

## 5. Frontend-agnostic integration

Tauri ships **no frontend framework**. `build.beforeDevCommand` / `beforeBuildCommand`
run your bundler (vite/webpack/bun); `frontendDist` points at its output;
`devUrl` is the dev server. The JS API (`@tauri-apps/api`) is framework-neutral.
Guard webview-only calls with a `"__TAURI_INTERNALS__" in window` check so a
plain-browser dev build renders instead of throwing.

## 6. Bundling per-OS

- **Targets:** `bundle.targets` = `"all"` or a subset (`["deb","appimage"]`,
  `["nsis","msi"]`, `["dmg"]`). `cargo tauri build` produces them.
- **Windows:** `nsis` (recommended) or `wix/msi`. `webviewInstallMode` controls
  the WebView2 bootstrapper.
- **macOS:** `bundle.macOS.dmg`, `signingIdentity`, `entitlements` (plist),
  `minimumSystemVersion`. Apple notarization via env (`APPLE_*`).
- **Linux:** `bundle.linux.deb` (`depends`), `appimage`, `rpm`.
- **Resources/binaries:** `bundle.resources` ships extra files; the shell
  plugin's `externalBin` / `sidecar` ships external executables per-target-triple.

## 7. Sidecar / external binaries

Two models. (a) **Tauri shell-plugin sidecar:** declare in `bundle` /
`plugins.shell`, spawn scoped via `app.shell().sidecar(...)`. (b) **Hand-rolled
child process** (`tokio::process::Command` / `std::process`) speaking your own
protocol over stdin/stdout — more control, your own correlation/restart logic,
no ACL scope to configure. The 8syncdev repos below use (b) for heavy engines.

## 8. Auto-updater + code signing

- Config `plugins.updater` = `{ pubkey, endpoints[] }`. Set
  `bundle.createUpdaterArtifacts: true`.
- Generate a keypair: `tauri signer generate` → `TAURI_SIGNING_PRIVATE_KEY` /
  `…_PASSWORD` at build; the **pubkey** goes in config.
- The bundler emits `<app>.<sig>` signature files alongside each artifact and a
  `latest.json` manifest. Host `latest.json` + artifacts at `endpoints`.
- Runtime (JS `@tauri-apps/plugin-updater` or Rust `tauri-plugin-updater`):
  check → download → verify signature against pubkey → install.
- **macOS** also needs Developer ID signing + notarization for the updater to
  run; Windows wants an EV/authenticode cert for a clean SmartScreen experience.

## 9. Webview debugging

- **DevTools:** auto-enabled in debug builds. `tauri = { features =
  ["devtools"] }` to force them in release. Right-click → Inspect, or
  `WebviewWindow::open_devtools()`.
- **Linux (WebKitGTK):** `WEBKIT_DISABLE_COMPOSITING_MODE=1`,
  `WEBKIT_DISABLE_DMABUF_RENDERER=1`, `GDK_BACKEND=x11`, `JSC_useJIT=0` diagnose
  blank/black windows (NVIDIA proprietary especially). Inspect with
  `WEBKIT_INSPECTOR_PORT`.
- **CSP violations** surface in the devtools console — `app.security.csp` /
  `devCsp` must whitelist `ipc:`, `asset:`, and any connect/img origins.

## Version pin (verify before pinning)

- `tauri` = **2.11.5**, `tauri-build` = **2.6.3** (current stable, 2026‑08). Always
  re-confirm with `cargo add tauri` / <https://crates.io/crates/tauri> — the 2.x
  line moves monthly and the 8syncdev repos track the latest patch.

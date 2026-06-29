# Wallpapers

`8sync setup` deploys the default terminal/desktop wallpaper to
`~/.config/8sync/wallpaper.png` and points the kitty glass theme
(`~/.config/kitty/8sync.conf`) at it.

Resolution order (see `crates/cli/src/verbs/setup.rs::deploy_wallpaper`):

1. **Bundled** — `assets/wallpapers/default.png` (embedded in the binary). Drop a
   PNG here named `default.png` and it becomes the committed default for everyone.
2. **URL** — otherwise downloads `[ui].wallpaper_url` from `global.toml`
   (default: this repo's raw `assets/wallpapers/default.png`).

To set the default anime/dark wallpaper: commit a `default.png` here (16:9, dark,
with empty space on one side so terminal text stays legible), then rebuild
(`cargo build --release`) so rust-embed picks it up.

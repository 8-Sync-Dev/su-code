use anyhow::Result;
use owo_colors::OwoColorize;

pub fn run() -> Result<()> {
    println!("{}\n", "8sync flow — workflow theo thứ tự dùng".bold().cyan());

    section("LẦN ĐẦU CÀI MÁY MỚI", &[
        ("git clone https://github.com/8-Sync-Dev/su-code", ""),
        ("cd su-code && bash scripts/bootstrap.sh", "cài rustup (nếu thiếu) + build + install 8sync"),
        ("8sync setup", "harness (slim) + hỏi y/N từng personal profile"),
        ("# hoặc 8sync setup --yall", "cài full không hỏi"),
        ("# hoặc 8sync setup --profile alexdev", "apply bundle cá nhân hóa"),
        ("forge login", "chọn provider AI + paste API key"),
        ("gh auth status", "kiểm tra GitHub đã login chưa"),
        ("8sync doctor", "verify"),
    ]);

    section("VIBE LOOP — mở project mới → ship code", &[
        ("cd ~/code/my-app", ""),
        ("8sync .", "attach/tạo session: kitty 3-pane + forge detached qua abduco"),
        ("8sync ai \"explain codebase\"", "AI đọc AGENTS.md + agents/* tự nhớ"),
        ("8sync ai \"add login form\"", "vibe code"),
        ("8sync run dev", "start dev server (chạy nền, sống qua đóng terminal)"),
        ("8sync shot /login", "screenshot UI → forge review bằng image (rẻ token)"),
        ("8sync ai \"fix z-index header\"", ""),
        ("8sync find auth", "tìm symbol/file nhanh qua rg + fzf preview"),
        ("8sync note \"đổi sang zustand\"", "append nhanh vào agents/NOTES.md không mất flow"),
        ("8sync ship \"feat: login\"", "git add + commit + push + gh pr create"),
        ("8sync end", "AI tự đúc kết → agents/{DECISIONS,KNOWLEDGE,...}.md"),
    ]);

    section("RESUME hôm sau (hoặc reboot)", &[
        ("cd ~/code/my-app", ""),
        ("8sync .", "forge nhớ toàn bộ session trước (qua agents + AGENTS.md)"),
    ]);

    section("ĐA SESSION SONG SONG", &[
        ("8sync . ls", "liệt kê session đang sống"),
        ("8sync . to other-project", "chuyển session"),
        ("8sync . new hotfix forge", "tạo session detached phụ"),
        ("8sync . rm hotfix", "xoá session"),
        ("8sync . wipe", "xoá all session của project hiện tại"),
    ]);

    section("LOOK & FEEL", &[
        ("# HyDE quản: kitty theme + wallpaper + Hyprland config", ""),
        ("hydectl wallpaper next", "đổi wallpaper (HyDE built-in)"),
        ("hydectl theme set <name>", "đổi theme (HyDE built-in)"),
    ]);

    section("SECURITY (VPN + Firewall)", &[
        ("8sync sec", "status WARP + ufw"),
        ("8sync sec on", "bật cả WARP VPN + ufw firewall"),
        ("8sync sec off", "tắt cả 2"),
        ("8sync sec warp on", "chỉ điều khiển WARP"),
        ("8sync sec ufw on", "chỉ điều khiển ufw"),
    ]);

    section("KHI CẦN", &[
        ("8sync up", "update 8sync + forge (hệ thống pkg user tự lo)"),
        ("8sync doctor", "health check"),
        ("8sync skill", "quản lý skill cho forge"),
        ("8sync setup profile list", "liệt kê profile"),
    ]);

    println!("Mọi verb có {} hoặc {} để xem chi tiết.", "-h".bold().green(), "--help".bold().green());
    Ok(())
}

fn section(title: &str, rows: &[(&str, &str)]) {
    println!("{}", title.bold().yellow());
    let w = rows.iter().map(|(k, _)| k.len()).max().unwrap_or(20).min(40);
    for (cmd, desc) in rows {
        if desc.is_empty() {
            println!("  {}", cmd.cyan());
        } else {
            println!("  {:<w$}  {}", cmd.cyan(), desc.dimmed(), w = w);
        }
    }
    println!();
}

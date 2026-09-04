# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)
> **Active feature:** Claude Code settings & Anthropic auto-setup (`8sync harness claude-code`).

## Goal
Tự động hóa hoàn toàn việc nạp cấu hình Claude Code JSON vào `8sync` và `omp`, cấu hình User environment variables, đăng ký 9 model Claude mới nhất vào `models.yml`, đặt model default (`claude-fable-5-1`), verify live probe và đóng gói thành command + script tiện dụng.

## Checklist
- [x] Task 1: Thiết kế module `crates/cli/src/verbs/harness/claude_code.rs` và tích hợp vào `8sync harness claude-code` (aliases: `claude`, `claude-key`, `import-claude`).
- [x] Task 2: Viết standalone scripts `scripts/setup-claude-key.ps1` và `scripts/setup-claude-key.sh` hỗ trợ pipeline stdin / JSON argument.
- [x] Task 3: Đăng ký đầy đủ 9 model Claude hiện tại vào `models.yml` (override `anthropic` và `apikey-fun`) với đầy đủ context window 1M, token limit 128K và thinking mode.
- [x] Task 4: Cập nhật default modelRole sang `anthropic/claude-fable-5-1:high` và advisor sang `anthropic/claude-opus-5:high`.
- [x] Task 5: Live probe test thành công qua endpoint `https://api.apikey.fun/v1/messages` (816ms).
- [x] Task 6: Unit tests 98/98 PASS, size-gate 4.11 MB (dưới goal 4.19 MB và trần 5.18 MB).
- [x] Task 7: Cập nhật `CHANGELOG.md` và `su-code/KNOWLEDGE.md`.

## Current
Đã hoàn thành toàn bộ mã nguồn, unit tests, release build và verification live. Sẵn sàng commit, push và release theo yêu cầu của user.

## Next
Commit toàn bộ thay đổi, push lên GitHub remote và tạo tag / release nếu cần.

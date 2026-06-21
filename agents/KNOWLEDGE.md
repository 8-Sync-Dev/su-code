<!-- 8sync:harness:begin -->
## 🧠 8sync harness (epoch:1782083921)

- **Always-on skills (đọc trước tool call đầu tiên, đúng thứ tự):** codegraph → karpathy-guidelines → ponytail → assp-skill → impeccable → taste-skill → 8sync-cli → image-routing.
- **Cách tận dụng:** codegraph = explore code (search/deps/callers, không grep) · karpathy + ponytail = YAGNI, làm ít nhất, xoá > thêm · assp = copy/offer · impeccable = design system CHUẨN, BẮT BUỘC cho mọi UI/design (kèm references/house/*) + taste chống slop.
- **Sau mỗi thay đổi:** cập nhật `CHANGELOG.md` (Unreleased) + ghi học được vào file này (prefix `validated:` nếu test/build xác nhận, `hypothesis:` nếu chưa).
<!-- 8sync:harness:end -->

# KNOWLEDGE (8sync managed — append-only)

## Learnings (append-only — ghi DƯỚI đây; KHÔNG sửa block `8sync:harness` ở trên)

- **skills.toml = update source-of-truth.** `skill::discover::read_registry` parses it
  (`toml` crate → `BTreeMap<String, SkillEntry { src, when }>`); `skill::update::update_skills`
  reinstalls per recorded `src`: git deduped by URL (clone once → reinstall all sub-skills),
  `builtin:` → embedded assets (`assets::install_tree`), `path:` → symlink. Best-effort per source.
- **`.gitignore` portability rule** (`harness::memory::seed_gitignore` via `upsert_block` sentinels):
  COMMIT learned/decided (`agents/*.md`, `AGENTS.md`, `CHANGELOG.md`, `agents/skills/`); IGNORE
  derived (`.codegraph/`, `.cache/8sync/`) + secrets (`.env*`, keep `.env.example`). Note: a
  trailing-slash pattern (`.codegraph/`) only matches once the dir exists — verify `git check-ignore`
  on a path INSIDE it, not the bare name.
- **KNOWLEDGE.md managed block** (`<!-- 8sync:harness:* -->`) is overwritten every `harness up`;
  durable learnings MUST live below it in the seeded `## Learnings` zone.

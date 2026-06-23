<!-- 8sync:harness:begin -->
## 🧠 8sync harness (epoch:1782173332)

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
- **validated: `harness init` was NOT a superset of bare `harness`.** `init` (init.rs)
  only deployed bundled skills + 2 hardcoded external packs (ponytail, addyosmani) and
  never called `update_skills` — so manifest skills (feynman: deep-research, …) never
  reached `agents/skills/` via `init`. Only bare `8sync harness` (auto.rs:46) and
  `harness up --pull` read `agents/skills.toml`. Fix: init.rs now runs
  `update::update_skills(env, global_toml, None)` as step 5/9 before the mirror step.
  Verified: temp project + feynman manifest → `8sync harness init` produces
  `agents/skills/deep-research/SKILL.md` (all 20 feynman skills vendored).

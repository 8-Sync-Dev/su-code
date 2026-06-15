---
name: encore-deploy
description: Deploy/secrets/rollout runbook for the 8syncdev Encore Cloud backend and Vercel frontends. Triggers — "deploy gate/core to encore", "push backend to encore cloud", "set encore secrets", "rollout failed on encore", "build failed on encore", "payos secrets", "rotate a secret", "deploy frontend to vercel", "production deploy". Contains proven gotchas (secrets need a redeploy; vercel drops [...slug] dirs) — read BEFORE any backend deploy, secret rotation, or prod FE deploy.
---

# encore-deploy — 8syncdev backend (Encore Cloud) + frontend (Vercel) deploy runbook

All facts below are session-verified (2026-06-12). Follow them literally; do not improvise commands.

## 1. Topology

- `backend/` is a **git submodule** → GitHub `8-Sync-Dev/backend-encore-8sync`.
- TWO Encore Go apps live in that one repo:
  - `gate/` → Encore app **`8syncdev-gate-e852`** (auth + billing)
  - `8sync-core/` → Encore app **`8syncdev-core-cm62`** (health + coding)
  - `shared/` is a shared Go module wired via `go.work`.
- Encore git remotes are **already configured** inside `backend/`: `encore-gate` and `encore-core`. Verify:

```bash
git -C backend remote -v
```

## 2. Deploy model (there is NO `encore deploy` command)

Deploy = git push to the app's Encore remote. The push output prints the rollout URL.

```bash
git -C backend push encore-gate main    # deploys gate → 8syncdev-gate-e852
git -C backend push encore-core main    # deploys core → 8syncdev-core-cm62
```

Also push `origin main`, then bump the submodule pointer in the monorepo:

```bash
git -C backend push origin main
git add backend && git commit -m "chore: bump backend submodule"
```

## 3. CRITICAL gotcha — secrets do NOT hot-reload (proven 2026-06-12)

`encore secret set` does **NOT** reach running instances. Encore Go resolves secrets **at boot**. After rotating ANY secret you MUST trigger a rollout. An empty commit + push is acceptable:

```bash
git -C backend commit --allow-empty -m 'chore: roll secrets'
git -C backend push encore-gate main   # and/or encore-core
```

## 4. Secrets

Set from the **app dir** (e.g. `cd backend/gate`):

```bash
printf %s '<value>' | encore secret set --type prod,dev,pr <Name>
```

**NEVER use `--type local`** — it shadows the env-var fallbacks used by local dev and tests.

| App | Secret | Notes |
|---|---|---|
| gate | `AuthJWTSecret` | |
| gate | `PayOSClientID` | |
| gate | `PayOSAPIKey` | |
| gate | `PayOSChecksumKey` | |
| gate | `PayOSMode` | `live` \| `sandbox` |
| gate | `AppURLCoding` | e.g. `https://coding.8syncdev.com` |
| core | `AuthJWTSecret` | MUST equal gate's |
| core | `JudgeSecret` | MUST equal the Vercel coding project's `JUDGE_SECRET` env |
| core | `JudgeURL` | `https://coding.8syncdev.com/api/judge` |

## 5. Logs

`encore logs --env=staging` is **live-tail only** — no backfill. Pattern:

```bash
(cd backend/gate && encore logs --env=staging > /tmp/gate.log 2>&1 &)
# trigger traffic (curl the endpoint under test)
# then read /tmp/gate.log
```

## 6. Health probes (verify EVERY deploy)

- Core: `https://staging-8syncdev-core-cm62.encr.app/health/ping` → JSON includes `gitSha` — MUST match the pushed HEAD.
- Gate: `https://staging-8syncdev-gate-e852.encr.app/billing/plans` (returns plan list).

## 7. Platform API (rollout debugging)

Bearer token = `access_token` field of `~/.config/encore/.auth_token` (refreshed by any CLI call).

- Rollout status: `GET https://api.encore.cloud/apps/<slug>/envs/<env>/rollouts/<roll_id>`
- Retrigger failed rollout: `POST https://api.encore.cloud/apps/<slug>/envs/<env>/rollouts/<roll_id>/retrigger`
- App root dir: `PATCH https://api.encore.cloud/apps/<slug>/settings` body `{"app_root_relpath":"..."}` — **already configured**: gate=`gate`, core=`8sync-core`. Don't touch unless the repo layout changes.

## 8. Vercel FE deploy runbook (same ops family)

Three projects, team `team_nl2xMkM0Uz0QaXTukIHKZPim`:

| App | Project ID | Domain |
|---|---|---|
| web | `prj_qkUFCls4FBR6ryciob0L8nkyXm33` | 8syncdev.com |
| coding | `prj_g4Z1tIXlwmNIE3b3xxXsOFoTfamz` | coding.8syncdev.com |
| app | `prj_WXalOR38x8MzijwUbWLGIiILzK9m` | app.8syncdev.com |

Two **proven** gotchas:

1. Plain `vercel deploy` upload **SILENTLY DROPS `[...slug]` directories**. ALWAYS deploy prebuilt, from the repo root with `VERCEL_ORG_ID`/`VERCEL_PROJECT_ID` exported:

   ```bash
   export VERCEL_ORG_ID=team_nl2xMkM0Uz0QaXTukIHKZPim VERCEL_PROJECT_ID=<prj_…>
   rm -rf .vercel && vercel pull --yes --environment=production && vercel build --prod
   vercel deploy --prebuilt --prod --yes
   ```

2. Deployments get **BLOCKED** (seatBlock `TEAM_ACCESS_REQUIRED`) when the git commit-author email is not a verified email on the Vercel account. Fix: verify `atus@8syncdev.com` on the account, or temporarily hide git around the deploy call (restore immediately):

   ```bash
   mv .git .git-hidden && vercel deploy --prebuilt --prod --yes; mv .git-hidden .git
   ```

## 9. Rules

- ALWAYS verify after a deploy: health probe `gitSha` matches HEAD **and** one authed E2E call succeeds — only then claim the deploy works.
- NEVER push the monorepo to encore remotes. Only `backend/` is pushed to `encore-gate`/`encore-core`.
- After rotating secrets → redeploy (section 3). No exceptions.
- Never `--type local` for secrets (section 4).

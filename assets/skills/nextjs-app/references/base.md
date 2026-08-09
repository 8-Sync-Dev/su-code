# base.md — Next.js App Router canonical base

This is the "chuẩn base" the skill extends from. It distils the load-bearing concepts of the App Router as of **Next.js 16.3.0 / React 19.2.x** (current stable, verified on npm `next@latest`). It is a summary + citations, NOT a vendored README — follow the links for the source of truth.

## Upstream sources (authoritative)

- **Docs**: https://nextjs.org/docs (App Router) — start at https://nextjs.org/docs/app and the caching guide https://nextjs.org/docs/app/building-your-application/caching
- **Repo / changelog**: https://github.com/vercel/next.js (`canary` = bleeding edge; `latest` = stable). Release notes: https://github.com/vercel/next.js/releases
- **App Router primer**: https://nextjs.org/docs/app/building-your-application/routing
- **Server & Client Components**: https://nextjs.org/docs/app/building-your-application/rendering/composition-patterns
- **Current stable versions (npm, 2026-08-09)**: `next@16.3.0`, `react@19.2.x`, `react-dom@19.2.x`. The companion i18n lib `next-intl@4.13.5` is what 8syncdev repos pin (`^4.12.0`).

> Next 15 was the breaking-cutover release (async request APIs, cache default flip). Next 16 (current stable line, `16.x`) builds on it: `cacheComponents`/PPR stabilization track, React Compiler support, Turbopack defaults. Code written for Next 15 App Router ports to 16 with minor config deltas.

## Load-bearing concepts (the things that break everything if wrong)

### 1. The App Router mental model

- A route = a directory under `app/` (or `src/app/`). `app/dashboard/page.tsx` → `/dashboard`. Nested directories nest routes.
- **Special files** define behavior, not just rendering:
  - `page.tsx` — the route UI (public route).
  - `layout.tsx` — wraps all nested routes; preserves state across navigation. Root `layout.tsx` is required (`<html>`/`<body>` live here).
  - `loading.tsx` — Suspense fallback shown while the segment streams.
  - `error.tsx` — error boundary (MUST be a Client Component).
  - `not-found.tsx` — 404 for the segment.
  - `route.ts` — a Route Handler (API endpoint) — mutually exclusive with `page.tsx` in the same segment.
  - `template.tsx` — like layout but re-mounts on each navigation.
  - `middleware.ts` — lives at the project root (or `src/`), NOT in `app/`.
- **Route groups** `(name)` — organize without affecting the URL. **Private folders** `_name` — excluded from routing. **Dynamic segments** `[id]`, catch-all `[...slug]`, optional catch-all `[[...slug]]`.

### 2. React Server Components (RSC) vs Client Components — the boundary

- **Default = Server Component.** Runs only on the server, never ships to the browser, can be `async`, can directly read `cookies()`/`headers()`/DB/filesystem.
- **`'use client'`** at the top of a file marks it a Client Component. Required the moment you use: `useState`/`useReducer`/`useEffect`/refs, event handlers (`onClick`), browser APIs (`window`, `localStorage`), class components, or most third-party widget libs.
- The boundary is **per-module** and **one-way downhill**: a Server Component can render a Client Component; a Client Component cannot `import` a Server Component. To compose, the Server Component renders the Client Component and passes Server Components (or their serializable output) as `children`.
- **Data fetching happens in RSC.** A common bug: putting a `fetch` in a Client Component's `useEffect`, throwing away streaming + caching + SEO. Fetch in the server tree; pass props down.

### 3. Async request APIs (Next 15+ — easy to miss)

These are now **Promises** and MUST be awaited:
- `params` and `searchParams` props in pages/layouts: `const { id } = await params`.
- `cookies()`, `headers()`, `draftMode()` from `next/headers`.
- Route Handler `params`: `async function GET(req, { params }: { params: Promise<{slug:string[]}> })`.

Forgetting `await` is the most common silent bug after a 14→15/16 upgrade.

### 4. Data fetching + the fetch cache — the stale-data trap

**Next 15 changed the default: `fetch` is NO LONGER cached.** Previously (`force-cache` default in 14), an unadorned `fetch` was cached indefinitely. Now it behaves like `cache: 'no-store'` unless you opt in. This single change causes two opposite failure modes:

- *Stale data* — you (or a dependency) set `{ cache: 'force-cache' }` or `{ next: { revalidate } }` and forgot; the page serves old content.
- *Over-fetching / no cache* — you expected caching and got none; every render hits the backend.

Per-fetch knobs on `fetch(url, options)`:
- `cache: 'no-store'` — always fresh, marks the route dynamic.
- `cache: 'force-cache'` — cache indefinitely (the old default).
- `next: { revalidate: <seconds> }` — time-based (ISR-ish).
- `next: { tags: ['products'] }` — manual on-demand invalidation via `revalidateTag`.

For non-`fetch` work (DB calls), `unstable_cache(fn, keyParts, { revalidate, tags, revalidateTag })` provides the same model.

### 5. Cache invalidation — revalidatePath / revalidateTag

Both run **inside a request context** (Server Action, Route Handler, or `generateMetadata` during a request) and purge the cache:
- `revalidatePath('/dashboard')` — by URL; `revalidatePath('/', 'layout')` cascades through a layout subtree; `revalidatePath('/', 'page')` the whole cache.
- `revalidateTag('products')` — every `fetch`/`unstable_cache` tagged `'products'` is re-fetched next access.

Nothing revalidates automatically. After a write you MUST call one. **A clean `next build` does not prove invalidation works** — you must mutate, reload, and observe.

### 6. Server Actions vs Route Handlers

- **Server Action** — an `async` function in a `'use server'` file (or inline `'use server'` fn). Invoked as an RPC (POST to a compiled endpoint). Best for form submissions and mutations that should revalidate the router afterward. Can return serializable data; use `useFormState`/`useActionState` for progress.
- **Route Handler** — `app/api/<path>/route.ts` exporting HTTP verbs (`GET`, `POST`, …). Best for webhooks, third-party callbacks, streaming, CORS-controlled APIs, and BFF proxies. Gives raw `Request`/`Response`; ideal when you need to stream a body or set headers.
- When in doubt: mutation-from-UI + revalidation → Server Action; HTTP contract with outsiders → Route Handler.

### 7. Streaming + Suspense

RSC streams HTML as it resolves. Wrap slow async subtrees in `<Suspense fallback={…}>` (or use `loading.tsx`) so the shell paints fast and the slow chunk streams in. With `cacheComponents` (PPR) on, every uncached data fetch must be inside a `<Suspense>` boundary or the build errors — the boundary is the static/dynamic split.

### 8. Middleware (Edge runtime)

- Runs before routing, on (almost) every request, in the **Edge runtime** (no Node built-ins, no most npm packages, no DB drivers).
- `export { default } function middleware(req: NextRequest) { … }` + `export const config = { matcher: […] }`.
- Use for: auth redirect, locale prefix, A/B rewrite, geo redirect. Verify a JWT/cookie here; do heavy work in a Route Handler or Server Action.
- A `matcher` is essential — without it middleware runs on every asset and destroys performance.

### 9. Metadata / SEO

- Static export `metadata: Metadata` (title template, `metadataBase`, `openGraph`, `twitter`, `robots`, `alternates`/`canonical`).
- Dynamic `generateMetadata({ params, searchParams })` → returns/a `Promise<Metadata>` (await the params!).
- File conventions emit correct tags with zero code: `icon.tsx`/`apple-icon.tsx`, `manifest.ts`, `robots.ts`, `sitemap.ts`, `opengraph-image.tsx`/`twitter-image.tsx`. `metadataBase` resolves relative OG image URLs.

### 10. Deployment targets

| Target | Config | When |
|---|---|---|
| **Vercel** | (none special) — multi-project per `apps/<name>`, `turbo-ignore` for monorepo, per-project env, Edge + ISR/PPR + preview per PR | Default for 8syncdev web |
| **Self-host / Docker** | `output: 'standalone'` → build, ship `.next/standalone` + `.next/static`, run `node server.js` | When you control the host |
| **Static export** | `output: 'export'` → `out/` (NO Server Actions, NO middleware, NO ISR, NO dynamic routes without `generateStaticParams`) | Marketing/docs only |

**Env vars**: `NEXT_PUBLIC_*` is the ONLY prefix the browser receives (inlined at BUILD time). Everything else is server-only and resolved at runtime. For self-host, set non-public env at `next start`/`node server` time; for Vercel, per-project in the dashboard.

### 11. Turborepo monorepo layout

`apps/*` (each a Next app, own `next.config.ts` + port), `packages/*` (shared UI/config/api-client, consumed via `workspace:*`), `turbo.json` task graph (`build` `dependsOn: ["^build"]`, outputs `.next/**` + `encore.gen/**`), one root `packageManager` + lockfile. `transpilePackages: ["@scope/ui"]` in each app's `next.config.ts` lets Next compile the TS workspace packages. 8syncdev uses both pnpm (`pro-v2`) and Bun (`8sync-verse`) variants.

## What to check upstream when a build behaves oddly

- Async params/cookies? (Next 15+ breaking) → https://nextjs.org/docs/app/building-your-application/upgrading
- Cache flip? → https://nextjs.org/docs/app/building-your-application/caching
- `cacheComponents`/PPR Suspense requirement → the Next 16 release notes / upgrading guide.
- Edge runtime limits in middleware → https://nextjs.org/docs/app/building-your-application/routing/middleware

# patterns.md — real Next.js App Router patterns from 8syncdev repos

The opinionated extension. Every pattern is cited `repo:path` and was read from the repo (via `gh api`), not invented. These are the shapes the skill expects you to recognize and reuse. Sources: `8syncdev/crm-pro-ai` (Next 16.2.6 + React 19.2.6 + Encore.ts + next-intl), `8syncdev/8syncdev-pro-v2` (Turborepo + pnpm monorepo), `8syncdev/8sync-verse` (Turborepo + Bun + Next 16.2.3 + Encore.ts).

---

## P1. Catch-all BFF proxy Route Handler — CORS-safe bridge to the backend

**`8syncdev/crm-pro-ai:apps/web/src/app/api/backend/[...slug]/route.ts`**

The browser never calls the Encore backend directly (CORS, secret exposure). A single catch-all Route Handler forwards everything under `/api/backend/*`, injecting server-only context:

```ts
// app/api/backend/[...slug]/route.ts  (shape, trimmed)
async function handler(request, { params }: { params: Promise<{ slug: string[] }> }) {
  if (!isCsrfAllowed(request)) return Response.json({ success:false, message:'Forbidden origin' }, { status: 403 })
  const { slug } = await params                       // Next 15+: params is a Promise
  const target = buildTarget(slug, request.nextUrl.searchParams.toString())

  const h = new Headers(request.headers)
  HOP_BY_HOP.forEach(k => /* strip */);  h.delete('host');  h.set('accept-encoding','identity')

  if (!h.has('authorization')) {
    const at = request.cookies.get('ezen_at')?.value            // httpOnly cookie → bearer
    if (at) h.set('authorization', `8syncdev ${at}`)
  }
  if (!h.has('x-workspace-id'))
    h.set('x-workspace-id', request.cookies.get('active_workspace')?.value)

  const upstream = new Request(target, { method: request.method, headers: h,
    body: unsafe(request.method) ? request.body : undefined, duplex: 'half' }) // stream body
  const res = await fetch(upstream)
  return new Response(res.body, { status: res.status, headers: cleaned(res.headers) })
}
export const { GET, POST, PUT, PATCH, DELETE, OPTIONS } = { /* =handler */ }
```

**Takeaways:** (1) CSRF = compare `Origin`/`Referer` against a trusted-origins set for unsafe methods. (2) Auth lives in an **httpOnly cookie**, rehydrated into an `authorization` header server-side — the browser can't read it. (3) Strip hop-by-hop headers both ways. (4) `duplex: 'half'` streams the request body (required for body proxying in edge/web fetch). (5) Export every verb you support from one handler. This is the canonical shape when your Next app fronts an external API.

## P2. `apiFetch` — the never-throws Server Action data helper

**`8syncdev/crm-pro-ai:apps/web/src/lib/actions/common/api-client.ts`**

A single typed wrapper used by every Server Action. Two paths by environment:

```ts
function resolveUrl(path: string): string {
  if (typeof window !== 'undefined') return `/api/backend${path}`        // client → BFF proxy (P1)
  return resolveAbsoluteApiUrl(path)                                     // server → Encore direct
}
// auth: server reads cookies() dynamically; client reads a non-httpOnly cookie
// fetch(..., { cache: 'no-store' }) — CRM data is dynamic, NEVER cached (skill §2)
// NEVER throws: on error returns { success:false, message, result:undefined } as TResponse
```

**Takeaways:** (1) **`cache: 'no-store'` is pinned** — this is dynamic, user-scoped CRM data; the default-no-cache of Next 15+ is exactly right here, and they make it explicit. (2) Returning a typed error object instead of throwing keeps Server Actions total (every code path yields a value the client can render). (3) Same code runs client- and server-side via `typeof window` — the URL flips to the proxy only in the browser.

## P3. Domain-organized Server Actions

**`8syncdev/crm-pro-ai:apps/web/src/lib/actions/<domain>/`** — e.g. `person/{person.actions.ts, person.type.ts, index.ts}`, repeated for `company`, `opportunity`, `issue`, `task`, `agent`, `ai`, `auth`, `note`, `document`, `llm`, `session`, `workspace`, `webhook`, `tenant-domain`…

- Each domain = a folder: `*.actions.ts` (`'use server'` fns calling `apiFetch`), `*.type.ts` (DTOs), `index.ts` (barrel). `common/{api-client.ts,streaming-client.ts}` shared.
- `session/cookies.ts` and `workspace/cookies.ts` isolate cookie mutation (`cookies().set`/`.delete`) — cookie writes are only legal inside a Server Action/Route Handler.

**Takeaway:** colocate an action with its DTO + types per bounded domain; keep the transport (`apiFetch`) in one shared module so the cache/auth policy is central.

## P4. Env-driven runtime config + build-phase guard

**`8syncdev/crm-pro-ai:apps/web/src/lib/config/runtime-config.ts`**

```ts
const preferred = APP_ENV==='prod' ? NEXT_PUBLIC_API_URL_PROD : NEXT_PUBLIC_API_URL_LOCAL
export const API_BASE_URL = preferred ?? NEXT_PUBLIC_API_BASE_URL ?? NEXT_PUBLIC_API_URL ?? NEXT_PUBLIC_ENCORE_API_URL ?? 'http://127.0.0.1:4001'
export const WS_BASE_URL  = API_BASE_URL.replace(/^http(?=:)/,'ws')        // http→ws for streaming
export function resolveAbsoluteApiUrl(path:string){ return `${API_BASE_URL}${path.startsWith('/')?path:'/'+path}` }
export function isServerProductionBuildPhase(){ return typeof window==='undefined' && process.env.NEXT_PHASE==='phase-production-build' }
```

**Takeaways:** (1) Env resolution has a documented fallback chain — local dev works with zero env. (2) `NEXT_PHASE === 'phase-production-build'` lets you skip expensive fetches during `next build` (prerender) without runtime branches. (3) WS base URL derived from HTTP base — one env var drives both.

## P5. next-intl v4 i18n WITHOUT middleware — `[locale]` + `defineRouting` + `getRequestConfig`

**`8syncdev/crm-pro-ai:apps/web/src/i18n/{routing.ts,request.ts}` + `apps/web/next.config.ts`**

Route tree lives under `app/[locale]/(auth)/…` and `app/[locale]/(workspace)/…` (route groups for sections). No `middleware.ts` — next-intl v4 handles locale via the segment + `setRequestLocale`:

```ts
// i18n/routing.ts
export const routing = defineRouting({ locales:['vi','en'], defaultLocale:'vi',
  localePrefix:'always', localeCookie:{ name:'NEXT_LOCALE', sameSite:'lax', secure: NODE_ENV==='production' } })
// i18n/request.ts → getRequestConfig(): await requestLocale, hasLocale() guard, lazy import('./messages/vi.json'),
//   deep-merge fallback(default) ← target locale, and in DEV warn missing keys. (50+ lines, see repo)
// next.config.ts: const withNextIntl = createNextIntlPlugin(); export default withNextIntl(nextConfig)
```

**Takeaways:** (1) The `[locale]` dynamic segment IS the locale source — no middleware required for basic routing. (2) `getRequestConfig` is the place to merge a fallback dictionary so missing keys degrade gracefully (and log in dev). (3) Wire `next-intl/plugin` via `createNextIntlPlugin()` in `next.config.ts`.

## P6. `next.config.ts` — React Compiler, typedRoutes, fetch logging, deferred PPR

**`8syncdev/crm-pro-ai:apps/web/next.config.ts`**

```ts
const nextConfig: NextConfig = {
  reactCompiler: true,              // React 19 compiler (babel-plugin-react-compiler 1.0.0 in devDeps)
  typedRoutes: true,                // <Link href> is type-checked against the route tree
  logging: { fetches: { fullUrl: true } },   // see exactly what's fetched (cache debugging)
  turbopack: { root: projectRoot }, outputFileTracingRoot: projectRoot,
  images: { remotePatterns:[{protocol:'https',hostname:'**'}] }, poweredByHeader:false,
  // cacheComponents: true,   // PPR — DEFERRED: Next 16 requires every uncached fetch in <Suspense>;
  //                            retrofitting 30+ workspace pages is out of scope. Re-enable per-route.
}
```

**Takeaway:** `logging.fetches.fullUrl:true` is the fastest way to see whether a fetch is hitting the cache or the network — turn it on while debugging stale data (skill §2). PPR (`cacheComponents`) is opt-in and demands a Suspense boundary per dynamic fetch — adopt per-route, not repo-wide.

## P7. Auto-generated Encore.ts typed client (the other integration shape)

**`8syncdev/8sync-verse:packages/core/src/api/client.ts`** (header: `// Code generated by the Encore v1.56.6 client generator. DO NOT EDIT.`)

vs P1's hand-written proxy — here Encore emits a fully typed client shared as a workspace package:

```ts
export type BaseURL = string
export const Local: BaseURL = "http://localhost:4000"
export function Environment(name:string):BaseURL { return `https://${name}-on3su.encr.app` }
export function PreviewEnv(pr:number|string):BaseURL { return Environment(`pr${pr}`) }   // per-PR env
// usage: new Client(Environment('prod')).user.getUsers({page:1,size:20})  — typed per Encore service
// errors: APIError { status, code: ErrCode.Unauthenticated|...|Unknown, details }
// streaming: StreamInOut/StreamIn/StreamOut over WebSocket (encore-ws subprotocol + header passthrough)
```

**Takeaways:** (1) `PreviewEnv(pr)` gives a per-PR backend URL — pair with Vercel preview deploys (skill §8). (2) Typed `ErrCode` enum means Server Actions can `switch` on error semantics, not parse strings. (3) One generated file in `packages/core` consumed by all apps via `workspace:*` — regenerate with `encore gen client` (pro-v2 scripts this as `backend:gen:client`).

## P8. Turborepo + Bun + Vercel multi-app deployment

**`8syncdev/8sync-verse:apps/web/learn/vercel.json`** + `apps/web/learn/next.config.ts` + root `package.json`

```jsonc
// apps/web/learn/vercel.json — install & build from monorepo ROOT, turbo filters to this app
{ "framework":"nextjs",
  "installCommand":"cd ../../.. && bun install",
  "buildCommand":"cd ../../.. && bunx turbo build --filter=@8sync/learn" }
// apps/web/learn/next.config.ts
{ transpilePackages:["@8sync/ui"], output:"export" }   // some apps are static export
// apps/web/{admin,agent,learn} each: own next.config.ts, own port (3000/3001…), @8sync/ui workspace dep
```

**Takeaways:** (1) Vercel builds a monorepo app by running install+build from the root with `turbo build --filter=<pkg>` — only affected packages build. (2) `transpilePackages` lets Next compile the TS workspace UI package directly (no pre-build step). (3) Different apps can pick different `output` modes (`'export'` static vs default Node) in the same monorepo.

## P9. Turborepo task graph + Encore client generation (pnpm variant)

**`8syncdev/8syncdev-pro-v2:turbo.json`** + root `package.json`

```jsonc
// turbo.json
{ "ui":"stream", "globalEnv":["NODE_ENV","CI","VERCEL","VERCEL_ENV","ENCORE_ENV"],
  "tasks":{ "build":{ "dependsOn":["^build"], "outputs":["dist/**",".next/**","!.next/cache/**","build/**","encore.gen/**"], "env":["NEXT_PUBLIC_*","ENCORE_*"] },
            "dev":{ "cache":false, "persistent":true }, "typecheck":{ "dependsOn":["^build"] },
            "test:e2e":{ "dependsOn":["^build"], "outputs":["playwright-report/**","test-results/**"], "cache":false } } }
// root package.json: engines.node ">=22", packageManager "pnpm@9.15.0",
//   workspaces:["apps/*","backend","packages/@8sync/*","tools/*"],
//   scripts: "backend:gen:client" → pnpm --filter=@8sync/backend gen:client  (regenerate Encore client)
```

**Takeaways:** (1) `build.dependsOn:["^build"]` builds upstream workspace packages first; outputs list `.next/**` AND `encore.gen/**` so the generated client is cached. (2) Per-task `env` array tells Turbo which env vars invalidate cache — `NEXT_PUBLIC_*` + `ENCORE_*`. (3) `test:e2e` is uncached and emits `playwright-report/**`. (4) Encore client gen is a workspace script, not a manual step.

## P10. Pure SSG/RSC marketing site — no browser→backend leakage

**`8syncdev/8syncdev-pro-v2:apps/web/next.config.ts`** + `apps/web/src/app/layout.tsx`

```ts
// next.config.ts — comment in repo: "does NOT talk to backend from browser; keep free of NEXT_PUBLIC_* injections"
{ reactStrictMode:true, transpilePackages:["@8sync/tokens","@8sync/tailwind-config","@8sync/api-client"], typedRoutes:true }
// layout.tsx — static Metadata + Viewport exports, metadataBase, openGraph/twitter, skip-link a11y, <html lang="vi" data-theme="dark">
```

**Takeaways:** (1) A marketing surface should be all-RSC/SSG and ship ZERO `NEXT_PUBLIC_*` secrets — if you see one, it's a leak. (2) `metadata` + `viewport` are exported as typed objects from the root layout; `metadataBase` resolves relative OG/ canonical URLs. (3) `transpilePackages` pulls shared design tokens + tailwind config + the api-client package from the workspace.

## P11. App Router SEO file conventions (zero-config tags)

**`8syncdev/crm-pro-ai:apps/web/src/app/{icon,apple-icon,opengraph-image}.tsx` + `manifest.ts` + `robots.ts` + `sitemap.ts`** + `src/lib/seo/{build-metadata,site-url,structured-data}.ts`

The CRM uses the full file-convention set: `icon.tsx`/`apple-icon.tsx` render favicons dynamically, `manifest.ts` emits the web manifest, `opengraph-image.tsx` generates dynamic OG images, `robots.ts` + `sitemap.ts` emit crawl directives. A `lib/seo/` module centralizes `buildMetadata()` + JSON-LD `structured-data`.

**Takeaway:** prefer file conventions over hand-rolled `<meta>` tags; centralize per-page metadata building in one helper so `metadataBase`, title template, and JSON-LD stay consistent.

## P12. Embeddable widget as a separate Vite library

**`8syncdev/crm-pro-ai:apps/widget/{vite.lib.config.ts,src/widget.tsx,src/element.ts,src/theme.ts,src/identity.ts,src/api.ts}`** + backend `apps/api/src/dev/widget_api/` + web `domains/[domainId]/embed/page.tsx` + `components/modules/domains/embed-snippet-block.tsx`

The embeddable AI chat widget is NOT a Next route — it's a standalone Vite library (`vite.lib.config.ts` → distributable bundle) exposing a Custom Element (`element.ts`) + a React entry (`widget.tsx`), themed via `theme.ts`, identified via `identity.ts`, talking to a dedicated `widget_api` Encore service. The Next app only renders the **embed config page** and the **copy-paste snippet** (`embed-snippet-block.tsx`).

**Takeaway:** when a surface must be embedded on third-party sites, build it as an isolated framework-agnostic bundle (Vite lib), not a Next route — Next's runtime, RSC, and `next/script` assumptions don't survive cross-origin embedding. The Next app's job is hosting the *configuration* of that embed.

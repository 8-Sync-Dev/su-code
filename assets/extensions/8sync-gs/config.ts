// 8sync-gs — config load + validation. Pure normalize/merge logic plus a thin
// filesystem loader. Tests drive `normalizeConfig` / `mergeConfig` directly.

import type { GsConfig, GsModelRole, Stage, ThinkingLevel } from "./types.ts";

export const GS_CONFIG_SCHEMA_VERSION = 1;

const MODEL_ROLES: GsModelRole[] = [
  "coordinator",
  "research",
  "plan",
  "implement",
  "critic",
  "verify",
  "review",
  "security",
];

/** Shipped defaults — also the seed for ~/.config/8sync/gs.json. */
export const DEFAULT_CONFIG: GsConfig = {
  schemaVersion: GS_CONFIG_SCHEMA_VERSION,
  models: {
    coordinator: ["@plan", "@slow"],
    research: ["@slow", "@plan"],
    plan: ["@plan", "@slow"],
    implement: ["@task", "@default"],
    critic: ["@plan", "@slow"],
    verify: ["@plan", "@slow"],
    review: ["@plan", "@slow"],
    security: ["@plan", "@slow"],
  },
  thinking: {
    clarify: "high",
    research: "high",
    plan: "xhigh",
    plan_review: "xhigh",
    implement: "high",
    verify: "high",
    review: "xhigh",
    uat: "high",
  },
  limits: {
    maxPlanReviewLoops: 2,
    maxVerifyFailures: 3,
    maxReviewLoops: 2,
    maxParallelWorkers: 6,
    commandTimeoutSeconds: 900,
  },
  safety: {
    requireUserForDestructive: true,
    requireUserForExternalEffects: true,
    requireFinalUat: true,
  },
};

const THINKING: ThinkingLevel[] = ["minimal", "low", "medium", "high", "xhigh"];

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function stringArray(v: unknown, fallback: string[]): string[] {
  if (!Array.isArray(v)) return fallback;
  const out = v.filter((x): x is string => typeof x === "string" && x.length > 0);
  return out.length ? out : fallback;
}

function posInt(v: unknown, fallback: number): number {
  return typeof v === "number" && Number.isFinite(v) && v >= 0 ? Math.floor(v) : fallback;
}

function bool(v: unknown, fallback: boolean): boolean {
  return typeof v === "boolean" ? v : fallback;
}

/**
 * Safety flags are monotonic under project override: a project may *strengthen*
 * (set true) but never *disable* (set false) a base/global requirement. Result
 * is true when either base or override is true. Non-boolean override values are
 * ignored (fall back to base) rather than silently weakening.
 */
function mergeSafety(base: GsConfig["safety"], overRaw: unknown): GsConfig["safety"] {
  const over = isRecord(overRaw) ? overRaw : {};
  const strengthen = (key: keyof GsConfig["safety"]): boolean =>
    base[key] || (typeof over[key] === "boolean" ? over[key] : false);
  return {
    requireUserForDestructive: strengthen("requireUserForDestructive"),
    requireUserForExternalEffects: strengthen("requireUserForExternalEffects"),
    requireFinalUat: strengthen("requireFinalUat"),
  };
}

export class GsConfigError extends Error {}

/**
 * Validate + normalize a parsed config object against the defaults. Unknown
 * schema versions are rejected (never silently coerced). Missing keys fall back
 * to defaults so a partial user file still yields a complete config.
 */
export function normalizeConfig(raw: unknown, source = "config"): GsConfig {
  if (!isRecord(raw)) {
    throw new GsConfigError(`${source}: config must be a JSON object`);
  }
  const version = raw.schemaVersion;
  if (version !== undefined && version !== GS_CONFIG_SCHEMA_VERSION) {
    throw new GsConfigError(
      `${source}: unsupported gs config schemaVersion ${String(version)} (expected ${GS_CONFIG_SCHEMA_VERSION})`,
    );
  }

  const rawModels = isRecord(raw.models) ? raw.models : {};
  const models = {} as Record<GsModelRole, string[]>;
  for (const role of MODEL_ROLES) {
    models[role] = stringArray(rawModels[role], DEFAULT_CONFIG.models[role]);
  }

  const rawThinking = isRecord(raw.thinking) ? raw.thinking : {};
  const thinking: Partial<Record<Stage, ThinkingLevel>> = { ...DEFAULT_CONFIG.thinking };
  for (const key of Object.keys(rawThinking)) {
    const val = rawThinking[key];
    if (typeof val === "string" && (THINKING as string[]).includes(val)) {
      thinking[key as Stage] = val as ThinkingLevel;
    }
  }

  const rawLimits = isRecord(raw.limits) ? raw.limits : {};
  const limits = {
    maxPlanReviewLoops: posInt(rawLimits.maxPlanReviewLoops, DEFAULT_CONFIG.limits.maxPlanReviewLoops),
    maxVerifyFailures: Math.max(1, posInt(rawLimits.maxVerifyFailures, DEFAULT_CONFIG.limits.maxVerifyFailures)),
    maxReviewLoops: posInt(rawLimits.maxReviewLoops, DEFAULT_CONFIG.limits.maxReviewLoops),
    maxParallelWorkers: Math.max(1, posInt(rawLimits.maxParallelWorkers, DEFAULT_CONFIG.limits.maxParallelWorkers)),
    commandTimeoutSeconds: Math.max(
      1,
      posInt(rawLimits.commandTimeoutSeconds, DEFAULT_CONFIG.limits.commandTimeoutSeconds),
    ),
  };

  const rawSafety = isRecord(raw.safety) ? raw.safety : {};
  const safety = {
    requireUserForDestructive: bool(rawSafety.requireUserForDestructive, DEFAULT_CONFIG.safety.requireUserForDestructive),
    requireUserForExternalEffects: bool(
      rawSafety.requireUserForExternalEffects,
      DEFAULT_CONFIG.safety.requireUserForExternalEffects,
    ),
    requireFinalUat: bool(rawSafety.requireFinalUat, DEFAULT_CONFIG.safety.requireFinalUat),
  };

  return { schemaVersion: GS_CONFIG_SCHEMA_VERSION, models, thinking, limits, safety };
}

/**
 * Merge a project override onto a base config. Models/thinking/limits use
 * per-key project-wins; `safety` is monotonic (project may strengthen but never
 * disable a base/global requirement). An unsupported project `schemaVersion` is
 * rejected here rather than silently coerced to the supported version.
 */
export function mergeConfig(base: GsConfig, override: unknown): GsConfig {
  if (!isRecord(override)) return base;
  const v = override.schemaVersion;
  if (v !== undefined && v !== GS_CONFIG_SCHEMA_VERSION) {
    throw new GsConfigError(
      `project override: unsupported gs config schemaVersion ${String(v)} (expected ${GS_CONFIG_SCHEMA_VERSION})`,
    );
  }
  const merged: Record<string, unknown> = {
    schemaVersion: GS_CONFIG_SCHEMA_VERSION,
    models: { ...base.models, ...(isRecord(override.models) ? override.models : {}) },
    thinking: { ...base.thinking, ...(isRecord(override.thinking) ? override.thinking : {}) },
    limits: { ...base.limits, ...(isRecord(override.limits) ? override.limits : {}) },
    safety: mergeSafety(base.safety, override.safety),
  };
  return normalizeConfig(merged, "merged");
}

/**
 * Load config from global + optional project paths using an injected reader so
 * this stays testable without touching the real filesystem. `read` returns the
 * file text or undefined when absent. A malformed file throws GsConfigError.
 */
export function loadConfig(
  read: (path: string) => string | undefined,
  globalPath: string,
  projectPath?: string,
): GsConfig {
  const globalRaw = read(globalPath);
  let base = DEFAULT_CONFIG;
  if (globalRaw !== undefined) {
    base = normalizeConfig(parseJson(globalRaw, globalPath), globalPath);
  }
  if (projectPath) {
    const projRaw = read(projectPath);
    if (projRaw !== undefined) {
      base = mergeConfig(base, parseJson(projRaw, projectPath));
    }
  }
  return base;
}

function parseJson(text: string, source: string): unknown {
  try {
    return JSON.parse(text);
  } catch (e) {
    throw new GsConfigError(`${source}: invalid JSON — ${(e as Error).message}`);
  }
}

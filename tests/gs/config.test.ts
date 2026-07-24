import { describe, expect, test } from "bun:test";
import {
  DEFAULT_CONFIG,
  GS_CONFIG_SCHEMA_VERSION,
  GsConfigError,
  loadConfig,
  mergeConfig,
  normalizeConfig,
} from "../../assets/extensions/8sync-gs/config.ts";

describe("normalizeConfig", () => {
  test("empty object yields complete defaults", () => {
    const c = normalizeConfig({});
    expect(c.schemaVersion).toBe(GS_CONFIG_SCHEMA_VERSION);
    expect(c.safety).toEqual(DEFAULT_CONFIG.safety);
    expect(c.models).toEqual(DEFAULT_CONFIG.models);
    expect(c.limits).toEqual(DEFAULT_CONFIG.limits);
  });

  test("rejects an unsupported schemaVersion", () => {
    expect(() => normalizeConfig({ schemaVersion: 2 })).toThrow(GsConfigError);
    expect(() => normalizeConfig({ schemaVersion: 99 })).toThrow(/schemaVersion/);
  });

  test("accepts an explicit supported schemaVersion", () => {
    expect(() => normalizeConfig({ schemaVersion: 1 })).not.toThrow();
  });

  test("rejects a non-object config", () => {
    expect(() => normalizeConfig("nope")).toThrow(GsConfigError);
    expect(() => normalizeConfig(null)).toThrow(GsConfigError);
    expect(() => normalizeConfig([1, 2])).toThrow(GsConfigError);
  });

  test("partial models fall back per role", () => {
    const c = normalizeConfig({ models: { plan: ["@x"] } });
    expect(c.models.plan).toEqual(["@x"]);
    expect(c.models.critic).toEqual(DEFAULT_CONFIG.models.critic);
  });

  test("unknown thinking levels are ignored", () => {
    const c = normalizeConfig({ thinking: { plan: "bogus", research: "low" } });
    expect(c.thinking.research).toBe("low");
    expect(c.thinking.plan).toBe(DEFAULT_CONFIG.thinking.plan);
  });

  test("limits clamp to >=1 where required", () => {
    const c = normalizeConfig({ limits: { maxVerifyFailures: 0, maxParallelWorkers: 0 } });
    expect(c.limits.maxVerifyFailures).toBe(1);
    expect(c.limits.maxParallelWorkers).toBe(1);
  });

  test("safety booleans are honored", () => {
    const c = normalizeConfig({ safety: { requireFinalUat: false } });
    expect(c.safety.requireFinalUat).toBe(false);
    expect(c.safety.requireUserForDestructive).toBe(true);
  });
});

describe("mergeConfig safety is monotonic", () => {
  test("project false cannot weaken a true base requirement", () => {
    const m = mergeConfig(DEFAULT_CONFIG, {
      safety: {
        requireUserForDestructive: false,
        requireUserForExternalEffects: false,
        requireFinalUat: false,
      },
    });
    expect(m.safety.requireUserForDestructive).toBe(true);
    expect(m.safety.requireUserForExternalEffects).toBe(true);
    expect(m.safety.requireFinalUat).toBe(true);
  });

  test("project true strengthens a false base requirement", () => {
    const baseFalse = normalizeConfig({
      safety: {
        requireUserForDestructive: false,
        requireUserForExternalEffects: false,
        requireFinalUat: false,
      },
    });
    const m = mergeConfig(baseFalse, { safety: { requireUserForDestructive: true } });
    expect(m.safety.requireUserForDestructive).toBe(true);
    // unmentioned flags stay at their (false) base — strengthen only, never invented
    expect(m.safety.requireUserForExternalEffects).toBe(false);
    expect(m.safety.requireFinalUat).toBe(false);
  });

  test("non-boolean safety values are ignored (fall back to base)", () => {
    const m = mergeConfig(DEFAULT_CONFIG, { safety: { requireFinalUat: "no" } });
    expect(m.safety.requireFinalUat).toBe(true);
  });

  test("partial safety override leaves the rest at base", () => {
    const m = mergeConfig(DEFAULT_CONFIG, { safety: { requireUserForDestructive: true } });
    expect(m.safety).toEqual(DEFAULT_CONFIG.safety);
  });
});

describe("mergeConfig schemaVersion + non-safety override", () => {
  test("rejects an unsupported project override schemaVersion instead of coercing to v1", () => {
    expect(() => mergeConfig(DEFAULT_CONFIG, { schemaVersion: 2 })).toThrow(GsConfigError);
    expect(() => mergeConfig(DEFAULT_CONFIG, { schemaVersion: 7 })).toThrow(/schemaVersion/);
  });

  test("accepts a v1 project override", () => {
    expect(() => mergeConfig(DEFAULT_CONFIG, { schemaVersion: 1 })).not.toThrow();
  });

  test("models/thinking/limits use project-wins (non-safety keys)", () => {
    const m = mergeConfig(DEFAULT_CONFIG, { models: { plan: ["@x"] }, limits: { maxParallelWorkers: 3 } });
    expect(m.models.plan).toEqual(["@x"]);
    expect(m.limits.maxParallelWorkers).toBe(3);
    expect(m.models.critic).toEqual(DEFAULT_CONFIG.models.critic);
  });

  test("a non-record override returns base unchanged", () => {
    expect(mergeConfig(DEFAULT_CONFIG, "nope")).toBe(DEFAULT_CONFIG);
    expect(mergeConfig(DEFAULT_CONFIG, null)).toBe(DEFAULT_CONFIG);
  });
});

describe("loadConfig", () => {
  const reader = (files: Record<string, string>) => (p: string): string | undefined => files[p];

  test("defaults when no files are present", () => {
    expect(loadConfig(reader({}), "/g.json", "/p.json")).toEqual(DEFAULT_CONFIG);
  });

  test("global then project merge with monotonic safety", () => {
    const files = {
      "/g.json": JSON.stringify({ safety: { requireUserForDestructive: false } }),
      "/p.json": JSON.stringify({ safety: { requireUserForDestructive: true } }),
    };
    const c = loadConfig(reader(files), "/g.json", "/p.json");
    expect(c.safety.requireUserForDestructive).toBe(true); // strengthened by project
  });

  test("project false cannot weaken a global true", () => {
    const files = {
      "/g.json": JSON.stringify({ safety: { requireFinalUat: true } }),
      "/p.json": JSON.stringify({ safety: { requireFinalUat: false } }),
    };
    const c = loadConfig(reader(files), "/g.json", "/p.json");
    expect(c.safety.requireFinalUat).toBe(true);
  });

  test("malformed global JSON throws GsConfigError", () => {
    expect(() => loadConfig(reader({ "/g.json": "{bad" }), "/g.json")).toThrow(GsConfigError);
  });

  test("malformed project JSON throws GsConfigError", () => {
    expect(() => loadConfig(reader({ "/p.json": "{bad" }), "/g.json", "/p.json")).toThrow(GsConfigError);
  });

  test("unsupported project schemaVersion throws at load time", () => {
    const files = { "/p.json": JSON.stringify({ schemaVersion: 5 }) };
    expect(() => loadConfig(reader(files), "/g.json", "/p.json")).toThrow(GsConfigError);
  });

  test("global only (no project path) still normalizes", () => {
    const files = { "/g.json": JSON.stringify({ limits: { maxVerifyFailures: 7 } }) };
    const c = loadConfig(reader(files), "/g.json");
    expect(c.limits.maxVerifyFailures).toBe(7);
  });
});

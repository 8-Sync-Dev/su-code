import { describe, expect, test } from "bun:test";
import { runContractBenchmark } from "../../assets/extensions/8sync-gs/benchmark.ts";

describe("contract benchmark", () => {
  const report = runContractBenchmark();

  test("every scenario matches its GS expectation", () => {
    const failed = report.scenarios.filter((s) => !s.ok);
    expect(failed.map((s) => `${s.name}: expected ${s.gsExpected}, got ${s.gsActual}`)).toEqual([]);
  });

  test("zero critical policy escapes", () => {
    expect(report.criticalEscapes).toBe(0);
  });

  test("zero false blocks on happy paths", () => {
    expect(report.falseBlocks).toBe(0);
  });

  test("overall pass with the full scenario set", () => {
    expect(report.pass).toBe(true);
    expect(report.scenarios.length).toBeGreaterThanOrEqual(14);
  });
});

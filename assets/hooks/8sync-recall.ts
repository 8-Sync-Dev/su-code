import type { HookAPI } from "@oh-my-pi/pi-coding-agent/extensibility/hooks";
import { readFileSync, existsSync, readdirSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

// 8sync recall hook — anti-forget (the LIVE half). The static, always-apply
// directives (RULE #0 + always-on skills) live in ~/.omp/agent/APPEND_SYSTEM.md,
// which is always in the system prompt and never compacts away. This hook adds
// the per-session LIVE context at every agent-start and into every compaction
// summary: the available skill index (NAMES ONLY — progressive disclosure stays
// intact, no bodies dumped) + the live STATE Current/Next.
// Hard cap ~1k token. Fail-safe: any read error is swallowed (session unaffected).
export default function (pi: HookAPI): void {
  const home = homedir();
  const skillsDir = join(home, ".omp/skills");

  function skillIndex(): string[] {
    try {
      return readdirSync(skillsDir, { withFileTypes: true })
        .filter((e) => e.isDirectory() && !e.name.startsWith("."))
        .map((e) => e.name)
        .sort();
    } catch {
      return [];
    }
  }

  function stateHead(): string {
    try {
      const state = join(process.cwd(), "agents/STATE.md");
      if (!existsSync(state)) return "";
      const md = readFileSync(state, "utf8");
      const grab = (heading: string): string => {
        const m = md.match(new RegExp(`## ${heading}[\\s\\S]*?(?:\\n## |$)`));
        return m ? m[0].trim() : "";
      };
      return ["Current step", "Next"].map(grab).filter(Boolean).join("\n\n");
    } catch {
      return "";
    }
  }

  function bundle(): string {
    const lines: string[] = [
      "# 8sync recall — obey ~/.omp/agent/APPEND_SYSTEM.md",
      "Code-intel first (codegraph · codebase-memory-mcp · serena · headroom) BEFORE grep/Read; images → zai-vision (never guess a tool name — exact catalog: ~/.omp/capabilities.md); recall before / retain durable facts after; browser to verify web/UI; open a skill's SKILL.md before acting.",
    ];
    const skills = skillIndex();
    if (skills.length) {
      lines.push("", `## Skills available (open SKILL.md when the task matches): ${skills.join(", ")}`);
    }
    const head = stateHead();
    if (head) lines.push("", "## STATE", head);
    return lines.join("\n").slice(0, 4000);
  }

  pi.on("before_agent_start", async () => {
    const content = bundle();
    return content ? { message: { customType: "8sync-recall", content } } : undefined;
  });

  pi.on("session.compacting", async () => {
    const content = bundle();
    return content ? { context: [content] } : undefined;
  });
}

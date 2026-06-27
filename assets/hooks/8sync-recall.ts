import type { HookAPI } from "@oh-my-pi/pi-coding-agent/extensibility/hooks";
import { readFileSync, existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

// 8sync recall hook — anti-forget. Injects a lean ref bundle (skill index +
// live STATE) at every agent-start and into every compaction summary, so the
// agent keeps the skill/rule/workflow index fresh even past 50% context or
// after compaction. Lean-by-construction = the token optimization: no skill
// bodies are dumped (progressive disclosure stays intact). Hard cap ~1k token.
// Fail-safe: any read/parse error is swallowed (omp logs, session unaffected).
export default function (pi: HookAPI): void {
  const home = homedir();
  const forceLoad = join(home, ".omp/skills/00-force-load.md");

  function bundle(): string {
    const lines: string[] = ["# 8sync recall (skill index + live state)"];
    try {
      if (existsSync(forceLoad)) {
        const fl = readFileSync(forceLoad, "utf8");
        const idx = fl
          .split("\n")
          .filter((l) => /SKILL\.md|agents\/skills|\.omp\/skills|always|on-demand/i.test(l))
          .slice(0, 40);
        if (idx.length) lines.push("", "## Skills (index)", ...idx);
      }
    } catch {
      /* fail-safe: omit */
    }
    try {
      const root = process.cwd();
      const state = join(root, "agents/STATE.md");
      if (existsSync(state)) {
        const md = readFileSync(state, "utf8");
        const grab = (heading: string): string => {
          const re = new RegExp(`## ${heading}[\\s\\S]*?(?:\\n## |$)`);
          const m = md.match(re);
          return m ? m[0].trim() : "";
        };
        const head = ["Current step", "Next"].map(grab).filter(Boolean).join("\n\n");
        if (head) lines.push("", "## State", head);
      }
    } catch {
      /* fail-safe: omit */
    }
    return lines.join("\n").slice(0, 4000);
  }

  pi.on("before_agent_start", async () => {
    const content = bundle();
    if (!content) return;
    return { message: { customType: "8sync-recall", content } };
  });

  pi.on("session.compacting", async () => {
    const content = bundle();
    if (!content) return;
    return { context: [content] };
  });
}

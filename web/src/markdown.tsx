// Tiny, self-contained, XSS-safe Markdown renderer for project memory content
// (agents/STATE.md, KNOWLEDGE.md, …). Returns React nodes directly — React escapes
// text children by default, so there is no raw HTML injection surface and no
// need for dangerouslySetInnerHTML. Covers the subset these files actually use:
// headings, paragraphs, ordered/unordered lists, GFM task checkboxes, fenced +
// inline code, bold/italic/strike, links, blockquotes, and horizontal rules.
import { Fragment, type ReactNode } from "react";

type Inline =
  | { k: "text"; v: string }
  | { k: "code"; v: string }
  | { k: "strong"; c: Inline[] }
  | { k: "em"; c: Inline[] }
  | { k: "del"; c: Inline[] }
  | { k: "link"; href: string; c: Inline[] };

// One combined scan so the leftmost construct wins; alternation order keeps
// **bold** ahead of *italic* and `code` ahead of everything (protects content).
// Source kept as a string; each parseInline builds a fresh RegExp so recursive
// calls don't clobber a shared .lastIndex (which would re-match forever).
const INLINE_SRC =
  /(`[^`]+`)|(\*\*[^*\n]+\*\*)|(__[^_\n]+__)|(\*[^*\n]+\*\*)|(_[^_\n]+_)|(~~[^~\n]+~~)|(\[[^\]\n]+\]\([^)\s]+\))/.source;

function parseInline(text: string): Inline[] {
  const out: Inline[] = [];
  const re = new RegExp(INLINE_SRC, "g");
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    if (m.index > last) out.push({ k: "text", v: text.slice(last, m.index) });
    const tok = m[0];
    if (tok.startsWith("`")) out.push({ k: "code", v: tok.slice(1, -1) });
    else if (tok.startsWith("**")) out.push({ k: "strong", c: parseInline(tok.slice(2, -2)) });
    else if (tok.startsWith("__")) out.push({ k: "strong", c: parseInline(tok.slice(2, -2)) });
    else if (tok.startsWith("*")) out.push({ k: "em", c: parseInline(tok.slice(1, -1)) });
    else if (tok.startsWith("_")) out.push({ k: "em", c: parseInline(tok.slice(1, -1)) });
    else if (tok.startsWith("~~")) out.push({ k: "del", c: parseInline(tok.slice(2, -2)) });
    else {
      // [text](href) — group 7. Only linkify safe schemes; else plain text.
      const inner = m[7] ?? "";
      const sep = inner.indexOf("](");
      const label = inner.slice(1, sep);
      const href = inner.slice(sep + 2, -1);
      if (/^(https?:|mailto:|\/|\.\/|\.\.\/|#)/i.test(href)) {
        out.push({ k: "link", href, c: parseInline(label) });
      } else {
        out.push({ k: "text", v: tok });
      }
    }
    last = m.index + tok.length;
  }
  if (last < text.length) out.push({ k: "text", v: text.slice(last) });
  return out;
}

function renderInline(tokens: Inline[], keyBase: string): ReactNode[] {
  return tokens.map((t, i) => {
    const key = `${keyBase}-${i}`;
    switch (t.k) {
      case "text":
        return <Fragment key={key}>{t.v}</Fragment>;
      case "code":
        return <code key={key} className="md-code">{t.v}</code>;
      case "strong":
        return <strong key={key}>{renderInline(t.c, key)}</strong>;
      case "em":
        return <em key={key}>{renderInline(t.c, key)}</em>;
      case "del":
        return <del key={key}>{renderInline(t.c, key)}</del>;
      case "link":
        return (
          <a key={key} className="md-a" href={t.href} target="_blank" rel="noreferrer noopener">
            {renderInline(t.c, key)}
          </a>
        );
    }
  });
}

type Block =
  | { k: "h"; level: number; text: string }
  | { k: "p"; text: string }
  | { k: "code"; lang: string; text: string }
  | { k: "hr" }
  | { k: "quote"; text: string }
  | { k: "ul"; items: { checked: boolean | null; text: string }[] }
  | { k: "ol"; items: { checked: boolean | null; text: string }[] };

function parseBlocks(src: string): Block[] {
  const lines = src.replace(/\r\n?/g, "\n").split("\n");
  const blocks: Block[] = [];
  let i = 0;
  const paragraph: string[] = [];
  const flushPara = () => {
    if (paragraph.length) {
      blocks.push({ k: "p", text: paragraph.join(" ").trim() });
      paragraph.length = 0;
    }
  };
  while (i < lines.length) {
    const line = lines[i] ?? "";
    // Fenced code block.
    const fence = line.match(/^```(.*)$/);
    if (fence) {
      flushPara();
      const lang = (fence[1] ?? "").trim();
      const buf: string[] = [];
      i++;
      while (i < lines.length && !/^```/.test(lines[i] ?? "")) {
        buf.push(lines[i] ?? "");
        i++;
      }
      i++; // consume closing fence (or EOF)
      blocks.push({ k: "code", lang, text: buf.join("\n") });
      continue;
    }
    // Heading.
    const h = line.match(/^(#{1,6})\s+(.*)$/);
    if (h) {
      flushPara();
      blocks.push({ k: "h", level: (h[1] ?? "").length, text: (h[2] ?? "").trim() });
      i++;
      continue;
    }
    // Horizontal rule.
    if (/^\s*(-{3,}|\*{3,}|_{3,})\s*$/.test(line)) {
      flushPara();
      blocks.push({ k: "hr" });
      i++;
      continue;
    }
    // List (ordered or unordered), possibly with GFM task checkboxes.
    const li = line.match(/^(\s*)([-*+]|\d+\.)\s+(?:\[([ xX])\]\s+)?(.*)$/);
    if (li) {
      flushPara();
      const ordered = /^\d+\./.test(li[2] ?? "");
      const items: { checked: boolean | null; text: string }[] = [];
      while (i < lines.length) {
        const l = lines[i] ?? "";
        const m = l.match(/^\s*([-*+]|\d+\.)\s+(?:\[([ xX])\]\s+)?(.*)$/);
        if (!m) {
          // lazy continuation / sublist text on next line without bullet → append
          if (/^\s{2,}\S/.test(l) && items.length) {
            const last = items[items.length - 1];
            if (last) last.text += " " + l.trim();
            i++;
            continue;
          }
          break;
        }
        const box = m[2];
        items.push({
          checked: box === undefined ? null : box.toLowerCase() === "x",
          text: (m[3] ?? "").trim(),
        });
        i++;
      }
      blocks.push(ordered ? { k: "ol", items } : { k: "ul", items });
      continue;
    }
    // Blockquote.
    if (/^>\s?/.test(line)) {
      flushPara();
      const buf: string[] = [];
      while (i < lines.length && /^>\s?/.test(lines[i] ?? "")) {
        buf.push((lines[i] ?? "").replace(/^>\s?/, ""));
        i++;
      }
      blocks.push({ k: "quote", text: buf.join(" ").trim() });
      continue;
    }
    // Blank line → paragraph boundary.
    if (line.trim() === "") {
      flushPara();
      i++;
      continue;
    }
    paragraph.push(line.trim());
    i++;
  }
  flushPara();
  return blocks;
}

export function Markdown({ source }: { source: string }) {
  const blocks = parseBlocks(source ?? "");
  return (
    <div className="md">
      {blocks.map((b, i) => {
        const key = `b${i}`;
        switch (b.k) {
          case "h": {
            const inner = renderInline(parseInline(b.text), key);
            const lvl = Math.min(Math.max(b.level, 1), 4);
            return lvl === 1 ? (
              <h2 key={key} className="md-h md-h1">{inner}</h2>
            ) : lvl === 2 ? (
              <h3 key={key} className="md-h md-h2">{inner}</h3>
            ) : lvl === 3 ? (
              <h4 key={key} className="md-h md-h3">{inner}</h4>
            ) : (
              <h5 key={key} className="md-h md-h4">{inner}</h5>
            );
          }
          case "p":
            return <p key={key} className="md-p">{renderInline(parseInline(b.text), key)}</p>;
          case "code":
            return (
              <pre key={key} className="md-pre">
                <code className="md-blockcode">{b.text}</code>
              </pre>
            );
          case "hr":
            return <hr key={key} className="md-hr" />;
          case "quote":
            return <blockquote key={key} className="md-quote">{renderInline(parseInline(b.text), key)}</blockquote>;
          case "ul":
            return (
              <ul key={key} className="md-ul">
                {b.items.map((it, j) => (
                  <li key={`${key}-${j}`} className="md-li">
                    {it.checked !== null && (
                      <span className={`md-check ${it.checked ? "on" : ""}`} aria-hidden="true">
                        {it.checked ? "✓" : ""}
                      </span>
                    )}
                    <span>{renderInline(parseInline(it.text), `${key}-${j}`)}</span>
                  </li>
                ))}
              </ul>
            );
          case "ol":
            return (
              <ol key={key} className="md-ol">
                {b.items.map((it, j) => (
                  <li key={`${key}-${j}`} className="md-li" value={undefined}>
                    {it.checked !== null && (
                      <span className={`md-check ${it.checked ? "on" : ""}`} aria-hidden="true">
                        {it.checked ? "✓" : ""}
                      </span>
                    )}
                    <span>{renderInline(parseInline(it.text), `${key}-${j}`)}</span>
                  </li>
                ))}
              </ol>
            );
        }
      })}
    </div>
  );
}

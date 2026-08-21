---
name: super-pdf
description: Generate boardroom-grade PDF reports via an HTML design system + WeasyPrint — the same template family that produced the admired CloudGO review/architecture PDFs (kicker cover with chips strip + meta table, §N numbered section spine, navy comparison tables, colored callouts, pills, stat cards, running footer with page numbers). Use whenever asked for a beautiful/professional PDF, "báo cáo PDF đẹp", super pdf, review verdict PDF, architecture doc PDF, tài liệu giao sếp/khách, or "PDF như bản review MR".
---

# super-pdf — HTML design system → PDF A4

Turn notes/markdown into a **cited, sharp PDF**. The template IS the design system:
copy `references/report-template.html`, **keep the `<style>` block byte-identical**,
swap only the body. One command renders; one raster pass verifies.

## 1. Pipeline

```
content ready ─► cp references/report-template.html /tmp/<slug>.html
             ─► swap body content (keep CSS + component classes)
             ─► scripts/build.sh /tmp/<slug>.html [out.pdf]
             ─► pdftoppm -png -r 96 → LOOK at cover / table page / last page
             ─► cp *.pdf ~/Downloads/ (or the project's outdir)
```

```bash
~/.omp/skills/super-pdf/scripts/build.sh /tmp/my-report.html /tmp/my-report.pdf
# rendered -> /tmp/my-report.pdf
# pages: N        ← a report is 2–10 pages; a jump means a broken page-break
```

WeasyPrint runs via `uv run --with weasyprint` — no permanent install, no network
after the first dependency pull.

## 2. Content rules (the part that makes it "đẹp")

1. **§0 là kết luận.** Busy reader reads only §0. One bold verdict sentence (in a
   `.callout`) + 3–6 evidence bullets. Front-load everything.
2. **Numbered spine.** Every `h2` carries `<span class="n">§N</span>` — manual numbering,
   never auto. `h3` for sub-points, no span.
3. **Cover = 5 blocks, in order:** `.chips` strip (2–4 category chips, uppercase —
   omit when there is nothing to classify) → `.kicker` (loại tài liệu · phạm vi) →
   `h1` (conclusion-oriented title) → `.sub` (one line) → orange `.rule` → `.meta`
   table (Chuẩn bị cho / Người soạn / Ngày / Phạm vi) → `.tagbox` (central question,
   verbatim).
4. **Numbers trace to a source.** No invented figures; the `.cap` of a `.stat` names
   its origin. Citations by publisher + title + bare domain.
5. **Comparison = `table.cmp`**: `th.axis` left column, `.pill` per cell
   (`p-red`/`p-amber`/`p-green` = Yếu/TB/Tốt), `td.win` on the winner column.
6. **Never split a block ugly:** `class="avoid-break"` on every table and stat row.

## 3. Design tokens (live in the template `<style>`)

| Token | Value | Use |
|---|---|---|
| Navy | `#0f2b46` | headings, table head, strong, chip border |
| Orange | `#d47a1e` | `.rule`, `h2 .n`, stat numbers, default callout |
| Body | `#1f2733` on white, Liberation Sans 9.7pt, justified | prose |
| Page | A4 `20mm 18mm 18mm 18mm` | footer left brand · center classification · right `Trang N/M`; `@page :first` blanks it |
| `.chips` | row of bordered uppercase navy chips | cover category strip (optional) |
| `.callout` | amber default; `.blue` `#10618f` · `.green` `#1c7c54` · `.red` `#c0392b` + uppercase `.lbl` | verdict / context / pass / risk |
| `.stats` | flex row of `.stat` → `.big` number + `.cap` source | headline metrics |
| `.quote` | italic, left rule | pull-quotes |

## 4. Gotchas — each already paid for once

- `WARNING: Deprecated -weasy-hyphens: none` is **harmless** — leave it (it disables
  hyphenation for Vietnamese).
- Fonts render via **fontconfig aliases** (Liberation Sans↔Arial) — no font files;
  keep the fallback chain in `font-family`.
- **Emoji glyphs silently render as empty boxes** — replace with text (`**BẪY**`,
  `PASS`) instead of trusting a symbol.
- **Vietnamese + `pdftotext` grep**: extracted text is often NFD-normalized; grep an
  ASCII-safe substring or NFC-normalize both sides before concluding "didn't render".
- **Long tables + `page-break-inside: avoid`**: if a no-break table is taller than a
  page, WeasyPrint overflows it — split the table instead.

## 5. Verify before "done" (mandatory)

```bash
pdftoppm -png -r 96 out.pdf /tmp/pg   # cover + 1 table page + last page
```

Then actually LOOK (read the PNGs; route through zai-vision if the model cannot see):
pills/colors render, tables inside margins, footer page numbers present (not on
cover), no orphaned headings at page bottom. Only then deliver.

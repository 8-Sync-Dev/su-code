// Inline SVG icon set for the sidebar nav + wordmark. Hand-tuned to one
// coherent line family (24px grid, 1.6 stroke, round caps/joins) so no icon
// dependency is needed. Sized via CSS (.ico { width/height: 1.1em }).
import type { ReactNode } from "react";

const svg = (children: ReactNode): ReactNode => (
  <svg
    className="ico"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.6}
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
    focusable="false"
  >
    {children}
  </svg>
);

// Keyed by NAV page id. Indexing yields `ReactNode | undefined`; callers fall
// back to null, so an unknown id renders no glyph rather than crashing.
const ICONS: Record<string, ReactNode> = {
  state: svg(<path d="M3 12h4l2.5 6 4-12 2.5 6H21" />),
  context: svg(
    <>
      <path d="M4 15a8 8 0 0 1 16 0" />
      <path d="M12 15l4-4.5" />
      <circle cx="12" cy="15" r="1.1" fill="currentColor" stroke="none" />
    </>,
  ),
  skills: svg(
    <path d="M12 3l1.9 5.6 5.6 1.9-5.6 1.9L12 18l-1.9-5.6L4.5 10.5l5.6-1.9z" />,
  ),
  memory: svg(
    <>
      <ellipse cx="12" cy="5.6" rx="7" ry="2.8" />
      <path d="M5 5.6v6.9c0 1.5 3.1 2.8 7 2.8s7-1.3 7-2.8V5.6" />
      <path d="M5 9.1c0 1.5 3.1 2.8 7 2.8s7-1.3 7-2.8" />
    </>,
  ),
  engines: svg(
    <>
      <rect x="6" y="6" width="12" height="12" rx="2" />
      <rect x="9.5" y="9.5" width="5" height="5" rx="1" />
      <path d="M9 3v3M15 3v3M9 18v3M15 18v3M3 9h3M3 15h3M18 9h3M18 15h3" />
    </>,
  ),
  bench: svg(
    <>
      <path d="M3.5 20.5h17" />
      <path d="M6.5 20.5V12" />
      <path d="M12 20.5V5" />
      <path d="M17.5 20.5v-6" />
    </>,
  ),
  eval: svg(
    <>
      <path d="M12 3l7 3v5.2c0 4.4-3 7.4-7 8.8-4-1.4-7-4.4-7-8.8V6z" />
      <path d="M9 12l2.2 2.2L15.5 10" />
    </>,
  ),
  workspaces: svg(
    <>
      <rect x="3.5" y="3.5" width="7" height="7" rx="1.6" />
      <rect x="13.5" y="3.5" width="7" height="7" rx="1.6" />
      <rect x="3.5" y="13.5" width="7" height="7" rx="1.6" />
      <rect x="13.5" y="13.5" width="7" height="7" rx="1.6" />
    </>,
  ),
  team: svg(
    <>
      <circle cx="9" cy="8" r="3.2" />
      <path d="M3.5 19c0-3.3 2.5-5.6 5.5-5.6S14.5 15.7 14.5 19" />
      <path d="M16 5.1a3.2 3.2 0 0 1 0 5.9" />
      <path d="M17.6 13.7c2.1.5 3.4 2.4 3.4 5.3" />
    </>,
  ),
  submodules: svg(
    <>
      <circle cx="6" cy="6" r="2.1" />
      <circle cx="6" cy="18" r="2.1" />
      <circle cx="18" cy="8.5" r="2.1" />
      <path d="M6 8.1v7.8" />
      <path d="M18 10.6v.9a5 5 0 0 1-5 5H6" />
    </>,
  ),
  mcp: svg(
    <>
      <path d="M9 2.5v3.5M15 2.5v3.5" />
      <path d="M7 6h10v3.8a5 5 0 0 1-10 0z" />
      <path d="M12 14.8V21" />
    </>,
  ),
  rules: svg(
    <>
      <path d="M7 3h6.5L18 7.5V20a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z" />
      <path d="M13 3v5h5" />
      <path d="M9 13h6M9 16.5h4" />
    </>,
  ),
  workflow: svg(
    <>
      <circle cx="6" cy="6" r="2.3" />
      <circle cx="18" cy="6" r="2.3" />
      <circle cx="12" cy="18" r="2.3" />
      <path d="M8 7.6l3 8.2M16 7.6l-3 8.2M8.3 6h7.4" />
    </>,
  ),
};

export function NavIcon({ name }: { name: string }) {
  return <>{ICONS[name] ?? null}</>;
}

// Wordmark glyph: two sync arrows on an accent badge (badge styling lives in
// CSS via .brand-mark). White stroke reads on the violet gradient.
export function LogoMark() {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d="M4.5 10a7.5 7.5 0 0 1 12.4-3.1L19 9" />
      <path d="M19.5 14a7.5 7.5 0 0 1-12.4 3.1L5 15" />
      <path d="M19 4.5V9h-4.5M5 19.5V15h4.5" />
    </svg>
  );
}

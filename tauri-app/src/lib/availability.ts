// Per-view availability across Claude surfaces.
//
// "native":  fully native to this mode. No caveat, no banner.
// "muted":   still works, but only in part of this surface (e.g. Hooks
//            apply inside Cowork but not during regular Chat). The
//            sidebar dims the item and the view body shows a ModeBanner
//            explaining the nuance.
//
// Keep this matrix in sync with the copy in components/ModeBanner.tsx.

import type { AppMode, ViewKey } from "../types";

export type Availability = "native" | "muted";

export const AVAILABILITY: Record<ViewKey, Record<AppMode, Availability>> = {
  mcp: { cli: "native", desktop: "native" },
  // Hooks only apply to Claude Code. In the desktop app they still work
  // inside Cowork (which IS Claude Code) but do nothing during Chat.
  hooks: { cli: "native", desktop: "muted" },
  // Agents on disk are consumed by Claude Code. Cowork reads the same
  // files; Chat has no agent-creation UI at all, so users ask Claude
  // to spin one up conversationally.
  agents: { cli: "native", desktop: "muted" },
  // Same story as agents. Chat skills are added through the Customize
  // UI inside the desktop app.
  skills: { cli: "native", desktop: "muted" },
  backups: { cli: "native", desktop: "native" },
};

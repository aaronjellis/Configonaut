// ModeBanner: contextual explainer shown at the top of a view when the
// current (view, mode) pairing has caveats. For example, when the user is
// in Desktop mode and opens the Hooks view, we tell them that hooks only
// fire when Claude Code is running (including inside Cowork), not during
// regular Chat sessions.
//
// Renders nothing when the pairing is "native", so it's safe to drop into
// every view unconditionally.
//
// Keep COPY in sync with lib/availability.ts.

import { AVAILABILITY } from "../lib/availability";
import type { AppMode, ViewKey } from "../types";

interface Copy {
  title: string;
  body: string;
}

const COPY: Partial<Record<ViewKey, Partial<Record<AppMode, Copy>>>> = {
  hooks: {
    desktop: {
      title: "Hooks are a Claude Code feature",
      body:
        "The rules below fire when Claude Code is running, including inside " +
        "Cowork in the desktop app. They don't trigger during Chat.",
    },
  },
  agents: {
    desktop: {
      title: "These agents belong to Claude Code",
      body:
        "They apply to Claude Code and to Cowork inside the desktop app. " +
        "Chat has no agent-creation UI; start a conversation and ask " +
        "Claude to create the agent for you.",
    },
  },
  skills: {
    desktop: {
      title: "These skills belong to Claude Code",
      body:
        "They apply to Claude Code and to Cowork inside the desktop app. " +
        "To add skills for Chat, use the Customize UI inside the desktop app.",
    },
  },
};

interface Props {
  view: ViewKey;
  mode: AppMode;
}

export function ModeBanner({ view, mode }: Props) {
  // Bail early if this pairing is fully native; nothing to explain.
  if (AVAILABILITY[view][mode] === "native") return null;
  const copy = COPY[view]?.[mode];
  if (!copy) return null;

  return (
    <div className="mode-banner" role="note">
      <span className="mode-banner__icon" aria-hidden>
        ℹ
      </span>
      <div className="mode-banner__text">
        <div className="mode-banner__title">{copy.title}</div>
        <div className="mode-banner__body">{copy.body}</div>
      </div>
    </div>
  );
}

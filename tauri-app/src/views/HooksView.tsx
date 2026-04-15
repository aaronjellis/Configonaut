// Hooks view — ported from Sources/SupportViews.swift HooksView.
//
// Layout:
//   ┌──────────────────────────────────────────────┐
//   │ Hooks  (N)                          ↻  📁    │
//   │ Automation triggers that run when Claude…   │
//   ├──────────────────────────────────────────────┤
//   │ ● PreToolUse    *   bash ./lint.sh    [ON] › │
//   │ ● PostToolUse   …                     [ON] › │
//   │ ● Stop          …                     [OFF]› │
//   ├───────── (resize handle, when expanded) ─────┤
//   │ ┌── Editor panel ─────────────────────────┐ │
//   │ │ ● PreToolUse  [ON]  *             [x]   │ │
//   │ │ { JSON textarea }                       │ │
//   │ │ [Save]  [Disable]         ~/.claude/…   │ │
//   │ └─────────────────────────────────────────┘ │
//   │ Defined in ~/.claude/settings.json  N on, M off │
//   └──────────────────────────────────────────────┘
//
// Empty state shows the five hook types with short descriptions in a
// glass card. Clicking a row toggles the editor panel open/closed; the
// resize handle only shows when the panel is open.

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  createHook as apiCreateHook,
  deleteHook as apiDeleteHook,
  getClaudeCodeSettingsPath,
  getHookRuleJson,
  listHooks,
  toggleHook as apiToggleHook,
  updateHookRule,
} from "../api";
import { ModeBanner } from "../components/ModeBanner";
import { useToast } from "../components/Toast";
import { displayPath } from "../lib/displayPath";
import type { AppMode, HookRule } from "../types";

interface Props {
  // Used purely to decide whether to show the ModeBanner. The hook rules
  // themselves always live in `~/.claude/settings.json`.
  mode: AppMode;
  /// Same handoff as BackupsView — bumps the shell's refresh counter so
  /// future badge counts can pick up any changes.
  onMutated: () => void;
}

const DETAIL_HEIGHT_KEY = "configonaut.hooks.detailHeight";
const DETAIL_MIN_HEIGHT = 180;
const LIST_MIN_HEIGHT = 160;

// Hook events Claude Code currently fires. Keep in sync with Claude Code's
// docs; if the user picks one the runtime doesn't recognize the hook is
// simply never triggered (no harm, no error).
const HOOK_EVENTS = [
  { name: "PreToolUse", desc: "Before Claude uses a tool (can block it)" },
  { name: "PostToolUse", desc: "After a tool completes" },
  { name: "UserPromptSubmit", desc: "When the user submits a prompt" },
  { name: "Notification", desc: "When Claude sends a notification" },
  { name: "Stop", desc: "When Claude finishes a task" },
  { name: "SubagentStop", desc: "When a dispatched subagent finishes" },
  { name: "SessionStart", desc: "When a session begins" },
  { name: "SessionEnd", desc: "When a session ends" },
] as const;

const DEFAULT_NEW_EVENT: (typeof HOOK_EVENTS)[number]["name"] = "PreToolUse";

export function HooksView({ mode, onMutated }: Props) {
  const toast = useToast();
  const [hooks, setHooks] = useState<HookRule[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editedJson, setEditedJson] = useState("");
  const [editError, setEditError] = useState<string | null>(null);
  const [status, setStatusMessage] = useState("Ready.");
  const [statusIsError, setStatusIsError] = useState(false);
  const [settingsPath, setSettingsPath] = useState<string>("~/.claude/settings.json");

  const [detailHeight, setDetailHeight] = useState<number>(() => {
    const stored = localStorage.getItem(DETAIL_HEIGHT_KEY);
    if (stored) {
      const n = parseInt(stored, 10);
      if (Number.isFinite(n) && n >= DETAIL_MIN_HEIGHT) return n;
    }
    return 280;
  });

  // New-hook flow
  const [showNew, setShowNew] = useState(false);
  const [newEvent, setNewEvent] = useState<string>(DEFAULT_NEW_EVENT);
  const [newMatcher, setNewMatcher] = useState("*");
  const [newCommand, setNewCommand] = useState("");

  // Delete confirmation
  const [confirmDelete, setConfirmDelete] = useState<HookRule | null>(null);

  const containerRef = useRef<HTMLDivElement | null>(null);

  const setStatus = useCallback((msg: string, isError = false) => {
    setStatusMessage(msg);
    setStatusIsError(isError);
  }, []);

  const refresh = useCallback(async () => {
    try {
      const data = await listHooks();
      setHooks(data);
      setError(null);
      // If the currently-selected rule no longer exists (e.g. after a
      // manual edit wiped it), drop the selection so the editor closes
      // cleanly.
      setSelectedId((prev) =>
        prev && data.find((h) => h.id === prev) ? prev : null
      );
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();
    getClaudeCodeSettingsPath()
      .then((p) => setSettingsPath(displayPath(p)))
      .catch(() => {});
  }, [refresh]);

  const selected = selectedId
    ? hooks.find((h) => h.id === selectedId) ?? null
    : null;

  // When the selection changes, pull the raw JSON for the editor.
  useEffect(() => {
    if (!selected) {
      setEditedJson("");
      setEditError(null);
      return;
    }
    let cancelled = false;
    getHookRuleJson(selected.event, selected.matcher)
      .then((json) => {
        if (cancelled) return;
        setEditedJson(json);
        setEditError(null);
      })
      .catch((e) => {
        if (cancelled) return;
        setEditedJson("{}");
        setEditError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [selected]);

  async function handleToggle(rule: HookRule) {
    try {
      await apiToggleHook(rule.event, rule.matcher, !rule.isEnabled);
      const msg = `${rule.isEnabled ? "Disabled" : "Enabled"} ${rule.event} (${rule.matcher}).`;
      setStatus(msg);
      toast.show(msg, "success");
      onMutated();
      await refresh();
    } catch (e) {
      setStatus(String(e), true);
    }
  }

  async function handleSave() {
    if (!selected) return;
    setEditError(null);
    try {
      JSON.parse(editedJson);
    } catch (e) {
      setEditError(`invalid JSON: ${String(e)}`);
      return;
    }
    try {
      await updateHookRule(selected.event, selected.matcher, editedJson);
      setStatus(`Saved ${selected.event} hook.`);
      toast.show(`Saved ${selected.event} hook.`, "success");
      onMutated();
      await refresh();
    } catch (e) {
      setEditError(String(e));
    }
  }

  function openNewHookPanel() {
    setSelectedId(null);
    setShowNew(true);
    setNewEvent(DEFAULT_NEW_EVENT);
    setNewMatcher("*");
    setNewCommand("");
  }

  async function handleCreate() {
    const event = newEvent.trim();
    const matcher = newMatcher.trim() || "*";
    const command = newCommand.trim();
    if (!event || !command) return;
    try {
      await apiCreateHook(event, matcher, [command]);
      setStatus(`Created ${event} hook.`);
      toast.show(`Created ${event} hook.`, "success");
      setShowNew(false);
      setNewCommand("");
      onMutated();
      await refresh();
      // Auto-select the newly created rule so the user can jump straight
      // into editing if they want to.
      setSelectedId(`${event}::${matcher}`);
    } catch (e) {
      setStatus(`Create error: ${String(e)}`, true);
    }
  }

  async function handleDelete(rule: HookRule) {
    try {
      await apiDeleteHook(rule.event, rule.matcher);
      setStatus(`Deleted ${rule.event} hook.`);
      toast.show(`Deleted ${rule.event} hook.`, "success");
      if (selectedId === rule.id) setSelectedId(null);
      setConfirmDelete(null);
      onMutated();
      await refresh();
    } catch (e) {
      setStatus(`Delete error: ${String(e)}`, true);
    }
  }

  // --- Resize handle (see McpServersView for the full pattern) ---
  function handleResizePointerDown(
    e: ReactPointerEvent<HTMLDivElement>
  ) {
    if (e.button !== 0) return;
    e.preventDefault();
    const startY = e.clientY;
    const startHeight = detailHeight;
    const rect = containerRef.current?.getBoundingClientRect();

    const onMove = (ev: PointerEvent) => {
      const dy = ev.clientY - startY;
      let next = startHeight - dy;
      const max = rect
        ? Math.max(DETAIL_MIN_HEIGHT, rect.height - LIST_MIN_HEIGHT)
        : Number.POSITIVE_INFINITY;
      if (next < DETAIL_MIN_HEIGHT) next = DETAIL_MIN_HEIGHT;
      if (next > max) next = max;
      setDetailHeight(next);
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
      document.body.classList.remove("is-resizing");
      setDetailHeight((h) => {
        try {
          localStorage.setItem(DETAIL_HEIGHT_KEY, String(Math.round(h)));
        } catch {
          /* ignore */
        }
        return h;
      });
    };
    document.body.classList.add("is-resizing");
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }

  const enabledCount = hooks.filter((h) => h.isEnabled).length;
  const disabledCount = hooks.length - enabledCount;

  return (
    <>
      <header className="main-header" data-tauri-drag-region>
        <div className="title-block" data-tauri-drag-region>
          <div className="title-row">
            <h2>Global Hooks</h2>
            <span
              className="count-pill"
              style={{ color: "var(--blue)", background: "rgba(90, 147, 255, 0.12)" }}
            >
              {hooks.length}
            </span>
          </div>
          <div className="section-description">
            Shell commands that run automatically when Claude uses tools.
            Useful for linting, logging, or blocking risky actions before
            they happen.
          </div>
        </div>
        <div className="header-actions">
          <button className="icon" onClick={refresh} title="Reload">
            ↻
          </button>
          <button
            className="icon"
            onClick={async () => {
              try {
                await revealItemInDir(
                  settingsPath.startsWith("~")
                    ? settingsPath.replace(/^~/, homeGuess())
                    : settingsPath
                );
              } catch (e) {
                setStatus(`Couldn't reveal settings file: ${String(e)}`, true);
              }
            }}
            title="Reveal settings.json in Finder"
            aria-label="Reveal settings.json in Finder"
          >
            <svg
              viewBox="0 0 24 24"
              width="15"
              height="15"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.75"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
            </svg>
          </button>
          <button
            className="gradient-btn gradient-btn--blue"
            onClick={openNewHookPanel}
          >
            <span className="plus">+</span>
            New Hook
          </button>
        </div>
      </header>

      <div className="main-body main-body--flex" ref={containerRef}>
        <ModeBanner view="hooks" mode={mode} />
        {error && <div className="banner error">{error}</div>}

        {hooks.length === 0 ? (
          <HooksEmptyState />
        ) : (
          <div className="claude-list">
            {hooks.map((rule) => {
              const isSelected = selectedId === rule.id;
              const color = eventColorVar(rule.event);
              return (
                <div
                  key={rule.id}
                  className={`hook-row ${isSelected ? "selected" : ""} ${
                    rule.isEnabled ? "" : "off"
                  }`}
                  onClick={() =>
                    setSelectedId((prev) => (prev === rule.id ? null : rule.id))
                  }
                  style={
                    isSelected
                      ? { borderColor: `${color}55`, background: `${color}14` }
                      : undefined
                  }
                >
                  <span
                    className="glow-dot"
                    style={{
                      background: rule.isEnabled ? color : "var(--text-quaternary)",
                      boxShadow: rule.isEnabled
                        ? `0 0 6px ${color}`
                        : "none",
                    }}
                  />
                  <span
                    className="hook-event"
                    style={{ color: rule.isEnabled ? color : "var(--text-tertiary)" }}
                  >
                    {rule.event}
                  </span>
                  {rule.matcher !== "*" && (
                    <span className="hook-matcher">{rule.matcher}</span>
                  )}
                  {rule.commands[0] && (
                    <span className="hook-command">{rule.commands[0]}</span>
                  )}
                  <span className="spacer" />
                  <span
                    className="state-pill"
                    style={{
                      color: rule.isEnabled ? "var(--green)" : "var(--red)",
                      background: rule.isEnabled
                        ? "rgba(47, 214, 108, 0.12)"
                        : "rgba(255, 90, 95, 0.12)",
                    }}
                  >
                    {rule.isEnabled ? "ON" : "OFF"}
                  </span>
                  <span className="chevron">{isSelected ? "⌄" : "›"}</span>
                </div>
              );
            })}
          </div>
        )}

        {selected && (
          <>
            <div
              className="detail-resize-handle"
              onPointerDown={handleResizePointerDown}
            />
            <div
              className="detail-panel hook-editor"
              style={{ height: detailHeight, flex: "none" }}
            >
              <div className="hook-editor-header">
                <span
                  className="glow-dot"
                  style={{
                    background: selected.isEnabled
                      ? eventColorVar(selected.event)
                      : "var(--text-quaternary)",
                    boxShadow: selected.isEnabled
                      ? `0 0 6px ${eventColorVar(selected.event)}`
                      : "none",
                  }}
                />
                <span
                  className="hook-event"
                  style={{ color: eventColorVar(selected.event) }}
                >
                  {selected.event}
                </span>
                <span
                  className="state-pill"
                  style={{
                    color: selected.isEnabled ? "var(--green)" : "var(--red)",
                    background: selected.isEnabled
                      ? "rgba(47, 214, 108, 0.12)"
                      : "rgba(255, 90, 95, 0.12)",
                  }}
                >
                  {selected.isEnabled ? "ON" : "OFF"}
                </span>
                {selected.matcher !== "*" && (
                  <span className="hook-matcher">{selected.matcher}</span>
                )}
                <span className="spacer" />
                <button
                  className="icon"
                  onClick={() => setSelectedId(null)}
                  aria-label="Close editor"
                  title="Close"
                >
                  ✕
                </button>
              </div>

              <textarea
                className="hook-editor-textarea"
                value={editedJson}
                onChange={(e) => setEditedJson(e.target.value)}
                spellCheck={false}
              />

              {editError && <div className="inline-error">{editError}</div>}

              <div className="hook-editor-footer">
                <button className="primary" onClick={handleSave}>
                  Save
                </button>
                <button
                  className={selected.isEnabled ? "danger" : ""}
                  onClick={() => handleToggle(selected)}
                  style={
                    selected.isEnabled
                      ? undefined
                      : { color: "var(--green)" }
                  }
                >
                  {selected.isEnabled ? "Disable" : "Enable"}
                </button>
                <button
                  className="danger"
                  onClick={() => setConfirmDelete(selected)}
                >
                  Delete
                </button>
                <span className="spacer" />
                <span className="settings-path-hint">{settingsPath}</span>
              </div>
            </div>
          </>
        )}
      </div>

      <footer className="hooks-footer">
        <div className={`status-pill ${statusIsError ? "error" : ""}`}>
          <span
            className="dot"
            style={{
              background: statusIsError ? "var(--red)" : "var(--green)",
            }}
          />
          <span>{status}</span>
        </div>
        <div className="spacer" />
        {hooks.length > 0 && (
          <div className="footer-counts">
            {enabledCount} on, {disabledCount} off
          </div>
        )}
      </footer>

      {showNew && (
        <CreateHookModal
          event={newEvent}
          matcher={newMatcher}
          command={newCommand}
          onEventChange={setNewEvent}
          onMatcherChange={setNewMatcher}
          onCommandChange={setNewCommand}
          onCreate={handleCreate}
          onCancel={() => {
            setShowNew(false);
            setNewCommand("");
          }}
        />
      )}

      {confirmDelete && (
        <ConfirmDeleteModal
          rule={confirmDelete}
          onCancel={() => setConfirmDelete(null)}
          onConfirm={() => handleDelete(confirmDelete)}
        />
      )}
    </>
  );
}

// ----------------------------------------------------------------------
// Empty state: shows a short primer on each hook type.
// ----------------------------------------------------------------------

function HooksEmptyState() {
  const rows = [
    {
      name: "PreToolUse",
      desc: "Runs before Claude uses a tool (can block it)",
      color: "var(--blue)",
    },
    {
      name: "PostToolUse",
      desc: "Runs after a tool completes",
      color: "var(--green)",
    },
    {
      name: "Notification",
      desc: "Runs when Claude sends a notification",
      color: "var(--orange)",
    },
    {
      name: "Stop",
      desc: "Runs when Claude finishes a task",
      color: "var(--red)",
    },
  ];
  return (
    <div className="empty-wrap">
      <div className="empty-glow empty-glow--blue">
        <svg
          viewBox="0 0 24 24"
          width="34"
          height="34"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M6 3v12a3 3 0 0 0 3 3h9" />
          <circle cx="18" cy="18" r="3" />
          <path d="M15 3h3a3 3 0 0 1 3 3v3" />
        </svg>
      </div>
      <h3>No Hooks Configured</h3>
      <p className="empty-blurb">
        Hooks run custom commands when Claude performs actions. Great for
        linting, formatting, or validation. Use the New Hook button above
        to add one.
      </p>
      <div className="empty-card">
        <div className="empty-card-label">HOOK TYPES</div>
        {rows.map((r) => (
          <div key={r.name} className="hook-type-row">
            <span className="hook-type-name" style={{ color: r.color }}>
              {r.name}
            </span>
            <span className="hook-type-desc">{r.desc}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ----------------------------------------------------------------------
// Small helpers
// ----------------------------------------------------------------------

function eventColorVar(event: string): string {
  switch (event) {
    case "PreToolUse":
      return "#5a93ff";
    case "PostToolUse":
      return "#2fd66c";
    case "Notification":
      return "#ffaa55";
    case "Stop":
      return "#ff5a5f";
    case "SubagentStop":
      return "#b36cff";
    default:
      return "#9aa0a6";
  }
}

/// Best-effort guess of the user's home directory from a `~`-prefixed path.
/// We only use this for the "Reveal in Finder" button fallback — if the
/// backend gave us a `~` path (it doesn't right now) this turns it back
/// into something the OS can open.
function homeGuess(): string {
  return "/";
}

// ----------------------------------------------------------------------
// Create modal
// ----------------------------------------------------------------------

function CreateHookModal({
  event,
  matcher,
  command,
  onEventChange,
  onMatcherChange,
  onCommandChange,
  onCreate,
  onCancel,
}: {
  event: string;
  matcher: string;
  command: string;
  onEventChange: (v: string) => void;
  onMatcherChange: (v: string) => void;
  onCommandChange: (v: string) => void;
  onCreate: () => void;
  onCancel: () => void;
}) {
  function handleKeyDown(e: ReactKeyboardEvent<HTMLDivElement>) {
    if (e.key === "Escape") onCancel();
  }
  const active = HOOK_EVENTS.find((h) => h.name === event);
  const canSubmit = command.trim().length > 0;
  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div
        className="modal modal-wide modal-editor create-panel"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header modal-editor-header">
          <span className="create-icon">+</span>
          <span className="agent-name">Create New Hook</span>
          <span className="spacer" />
          <button className="icon" onClick={onCancel} aria-label="Close" title="Close">
            ✕
          </button>
        </div>

        <div className="modal-body modal-editor-body">
          <div className="name-row">
            <label>Event:</label>
            <select
              value={event}
              onChange={(e) => onEventChange(e.target.value)}
              className="name-input"
            >
              {HOOK_EVENTS.map((h) => (
                <option key={h.name} value={h.name}>
                  {h.name}
                </option>
              ))}
            </select>
          </div>
          {active && (
            <div
              style={{
                fontSize: 11.5,
                color: "var(--text-secondary)",
                marginTop: -6,
                marginBottom: 10,
                paddingLeft: 54,
              }}
            >
              {active.desc}
            </div>
          )}

          <div className="name-row">
            <label>Matcher:</label>
            <input
              type="text"
              placeholder="* (all tools) or e.g. Bash, Edit|Write"
              value={matcher}
              onChange={(e) => onMatcherChange(e.target.value)}
              className="name-input"
            />
          </div>

          <div className="name-row" style={{ alignItems: "flex-start" }}>
            <label style={{ marginTop: 6 }}>Command:</label>
            <textarea
              className="code-editor"
              style={{ minHeight: 140, flex: 1 }}
              placeholder="bash ./lint.sh"
              value={command}
              onChange={(e) => onCommandChange(e.target.value)}
              spellCheck={false}
              autoFocus
            />
          </div>
        </div>

        <div className="modal-footer modal-editor-footer">
          <span className="spacer" />
          <button onClick={onCancel}>Cancel</button>
          <button
            className="primary"
            disabled={!canSubmit}
            onClick={onCreate}
          >
            Create
          </button>
        </div>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------------
// Confirm delete modal
// ----------------------------------------------------------------------

function ConfirmDeleteModal({
  rule,
  onCancel,
  onConfirm,
}: {
  rule: HookRule;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div
        className="modal"
        style={{ width: 440 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <h3>Delete Hook?</h3>
        </div>
        <div className="modal-body">
          <p style={{ margin: 0, color: "var(--text-secondary)", fontSize: 12 }}>
            Permanently delete the <strong>{rule.event}</strong> hook
            {rule.matcher !== "*" && (
              <> with matcher <strong>{rule.matcher}</strong></>
            )}
            ? This can't be undone.
          </p>
        </div>
        <div className="modal-footer">
          <button onClick={onCancel}>Cancel</button>
          <button className="danger" onClick={onConfirm}>
            Delete
          </button>
        </div>
      </div>
    </div>
  );
}

// Skills view — ported from Sources/SkillsView.swift.
//
// Layout:
//   ┌──────────────────────────────────────────────┐
//   │ Skills (N)              [+ New Skill] ↻ 📁   │
//   │ Custom commands and skills for Claude Code.  │
//   │ [🔍 search skills...]                        │
//   ├──────────────────────────────────────────────┤
//   │ ● Command (n)                                │
//   │   │ ● my-cmd  "Runs a thing"  [ON]      ›   │
//   │ ● Skill (n)                                  │
//   │ ● Plugin (n)                                 │
//   └──────────────────────────────────────────────┘
//
// Editing or creating opens a large modal (EditorModal / CreateModal),
// mirroring AgentsView — skill and command markdown files can run long
// and need the room.

import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  createSkill as apiCreateSkill,
  getCommandsDir,
  getSkillsDir,
  listSkills,
  readClaudeFile,
  toggleSkill as apiToggleSkill,
  writeClaudeFile,
} from "../api";
import { useToast } from "../components/Toast";
import { displayPath } from "../lib/displayPath";
import type { SkillEntry, SkillSource } from "../types";

interface Props {
  onMutated: () => void;
}

type SourceGroup = {
  source: SkillSource;
  skills: SkillEntry[];
};

function skillTemplate(name: string, source: SkillSource): string {
  if (source === "command") {
    return `---
name: ${name}
description: A custom slash command
---

You are executing the /${name} command.

## Instructions

Describe what this command should do when invoked.
`;
  }
  return `---
name: ${name}
description: A custom skill
---

You are a specialized skill.

## Instructions

Describe what this skill does and when it should activate.
`;
}

export function SkillsView({ onMutated }: Props) {
  const toast = useToast();
  const [skills, setSkills] = useState<SkillEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [searchText, setSearchText] = useState("");
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [editingContent, setEditingContent] = useState("");
  const [status, setStatusMessage] = useState("Ready.");
  const [statusIsError, setStatusIsError] = useState(false);

  // New-skill flow
  const [showNew, setShowNew] = useState(false);
  const [newName, setNewName] = useState("");
  const [newContent, setNewContent] = useState("");
  const [newType, setNewType] = useState<SkillSource>("command");

  const setStatus = useCallback((msg: string, isError = false) => {
    setStatusMessage(msg);
    setStatusIsError(isError);
  }, []);

  const refresh = useCallback(async () => {
    try {
      const data = await listSkills();
      setSkills(data);
      setError(null);
      setSelectedPath((prev) =>
        prev && data.find((s) => s.filePath === prev) ? prev : null
      );
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const selected = useMemo(
    () => (selectedPath ? skills.find((s) => s.filePath === selectedPath) ?? null : null),
    [selectedPath, skills]
  );

  useEffect(() => {
    if (!selected) {
      setEditingContent("");
      return;
    }
    let cancelled = false;
    readClaudeFile(selected.filePath)
      .then((content) => {
        if (cancelled) return;
        setEditingContent(content);
      })
      .catch((e) => {
        if (cancelled) return;
        setEditingContent(`(Unable to read file: ${String(e)})`);
      });
    return () => {
      cancelled = true;
    };
  }, [selected]);

  const filtered = useMemo(() => {
    if (!searchText.trim()) return skills;
    const q = searchText.toLowerCase();
    return skills.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.source.toLowerCase().includes(q)
    );
  }, [skills, searchText]);

  const groups: SourceGroup[] = useMemo(() => {
    const order: SkillSource[] = ["command", "skill", "plugin"];
    const map = new Map<SkillSource, SkillEntry[]>();
    for (const s of filtered) {
      const arr = map.get(s.source) ?? [];
      arr.push(s);
      map.set(s.source, arr);
    }
    return order
      .filter((src) => (map.get(src) ?? []).length > 0)
      .map((src) => ({
        source: src,
        skills: (map.get(src) ?? [])
          .slice()
          .sort((a, b) => a.name.localeCompare(b.name)),
      }));
  }, [filtered]);

  const enabledCount = skills.filter((s) => s.isEnabled).length;
  const disabledCount = skills.length - enabledCount;

  // --- Actions ---

  function handleSelect(skill: SkillEntry) {
    setShowNew(false);
    setSelectedPath((prev) => (prev === skill.filePath ? null : skill.filePath));
  }

  async function handleSave() {
    if (!selected) return;
    try {
      await writeClaudeFile(selected.filePath, editingContent);
      setStatus(`Saved "${selected.name}".`);
      toast.show(`Saved "${selected.name}".`, "success");
      onMutated();
      await refresh();
    } catch (e) {
      setStatus(`Save error: ${String(e)}`, true);
    }
  }

  async function handleToggle(skill: SkillEntry) {
    try {
      await apiToggleSkill(skill.filePath, skill.source, skill.isEnabled);
      const msg = `${skill.isEnabled ? "Disabled" : "Enabled"} "${skill.name}".`;
      setStatus(msg);
      toast.show(msg, "success");
      // Path changes when toggled (moved to/from .disabled/), so drop selection
      setSelectedPath(null);
      onMutated();
      await refresh();
    } catch (e) {
      setStatus(String(e), true);
    }
  }

  async function handleCreate() {
    const name = newName.trim();
    if (!name) return;
    try {
      const path = await apiCreateSkill(name, newType);
      if (newContent.trim()) {
        try {
          await writeClaudeFile(path, newContent);
        } catch {
          /* ignore */
        }
      }
      setStatus(`Created "${name}".`);
      toast.show(`Created skill "${name}".`, "success");
      setShowNew(false);
      setNewName("");
      setNewContent("");
      onMutated();
      await refresh();
      setSelectedPath(path);
    } catch (e) {
      setStatus(`Create error: ${String(e)}`, true);
    }
  }

  function openNewSkillPanel() {
    setSelectedPath(null);
    setShowNew(true);
    setNewName("");
    setNewType("command");
    setNewContent(skillTemplate("my-command", "command"));
  }

  function onNewNameChange(name: string) {
    setNewName(name);
    const safe = slugify(name);
    if (safe) setNewContent(skillTemplate(safe, newType));
  }

  function onNewTypeChange(t: SkillSource) {
    setNewType(t);
    const safe = slugify(newName) || (t === "command" ? "my-command" : "my-skill");
    setNewContent(skillTemplate(safe, t));
  }

  async function openSkillsFolder() {
    // Pick the most useful target:
    //   1. If a skill is selected, reveal that file.
    //   2. Otherwise, reveal the root directory matching the current
    //      view — commands if we're viewing commands, skills if we're
    //      viewing skills. For plugin-only views we default to the
    //      commands root, since plugin skills live in a read-only
    //      marketplaces tree the user probably shouldn't poke at.
    try {
      if (selected) {
        await revealItemInDir(selected.filePath);
        return;
      }
      // No selection — default to ~/.claude/commands (matches the
      // Swift version's behavior).
      const hasSkillsGroup = groups.some((g) => g.source === "skill");
      const hasCommandsGroup = groups.some((g) => g.source === "command");
      let dir: string;
      if (hasCommandsGroup && !hasSkillsGroup) {
        dir = await getCommandsDir();
      } else if (hasSkillsGroup && !hasCommandsGroup) {
        dir = await getSkillsDir();
      } else {
        dir = await getCommandsDir();
      }
      await revealItemInDir(dir);
    } catch (e) {
      setStatus(`Reveal error: ${String(e)}`, true);
    }
  }

  const hasSearchHits = filtered.length > 0;

  return (
    <>
      <header className="main-header" data-tauri-drag-region>
        <div className="title-block" data-tauri-drag-region>
          <div className="title-row">
            <h2>Skills</h2>
            <span
              className="count-pill"
              style={{ color: "var(--amber)", background: "rgba(255, 212, 59, 0.12)" }}
            >
              {skills.length}
            </span>
          </div>
          <div className="config-path">
            Custom commands and skills for Claude Code.
          </div>
        </div>
        {/* Header action order across every view is: icon buttons on the
            left, primary CTA anchored to the far right. This keeps the
            primary action in the same pixel position page-to-page so the
            user's eye doesn't have to hunt when switching views. */}
        <div className="header-actions">
          <button className="icon" onClick={refresh} title="Reload">
            ↻
          </button>
          <button
            className="icon"
            onClick={openSkillsFolder}
            title="Reveal selected skill"
            aria-label="Reveal in Finder"
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
          <button className="gradient-btn gradient-btn--amber" onClick={openNewSkillPanel}>
            <span className="plus">+</span>
            New Skill
          </button>
        </div>
      </header>

      <div className="search-bar-wrap">
        <div className="search-bar">
          <svg
            viewBox="0 0 24 24"
            width="13"
            height="13"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-3-3" />
          </svg>
          <input
            type="text"
            placeholder="Search skills..."
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
          />
          {searchText && (
            <button
              className="clear"
              onClick={() => setSearchText("")}
              aria-label="Clear search"
            >
              ×
            </button>
          )}
        </div>
      </div>

      <div className="main-body main-body--flex">
        {error && <div className="banner error">{error}</div>}

        {skills.length === 0 ? (
          <SkillsEmptyState />
        ) : !hasSearchHits ? (
          <NoResultsState query={searchText} />
        ) : (
          <div className="groups-scroll">
            {groups.map((g) => (
              <SourceSection
                key={g.source}
                group={g}
                selectedPath={selectedPath}
                onSelect={handleSelect}
              />
            ))}
          </div>
        )}
      </div>

      <footer className="hooks-footer">
        <div className={`status-pill ${statusIsError ? "error" : ""}`}>
          <span
            className="dot"
            style={{ background: statusIsError ? "var(--red)" : "var(--green)" }}
          />
          <span>{status}</span>
        </div>
        <div className="spacer" />
        {skills.length > 0 && (
          <div className="footer-counts">
            {enabledCount} on, {disabledCount} off
          </div>
        )}
      </footer>

      {selected && (
        <EditorModal
          skill={selected}
          content={editingContent}
          onChange={setEditingContent}
          onSave={handleSave}
          onToggle={() => handleToggle(selected)}
          onClose={() => setSelectedPath(null)}
        />
      )}

      {showNew && !selected && (
        <CreateModal
          name={newName}
          type={newType}
          content={newContent}
          onNameChange={onNewNameChange}
          onTypeChange={onNewTypeChange}
          onContentChange={setNewContent}
          onCreate={handleCreate}
          onCancel={() => {
            setShowNew(false);
            setNewName("");
            setNewContent("");
          }}
        />
      )}
    </>
  );
}

// ----------------------------------------------------------------------
// Section
// ----------------------------------------------------------------------

function SourceSection({
  group,
  selectedPath,
  onSelect,
}: {
  group: SourceGroup;
  selectedPath: string | null;
  onSelect: (s: SkillEntry) => void;
}) {
  const color = sourceColor(group.source);
  return (
    <div className="agent-group">
      <div className="agent-group-header" style={{ background: `${color}0a`, borderColor: `${color}1a` }}>
        <span className="glow-dot" style={{ background: color, boxShadow: `0 0 6px ${color}` }} />
        <SourceIcon source={group.source} color={color} />
        <span className="group-title" style={{ textTransform: "capitalize" }}>
          {group.source}
        </span>
        <span
          className="count-mini-pill"
          style={{ color, background: `${color}1a` }}
        >
          {group.skills.length}
        </span>
        <span className="spacer" />
      </div>
      <div className="agent-group-body">
        <div className="thread-line" style={{ background: `${color}33` }} />
        <div className="agent-cards">
          {group.skills.map((s) => (
            <SkillCard
              key={s.filePath}
              skill={s}
              isSelected={selectedPath === s.filePath}
              onClick={() => onSelect(s)}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function SkillCard({
  skill,
  isSelected,
  onClick,
}: {
  skill: SkillEntry;
  isSelected: boolean;
  onClick: () => void;
}) {
  const color = sourceColor(skill.source);
  const dotColor = skill.isEnabled ? color : "var(--text-quaternary)";
  return (
    <div
      className={`agent-card ${isSelected ? "selected" : ""} ${
        skill.isEnabled ? "" : "off"
      }`}
      onClick={onClick}
      style={
        isSelected
          ? { borderColor: `${color}55`, background: `${color}14` }
          : undefined
      }
    >
      <span
        className="glow-dot"
        style={{
          background: dotColor,
          boxShadow: skill.isEnabled ? `0 0 4px ${color}` : "none",
        }}
      />
      <span className="agent-name">{skill.name}</span>
      {skill.description && (
        <span className="agent-desc">{skill.description}</span>
      )}
      <span className="spacer" />
      {skill.source !== "plugin" ? (
        <span
          className="state-pill"
          style={{
            color: skill.isEnabled ? "var(--green)" : "var(--red)",
            background: skill.isEnabled
              ? "rgba(0, 245, 160, 0.12)"
              : "rgba(255, 92, 138, 0.12)",
          }}
        >
          {skill.isEnabled ? "ON" : "OFF"}
        </span>
      ) : !skill.isEnabled ? (
        <span className="plugin-off-tag">plugin off</span>
      ) : null}
      <span className="chevron">{isSelected ? "⌄" : "›"}</span>
    </div>
  );
}

// ----------------------------------------------------------------------
// Editor modal
// ----------------------------------------------------------------------

function EditorModal({
  skill,
  content,
  onChange,
  onSave,
  onToggle,
  onClose,
}: {
  skill: SkillEntry;
  content: string;
  onChange: (v: string) => void;
  onSave: () => void;
  onToggle: () => void;
  onClose: () => void;
}) {
  const color = sourceColor(skill.source);

  function handleKeyDown(e: ReactKeyboardEvent<HTMLDivElement>) {
    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      onSave();
    } else if (e.key === "Escape") {
      onClose();
    }
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div
        className="modal modal-wide modal-editor"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header modal-editor-header">
          <span
            className="glow-dot"
            style={{
              background: skill.isEnabled ? color : "var(--text-quaternary)",
              boxShadow: skill.isEnabled ? `0 0 6px ${color}` : "none",
            }}
          />
          <span className="agent-name">{skill.name}</span>
          <span
            className="source-pill"
            style={{ color, background: `${color}1a`, textTransform: "capitalize" }}
          >
            {skill.source}
          </span>
          {skill.source === "command" && (
            <span className="slash-cmd">/{skill.name}</span>
          )}
          <span className="spacer" />
          <button
            className="icon"
            onClick={async () => {
              try {
                await revealItemInDir(skill.filePath);
              } catch {
                /* ignore */
              }
            }}
            title="Show in Finder"
            aria-label="Show in Finder"
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
          <button className="icon" onClick={onClose} aria-label="Close" title="Close">
            ✕
          </button>
        </div>

        <div className="modal-body modal-editor-body">
          <textarea
            className="code-editor code-editor--large"
            value={content}
            onChange={(e) => onChange(e.target.value)}
            spellCheck={false}
            autoFocus
          />
        </div>

        <div className="modal-footer modal-editor-footer">
          {skill.source !== "plugin" && (
            <button
              className={skill.isEnabled ? "danger" : ""}
              onClick={onToggle}
              style={skill.isEnabled ? undefined : { color: "var(--green)" }}
            >
              {skill.isEnabled ? "Disable" : "Enable"}
            </button>
          )}
          <span className="path-hint">{displayPath(skill.filePath)}</span>
          <span className="spacer" />
          <button onClick={onClose}>Cancel</button>
          <button className="primary" onClick={onSave}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------------
// Create modal
// ----------------------------------------------------------------------

function CreateModal({
  name,
  type,
  content,
  onNameChange,
  onTypeChange,
  onContentChange,
  onCreate,
  onCancel,
}: {
  name: string;
  type: SkillSource;
  content: string;
  onNameChange: (v: string) => void;
  onTypeChange: (t: SkillSource) => void;
  onContentChange: (v: string) => void;
  onCreate: () => void;
  onCancel: () => void;
}) {
  function handleKeyDown(e: ReactKeyboardEvent<HTMLDivElement>) {
    if (e.key === "Escape") onCancel();
  }
  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div
        className="modal modal-wide modal-editor create-panel"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        <div className="modal-header modal-editor-header">
          <span className="create-icon">+</span>
          <span className="agent-name">Create New Skill</span>
          <span className="spacer" />
          <button className="icon" onClick={onCancel} aria-label="Close" title="Close">
            ✕
          </button>
        </div>

        <div className="modal-body modal-editor-body">
          <div className="name-row">
            <div className="segmented">
              <button
                className={type === "command" ? "active" : ""}
                onClick={() => onTypeChange("command")}
              >
                Command
              </button>
              <button
                className={type === "skill" ? "active" : ""}
                onClick={() => onTypeChange("skill")}
              >
                Skill
              </button>
            </div>
            <label>Name:</label>
            <input
              type="text"
              placeholder={type === "command" ? "e.g. my-command" : "e.g. my-skill"}
              value={name}
              onChange={(e) => onNameChange(e.target.value)}
              className="name-input"
              autoFocus
            />
          </div>

          <textarea
            className="code-editor code-editor--large"
            value={content}
            onChange={(e) => onContentChange(e.target.value)}
            spellCheck={false}
          />
        </div>

        <div className="modal-footer modal-editor-footer">
          <span className="spacer" />
          <button onClick={onCancel}>Cancel</button>
          <button
            className="primary"
            disabled={!name.trim()}
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
// Empty states
// ----------------------------------------------------------------------

function SkillsEmptyState() {
  const rows = [
    {
      name: "Custom Commands",
      desc: "Slash commands in ~/.claude/commands/",
      color: "var(--amber)",
    },
    {
      name: "Custom Skills",
      desc: "Skill definitions in ~/.claude/skills/",
      color: "var(--green)",
    },
    {
      name: "Plugin Skills",
      desc: "Skills from enabled Claude Code plugins",
      color: "var(--purple)",
    },
  ];
  return (
    <div className="empty-wrap">
      <div className="empty-glow empty-glow--amber">
        <svg
          viewBox="0 0 24 24"
          width="34"
          height="34"
          fill="currentColor"
        >
          <path d="M12 2l2.39 7.36H22l-6.18 4.49L18.2 21 12 16.51 5.8 21l2.38-7.15L2 9.36h7.61z" />
        </svg>
      </div>
      <h3>No Skills Found</h3>
      <p className="empty-blurb">
        Skills are custom commands and extensions for Claude Code. Create .md
        files in ~/.claude/commands/ or ~/.claude/skills/, or use the New
        Skill button above.
      </p>
      <div className="empty-card">
        <div className="empty-card-label">SKILL SOURCES</div>
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

function NoResultsState({ query }: { query: string }) {
  return (
    <div className="empty-wrap">
      <div className="empty-glow empty-glow--muted">
        <svg
          viewBox="0 0 24 24"
          width="30"
          height="30"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.75"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <circle cx="11" cy="11" r="7" />
          <path d="m20 20-3-3" />
        </svg>
      </div>
      <p className="empty-blurb">No skills match "{query}"</p>
    </div>
  );
}

// ----------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------

function sourceColor(source: SkillSource): string {
  switch (source) {
    case "command":
      return "#ffd43b";
    case "skill":
      return "#00f5a0";
    case "plugin":
      return "#b36cff";
  }
}

function SourceIcon({ source, color }: { source: SkillSource; color: string }) {
  const common = {
    width: 13,
    height: 13,
    fill: "none" as const,
    stroke: "currentColor",
    strokeWidth: 1.75,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    style: { color },
  };
  if (source === "command") {
    return (
      <svg viewBox="0 0 24 24" {...common}>
        <path d="m4 17 6-6-6-6" />
        <path d="M12 19h8" />
      </svg>
    );
  }
  if (source === "skill") {
    return (
      <svg viewBox="0 0 24 24" {...common} fill="currentColor" stroke="none">
        <path d="M12 2l2.39 7.36H22l-6.18 4.49L18.2 21 12 16.51 5.8 21l2.38-7.15L2 9.36h7.61z" />
      </svg>
    );
  }
  // plugin
  return (
    <svg viewBox="0 0 24 24" {...common}>
      <path d="M10 3v4a2 2 0 0 1-2 2H4v4h4a2 2 0 0 1 2 2v4h4v-4a2 2 0 0 1 2-2h4v-4h-4a2 2 0 0 1-2-2V3z" />
    </svg>
  );
}

function slugify(name: string): string {
  return name
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^a-z0-9-]/g, "");
}


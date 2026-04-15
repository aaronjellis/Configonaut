// Agents view — ported from Sources/AgentsView.swift.
//
// Layout:
//   ┌──────────────────────────────────────────────┐
//   │ Agents (N)              [+ New Agent] ↻      │
//   │ Personal and plugin agents for Claude Code.  │
//   │ [🔍 search agents...]                        │
//   ├──────────────────────────────────────────────┤
//   │ ● Personal (n)              📁               │
//   │   │ ● my-agent  sonnet  "Review PRs"     ›  │
//   │ ● plugin-name  [enabled/disabled]  [Toggle]  │
//   │   │ ● other-agent  ...                     ›│
//   └──────────────────────────────────────────────┘
//
// Editing an agent or creating a new one opens a large modal (see
// EditorModal / CreateModal below) rather than a cramped bottom panel —
// agent files routinely run hundreds of lines and need the room.

import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  createAgent as apiCreateAgent,
  deleteAgent as apiDeleteAgent,
  listAgents,
  readClaudeFile,
  togglePlugin,
  writeClaudeFile,
} from "../api";
import { ModeBanner } from "../components/ModeBanner";
import { useToast } from "../components/Toast";
import { displayPath } from "../lib/displayPath";
import type { AgentEntry, AppMode } from "../types";

interface Props {
  // Used purely to decide whether to show the ModeBanner. The underlying
  // files don't change per mode (they're always `~/.claude/agents/`).
  mode: AppMode;
  onMutated: () => void;
}

type PluginGroup = {
  plugin: string;
  agents: AgentEntry[];
  isEnabled: boolean;
};

const AGENT_TEMPLATE = (name: string) => `---
name: ${name}
description: A custom agent
tools: Read, Edit, Write, Bash, Glob, Grep
model: sonnet
color: blue
---

You are a specialized agent. Describe your role and capabilities here.

## Instructions

- What should this agent do?
- What tools should it use and when?
- What rules should it follow?
`;

export function AgentsView({ mode, onMutated }: Props) {
  const toast = useToast();
  const [agents, setAgents] = useState<AgentEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [searchText, setSearchText] = useState("");
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [editingContent, setEditingContent] = useState("");
  const [status, setStatusMessage] = useState("Ready.");
  const [statusIsError, setStatusIsError] = useState(false);

  // New-agent flow
  const [showNew, setShowNew] = useState(false);
  const [newName, setNewName] = useState("");
  const [newContent, setNewContent] = useState("");

  // Delete confirmation
  const [confirmDelete, setConfirmDelete] = useState<AgentEntry | null>(null);

  const setStatus = useCallback((msg: string, isError = false) => {
    setStatusMessage(msg);
    setStatusIsError(isError);
  }, []);

  const refresh = useCallback(async () => {
    try {
      const data = await listAgents();
      setAgents(data);
      setError(null);
      // Drop selection if it disappeared
      setSelectedPath((prev) =>
        prev && data.find((a) => a.filePath === prev) ? prev : null
      );
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const selected = useMemo(
    () => (selectedPath ? agents.find((a) => a.filePath === selectedPath) ?? null : null),
    [selectedPath, agents]
  );

  // Load content when selection changes
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

  // Filtered + grouped agents
  const filtered = useMemo(() => {
    if (!searchText.trim()) return agents;
    const q = searchText.toLowerCase();
    return agents.filter(
      (a) =>
        a.name.toLowerCase().includes(q) ||
        a.description.toLowerCase().includes(q) ||
        a.pluginName.toLowerCase().includes(q)
    );
  }, [agents, searchText]);

  const personalAgents = useMemo(
    () =>
      filtered
        .filter((a) => a.source === "personal")
        .sort((a, b) => a.name.localeCompare(b.name)),
    [filtered]
  );

  const pluginGroups: PluginGroup[] = useMemo(() => {
    const plugins = filtered.filter((a) => a.source === "plugin");
    const map = new Map<string, AgentEntry[]>();
    for (const a of plugins) {
      const arr = map.get(a.pluginName) ?? [];
      arr.push(a);
      map.set(a.pluginName, arr);
    }
    return Array.from(map.keys())
      .sort()
      .map((plugin) => {
        const list = (map.get(plugin) ?? []).slice().sort((a, b) =>
          a.name.localeCompare(b.name)
        );
        return {
          plugin,
          agents: list,
          // Each agent row knows its own plugin-enabled state
          isEnabled: list[0]?.isPluginEnabled ?? false,
        };
      });
  }, [filtered]);

  const personalCount = agents.filter((a) => a.source === "personal").length;
  const enabledPluginCount = useMemo(() => {
    const set = new Set<string>();
    for (const a of agents) {
      if (a.source === "plugin" && a.isPluginEnabled) set.add(a.pluginName);
    }
    return set.size;
  }, [agents]);

  // --- Actions ---

  function handleSelect(agent: AgentEntry) {
    setShowNew(false);
    setSelectedPath((prev) => (prev === agent.filePath ? null : agent.filePath));
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

  async function handleDelete(agent: AgentEntry) {
    try {
      await apiDeleteAgent(agent.filePath);
      setStatus(`Deleted "${agent.name}".`);
      toast.show(`Deleted "${agent.name}".`, "success");
      if (selectedPath === agent.filePath) setSelectedPath(null);
      setConfirmDelete(null);
      onMutated();
      await refresh();
    } catch (e) {
      setStatus(`Delete error: ${String(e)}`, true);
    }
  }

  async function handleTogglePlugin(pluginName: string) {
    try {
      const key = `${pluginName}@claude-plugins-official`;
      await togglePlugin(key);
      setStatus(`Toggled plugin "${pluginName}".`);
      toast.show(`Toggled plugin "${pluginName}".`, "success");
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
      const path = await apiCreateAgent(name);
      // Overwrite with user-edited content if they customized the template
      if (newContent.trim()) {
        try {
          await writeClaudeFile(path, newContent);
        } catch {
          /* ignore — file was created at least */
        }
      }
      setStatus(`Created "${name}".`);
      toast.show(`Created agent "${name}".`, "success");
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

  function openNewAgentPanel() {
    setSelectedPath(null);
    setShowNew(true);
    setNewName("");
    setNewContent(AGENT_TEMPLATE("my-agent"));
  }

  function onNewNameChange(name: string) {
    setNewName(name);
    const safe = slugify(name);
    if (safe) setNewContent(AGENT_TEMPLATE(safe));
  }

  // --- Render ---

  const hasSearchHits = filtered.length > 0;

  return (
    <>
      <header className="main-header" data-tauri-drag-region>
        <div className="title-block" data-tauri-drag-region>
          <div className="title-row">
            <h2>Agents</h2>
            <span
              className="count-pill"
              style={{ color: "var(--purple)", background: "rgba(179, 108, 255, 0.12)" }}
            >
              {agents.length}
            </span>
          </div>
          <div className="section-description">
            Specialized sub-agents Claude can delegate to. Each one carries
            its own system prompt and tool permissions for a focused job
            like code review, triage, or release notes.
          </div>
        </div>
        {/* Icons on the left, primary CTA anchored to the far right —
            keeps the "+ New X" button in the same pixel position across
            every view. */}
        <div className="header-actions">
          <button className="icon" onClick={refresh} title="Reload">
            ↻
          </button>
          <button className="gradient-btn gradient-btn--blue" onClick={openNewAgentPanel}>
            <span className="plus">+</span>
            New Agent
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
            placeholder="Search agents..."
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
        <ModeBanner view="agents" mode={mode} />
        {error && <div className="banner error">{error}</div>}

        {agents.length === 0 ? (
          <AgentsEmptyState />
        ) : !hasSearchHits ? (
          <NoResultsState query={searchText} />
        ) : (
          <div className="groups-scroll">
            {personalAgents.length > 0 && (
              <PersonalSection
                agents={personalAgents}
                selectedPath={selectedPath}
                onSelect={handleSelect}
              />
            )}
            {pluginGroups.map((g) => (
              <PluginSection
                key={g.plugin}
                group={g}
                selectedPath={selectedPath}
                onSelect={handleSelect}
                onTogglePlugin={() => handleTogglePlugin(g.plugin)}
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
        <div className="footer-counts">
          {personalCount} personal, {enabledPluginCount} plugins
        </div>
      </footer>

      {selected && (
        <EditorModal
          agent={selected}
          content={editingContent}
          onChange={setEditingContent}
          onSave={handleSave}
          onClose={() => setSelectedPath(null)}
          onDelete={() => setConfirmDelete(selected)}
          onTogglePlugin={() => handleTogglePlugin(selected.pluginName)}
        />
      )}

      {showNew && !selected && (
        <CreateModal
          name={newName}
          content={newContent}
          onNameChange={onNewNameChange}
          onContentChange={setNewContent}
          onCreate={handleCreate}
          onCancel={() => {
            setShowNew(false);
            setNewName("");
            setNewContent("");
          }}
        />
      )}

      {confirmDelete && (
        <ConfirmDeleteModal
          agent={confirmDelete}
          onCancel={() => setConfirmDelete(null)}
          onConfirm={() => handleDelete(confirmDelete)}
        />
      )}
    </>
  );
}

// ----------------------------------------------------------------------
// Sections
// ----------------------------------------------------------------------

function PersonalSection({
  agents,
  selectedPath,
  onSelect,
}: {
  agents: AgentEntry[];
  selectedPath: string | null;
  onSelect: (a: AgentEntry) => void;
}) {
  return (
    <div className="agent-group">
      <div className="agent-group-header personal">
        <span className="glow-dot" style={{ background: "var(--blue)", boxShadow: "0 0 6px var(--blue)" }} />
        <svg
          viewBox="0 0 24 24"
          width="13"
          height="13"
          fill="currentColor"
          style={{ color: "var(--blue)" }}
        >
          <circle cx="12" cy="8" r="4" />
          <path d="M4 21a8 8 0 0 1 16 0" />
        </svg>
        <span className="group-title">Personal</span>
        <span
          className="count-mini-pill"
          style={{ color: "var(--blue)", background: "rgba(0, 180, 255, 0.1)" }}
        >
          {agents.length}
        </span>
        <span className="spacer" />
      </div>
      <div className="agent-group-body">
        <div className="thread-line" style={{ background: "rgba(0, 180, 255, 0.2)" }} />
        <div className="agent-cards">
          {agents.map((a) => (
            <AgentCard
              key={a.filePath}
              agent={a}
              isSelected={selectedPath === a.filePath}
              pluginOn
              onClick={() => onSelect(a)}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function PluginSection({
  group,
  selectedPath,
  onSelect,
  onTogglePlugin,
}: {
  group: PluginGroup;
  selectedPath: string | null;
  onSelect: (a: AgentEntry) => void;
  onTogglePlugin: () => void;
}) {
  const statusColor = group.isEnabled ? "var(--green)" : "var(--red)";
  return (
    <div className="agent-group">
      <div className="agent-group-header plugin">
        <span
          className="glow-dot"
          style={{ background: statusColor, boxShadow: `0 0 6px ${statusColor}` }}
        />
        <span className="group-title">{group.plugin}</span>
        <span
          className="count-mini-pill"
          style={{
            color: statusColor,
            background: group.isEnabled
              ? "rgba(0, 245, 160, 0.1)"
              : "rgba(255, 92, 138, 0.1)",
          }}
        >
          {group.isEnabled ? "enabled" : "disabled"}
        </span>
        <span className="spacer" />
        <button
          className={`mini-btn ${group.isEnabled ? "danger" : "success"}`}
          onClick={onTogglePlugin}
        >
          {group.isEnabled ? "Disable" : "Enable"}
        </button>
      </div>
      <div className="agent-group-body">
        <div
          className="thread-line"
          style={{
            background: group.isEnabled
              ? "rgba(179, 108, 255, 0.2)"
              : "rgba(255, 92, 138, 0.12)",
          }}
        />
        <div className="agent-cards">
          {group.agents.map((a) => (
            <AgentCard
              key={a.filePath}
              agent={a}
              isSelected={selectedPath === a.filePath}
              pluginOn={group.isEnabled}
              onClick={() => onSelect(a)}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function AgentCard({
  agent,
  isSelected,
  pluginOn,
  onClick,
}: {
  agent: AgentEntry;
  isSelected: boolean;
  pluginOn: boolean;
  onClick: () => void;
}) {
  const color = agentColor(agent.color);
  return (
    <div
      className={`agent-card ${isSelected ? "selected" : ""}`}
      onClick={onClick}
      style={
        isSelected
          ? { borderColor: `${color}55`, background: `${color}14` }
          : undefined
      }
    >
      <span
        className="glow-dot"
        style={{ background: color, boxShadow: `0 0 4px ${color}` }}
      />
      <span className="agent-name">{agent.name}</span>
      {agent.model && <span className="agent-model">{agent.model}</span>}
      {agent.description && (
        <span className="agent-desc">{agent.description}</span>
      )}
      <span className="spacer" />
      {!pluginOn && <span className="plugin-off-tag">plugin off</span>}
      <span className="chevron">{isSelected ? "⌄" : "›"}</span>
    </div>
  );
}

// ----------------------------------------------------------------------
// Editor modal
// ----------------------------------------------------------------------

function EditorModal({
  agent,
  content,
  onChange,
  onSave,
  onClose,
  onDelete,
  onTogglePlugin,
}: {
  agent: AgentEntry;
  content: string;
  onChange: (v: string) => void;
  onSave: () => void;
  onClose: () => void;
  onDelete: () => void;
  onTogglePlugin: () => void;
}) {
  const color = agentColor(agent.color);
  const isPersonal = agent.source === "personal";

  // Cmd/Ctrl-S saves from anywhere inside the modal.
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
            style={{ background: color, boxShadow: `0 0 6px ${color}` }}
          />
          <span className="agent-name">{agent.name}</span>
          {agent.model && <span className="agent-model">{agent.model}</span>}
          <span
            className="source-pill"
            style={{
              color: isPersonal ? "var(--blue)" : "var(--purple)",
              background: isPersonal
                ? "rgba(0, 180, 255, 0.1)"
                : "rgba(179, 108, 255, 0.1)",
            }}
          >
            {isPersonal ? "Personal" : agent.pluginName}
          </span>
          <span className="spacer" />
          <button
            className="icon"
            onClick={async () => {
              try {
                await revealItemInDir(agent.filePath);
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
          {isPersonal && (
            <button className="danger" onClick={onDelete}>
              Delete
            </button>
          )}
          {agent.source === "plugin" && (
            <button
              className={agent.isPluginEnabled ? "danger" : ""}
              onClick={onTogglePlugin}
              style={agent.isPluginEnabled ? undefined : { color: "var(--green)" }}
            >
              {agent.isPluginEnabled ? "Disable Plugin" : "Enable Plugin"}
            </button>
          )}
          <span className="path-hint">{displayPath(agent.filePath)}</span>
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
  content,
  onNameChange,
  onContentChange,
  onCreate,
  onCancel,
}: {
  name: string;
  content: string;
  onNameChange: (v: string) => void;
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
          <span className="agent-name">Create New Agent</span>
          <span className="spacer" />
          <button className="icon" onClick={onCancel} aria-label="Close" title="Close">
            ✕
          </button>
        </div>

        <div className="modal-body modal-editor-body">
          <div className="name-row">
            <label>Name:</label>
            <input
              type="text"
              placeholder="e.g. my-reviewer"
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

function AgentsEmptyState() {
  return (
    <div className="empty-wrap">
      <div className="empty-glow empty-glow--purple">
        <svg
          viewBox="0 0 24 24"
          width="34"
          height="34"
          fill="currentColor"
        >
          <circle cx="9" cy="8" r="3.5" />
          <circle cx="17" cy="9" r="2.5" />
          <path d="M2 20a7 7 0 0 1 14 0z" />
          <path d="M14 20a6 6 0 0 1 10-4" />
        </svg>
      </div>
      <h3>No Agents Found</h3>
      <p className="empty-blurb">
        Agents come from plugins installed via Claude Code. Enable plugins in
        your Claude Code settings to see their agents here, or create your own
        with the New Agent button above.
      </p>
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
      <p className="empty-blurb">No agents match "{query}"</p>
    </div>
  );
}

// ----------------------------------------------------------------------
// Confirm delete modal
// ----------------------------------------------------------------------

function ConfirmDeleteModal({
  agent,
  onCancel,
  onConfirm,
}: {
  agent: AgentEntry;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div
        className="modal"
        style={{ width: 420 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <h3>Delete Agent?</h3>
        </div>
        <div className="modal-body">
          <p style={{ margin: 0, color: "var(--text-secondary)", fontSize: 12 }}>
            Permanently delete "<strong>{agent.name}</strong>"? This can't be
            undone.
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

// ----------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------

function agentColor(color: string): string {
  switch ((color ?? "").toLowerCase()) {
    case "red":
      return "#ff5c8a";
    case "green":
      return "#00f5a0";
    case "blue":
      return "#00b4ff";
    case "purple":
      return "#b36cff";
    case "orange":
      return "#ff8a3d";
    case "cyan":
      return "#00e5ff";
    case "amber":
    case "yellow":
      return "#ffd43b";
    default:
      return "#b36cff";
  }
}

function slugify(name: string): string {
  return name
    .toLowerCase()
    .replace(/\s+/g, "-")
    .replace(/[^a-z0-9-]/g, "");
}


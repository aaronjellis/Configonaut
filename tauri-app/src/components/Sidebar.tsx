// Sidebar — matches the structure in Sources/ContentView.swift:
//   • App icon + "Configonaut" + mode caption
//   • Segmented Desktop / CLI picker
//   • "CONFIGURE" section: MCP Servers, Hooks, Agents, Skills
//   • Gradient divider
//   • Backups (pinned at the bottom)
//   • Version footer
//
// Each nav item gets its own accent color and an optional badge count.
// Marketplace is intentionally NOT a sidebar item — in the Swift version
// it's a tab inside the Add Server flow.

import type { AppMode, ViewKey } from "../types";

interface NavItemDef {
  key: ViewKey;
  title: string;
  subtitle: string;
  glyph: string; // Unicode stand-in for SF Symbols
  accent: string; // CSS color (matches Theme.swift palette)
}

// Single CONFIGURE section — previously split into TOOLS (MCP, Hooks) and
// EXTEND (Agents, Skills), but the distinction wasn't load-bearing and
// just added visual clutter.
const CONFIGURE: NavItemDef[] = [
  {
    key: "mcp",
    title: "MCP Servers",
    subtitle: "Add, remove & swap tools",
    glyph: "▤",
    accent: "var(--green)",
  },
  {
    key: "hooks",
    title: "Hooks",
    subtitle: "Automation triggers",
    glyph: "⤴",
    accent: "var(--blue)",
  },
  {
    key: "agents",
    title: "Agents",
    subtitle: "Plugin agent configs",
    glyph: "☰",
    accent: "var(--purple)",
  },
  {
    key: "skills",
    title: "Skills",
    subtitle: "Commands & slash skills",
    glyph: "★",
    accent: "var(--amber)",
  },
];

const BACKUPS: NavItemDef = {
  key: "backups",
  title: "Backups",
  subtitle: "Config history & restore",
  glyph: "↻",
  accent: "var(--cyan)",
};

interface Props {
  currentView: ViewKey;
  onViewChange: (v: ViewKey) => void;
  mode: AppMode;
  onModeChange: (m: AppMode) => void;
  badges: Partial<Record<ViewKey, number>>;
  version: string;
}

export function Sidebar({
  currentView,
  onViewChange,
  mode,
  onModeChange,
  badges,
  version,
}: Props) {
  const renderItem = (item: NavItemDef) => {
    const isActive = currentView === item.key;
    const count = badges[item.key] ?? 0;
    return (
      <button
        key={item.key}
        className={`nav-item ${isActive ? "active" : ""}`}
        style={{ ["--nav-accent" as string]: item.accent }}
        onClick={() => onViewChange(item.key)}
      >
        <span className="glyph">{item.glyph}</span>
        <span className="labels">
          <span className="title">{item.title}</span>
          <span className="subtitle">{item.subtitle}</span>
        </span>
        {count > 0 && <span className="badge">{count}</span>}
      </button>
    );
  };

  return (
    <aside className="sidebar" data-tauri-drag-region>
      <div className="sidebar-brand" data-tauri-drag-region>
        <div className="icon-wrap" data-tauri-drag-region>
          <img src="/icon.png" alt="" draggable={false} />
        </div>
        <div className="brand-text" data-tauri-drag-region>
          <div className="title" data-tauri-drag-region>
            Configonaut
          </div>
          <div className={`mode-caption ${mode}`} data-tauri-drag-region>
            {mode === "desktop" ? "Desktop Config" : "CLI Config"}
          </div>
        </div>
      </div>

      <div className="mode-picker" role="tablist" aria-label="Claude target">
        <button
          role="tab"
          aria-selected={mode === "desktop"}
          className={mode === "desktop" ? "active" : ""}
          onClick={() => onModeChange("desktop")}
        >
          Desktop
        </button>
        <button
          role="tab"
          aria-selected={mode === "cli"}
          className={mode === "cli" ? "active" : ""}
          onClick={() => onModeChange("cli")}
        >
          CLI
        </button>
      </div>

      <div className="section-label">CONFIGURE</div>
      {CONFIGURE.map(renderItem)}

      <div className="sidebar-spacer" />

      <div className="sidebar-divider" />
      {renderItem(BACKUPS)}

      <div className="sidebar-version">v{version}</div>
    </aside>
  );
}

# Configonaut: Desktop/CLI Toggle & Windows Port

**Date:** 2026-04-09
**Status:** Approved design

## Overview

Two changes to Configonaut:

1. **Desktop/CLI toggle** — a global app-level switch that changes which config files the app reads/writes (Claude Desktop vs Claude Code). Applied to the existing macOS app and the new Windows app.
2. **Windows port** — a dedicated WinUI 3 / C# app in a separate repository (`Configonaut-Windows`) with feature parity to the macOS version.

---

## 1. Desktop/CLI Toggle

### Placement

Segmented control in the sidebar header, above navigation items. Persisted across launches via UserDefaults (macOS) / ApplicationData (Windows).

```
┌─────────────────────┐
│  ◈ Configonaut      │
│  [Desktop] [CLI]    │
│─────────────────────│
│  ● MCP Servers      │
│  ● Hooks            │
│  ● Agents           │
│  ● Skills           │
│  ● Backups          │
└─────────────────────┘
```

### Mode behavior

| View | Desktop Mode | CLI Mode |
|------|-------------|----------|
| MCP Servers | Reads/writes `claude_desktop_config.json` | Reads/writes `~/.claude/settings.json` → `mcpServers` |
| Hooks | Visible but disabled — shows "Not available for Claude Desktop" placeholder | Reads/writes `~/.claude/settings.json` → `hooks` |
| Agents | Visible but disabled — shows "Not available for Claude Desktop" placeholder | Scans `~/.claude/agents/` + plugins |
| Skills | Visible but disabled — shows "Not available for Claude Desktop" placeholder | Scans `~/.claude/commands/` + `~/.claude/skills/` |
| Backups | Backs up `claude_desktop_config.json` | Backs up `~/.claude/settings.json` |

### ConfigManager changes (macOS)

- New `AppMode` enum: `.desktop`, `.cli`
- `@Published var mode: AppMode` persisted to `UserDefaults`
- Config paths become computed properties that switch on `mode`
- Stored (inactive) servers separated per mode: `stored_servers_desktop.json` / `stored_servers_cli.json`
- Backups separated per mode: `backups/desktop/` and `backups/cli/`

### Migration (macOS)

Existing stored servers and backups are assumed to be Desktop (since that's what the app reads today). On first launch after update:
- Move `stored_servers.json` → `stored_servers_desktop.json`
- Move `backups/*.json` → `backups/desktop/*.json`
- Create empty `backups/cli/` directory

No data loss. Migration runs once, gated by a `migrated_v2` flag in UserDefaults.

---

## 2. Windows Port

### Repository

Separate repo: `Configonaut-Windows`. C# / WinUI 3 / .NET. Feature parity with macOS app.

### Project structure

```
Configonaut-Windows/
├── Configonaut/
│   ├── App.xaml(.cs)              # Entry point, theme setup
│   ├── MainWindow.xaml(.cs)       # Shell: sidebar + content frame
│   ├── Models/
│   │   ├── AppMode.cs             # Desktop/CLI enum
│   │   ├── ServerEntry.cs         # MCP server model
│   │   ├── HookRule.cs            # Hook model
│   │   ├── AgentEntry.cs          # Agent model
│   │   └── SkillEntry.cs          # Skill/command model
│   ├── ViewModels/
│   │   ├── ConfigManager.cs       # Core config I/O (port of Swift ConfigManager)
│   │   ├── MCPViewModel.cs        # MCP servers state
│   │   ├── HooksViewModel.cs      # Hooks state
│   │   ├── AgentsViewModel.cs     # Agents state
│   │   ├── SkillsViewModel.cs     # Skills state
│   │   └── BackupsViewModel.cs    # Backups state
│   ├── Views/
│   │   ├── MCPPage.xaml(.cs)
│   │   ├── HooksPage.xaml(.cs)
│   │   ├── AgentsPage.xaml(.cs)
│   │   ├── SkillsPage.xaml(.cs)
│   │   └── BackupsPage.xaml(.cs)
│   ├── Helpers/
│   │   ├── PathResolver.cs        # Platform config path logic + MSIX detection
│   │   └── JsonHelper.cs          # JSON read/write utilities
│   └── Theme/
│       └── NeonTheme.xaml         # Resource dictionary: colors, styles
├── Configonaut.sln
└── README.md
```

### Pattern

MVVM — `ConfigManager` as the central state object with `INotifyPropertyChanged` / `ObservableCollection<T>`. Views bind to ViewModels. Mirrors the macOS architecture where `ConfigManager` is `@StateObject` and views are `@ObservedObject`.

### Config path resolution

`PathResolver.cs` resolves all paths and detects MSIX vs standard installs.

| Config | macOS | Windows (standard) | Windows (MSIX) |
|--------|-------|--------------------|----------------|
| Desktop MCP | `~/Library/Application Support/Claude/claude_desktop_config.json` | `%APPDATA%\Claude\claude_desktop_config.json` | `%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\Claude\claude_desktop_config.json` |
| CLI settings | `~/.claude/settings.json` | `%USERPROFILE%\.claude\settings.json` | `%USERPROFILE%\.claude\settings.json` |
| CLI agents | `~/.claude/agents/` | `%USERPROFILE%\.claude\agents\` | same |
| CLI commands | `~/.claude/commands/` | `%USERPROFILE%\.claude\commands\` | same |
| CLI skills | `~/.claude/skills/` | `%USERPROFILE%\.claude\skills\` | same |
| CLI plugins | `~/.claude/plugins/marketplaces/...` | `%USERPROFILE%\.claude\plugins\marketplaces\...` | same |
| App storage | `~/Library/Application Support/Configonaut/` | `%APPDATA%\Configonaut\` | same |

MSIX detection: check if `%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\Claude\claude_desktop_config.json` exists (not just the directory — the directory can exist without a config file). If both standard and MSIX config files exist, prefer MSIX (that's what the app actually reads). Show an info banner if both are detected.

### Theme

WinUI 3 resource dictionary:
- Dark base: `#0A0E15`
- Neon accents: Green `#00F5A0`, Red `#FF5C8A`, Blue `#00B4FF`, Purple `#B36CFF`, Amber `#FFD43B`, Cyan `#00E5FF`, Orange `#FF8A3D`
- `AcrylicBrush` on cards for glass effect (built-in WinUI, no hacks)
- Neon glow via `DropShadow` where simple; skip where it'd be forced
- Match the aesthetic, don't fight the framework

### Packaging

- MSIX installer (self-signed initially, proper code signing certificate later)
- Optional portable `.exe` for users who prefer it

---

## 3. Feature Parity

| Feature | macOS (SwiftUI) | Windows (WinUI 3) |
|---------|-----------------|-------------------|
| Desktop/CLI global toggle | Segmented control in sidebar | SegmentedControl in NavigationView header |
| MCP Server management | Two-column drag-and-drop (active/inactive) | Two-column with WinUI drag-and-drop |
| JSON editor with validation | TextEditor + JSONSerialization | TextBox + System.Text.Json |
| Copy to clipboard | NSPasteboard | Windows.ApplicationModel.DataTransfer |
| Hooks viewer/editor | Read/write settings.json hooks | Same logic, Windows paths |
| Agents viewer/editor | Scan .md files, parse YAML frontmatter | Same logic, Windows paths |
| Skills enable/disable | Move files to/from .disabled/ | Same logic, Windows paths |
| Backups (per-mode) | Timestamped JSON snapshots, diff viewer | Same logic, Windows paths |
| Open in file manager | NSWorkspace → Finder | Process.Start("explorer.exe", "/select,...") |
| Restart banner | "Restart Claude Desktop/Code to apply" | Same |

### Intentional platform differences

- **Cursor changes on drag dividers** — macOS uses NSCursor push/pop. Windows uses standard WinUI drag affordances.
- **File permissions** — macOS sets POSIX 0o600/0o700. Windows v1 relies on default user-directory permissions (`%APPDATA%` is user-only). Explicit NTFS ACLs deferred.
- **MSIX path detection** — Windows-only.

---

## 4. Data Flow

### Config read/write flow (both platforms)

```
User toggles Desktop/CLI
        │
        ▼
ConfigManager.mode = .desktop | .cli
        │
        ▼
Computed paths switch to corresponding files
        │
        ├── loadActive()  → parse mcpServers from target config JSON
        ├── loadStored()  → parse stored_servers_{mode}.json
        ├── loadHooks()   → parse hooks from settings.json (CLI only)
        ├── loadAgents()  → scan agent .md files (CLI only)
        └── loadSkills()  → scan command/skill dirs (CLI only)
        │
        ▼
Views update reactively
```

### Write operations

1. Read the full JSON file
2. Modify the relevant key (`mcpServers`, `hooks`, etc.)
3. Write the full JSON back (preserving unrelated keys)
4. Trigger auto-backup (debounced, per-mode)

### Error handling

| Scenario | Behavior |
|----------|----------|
| Config file doesn't exist | Empty state: "No config found. Create one?" with button to write starter JSON |
| Config file has invalid JSON | Show parse error inline, don't overwrite |
| Claude Desktop not installed | Desktop mode: "Claude Desktop config not found at [path]" |
| Claude Code not installed | CLI mode: "Claude Code config not found at [path]" |
| MSIX vs standard conflict (Windows) | PathResolver prefers MSIX path, shows info banner if both detected |
| File permission denied | Show error with path, suggest fix (run as admin on Windows, check permissions on macOS) |

### Backup storage (per-mode)

```
{app-storage-dir}/
├── stored_servers_desktop.json
├── stored_servers_cli.json
├── backups/
│   ├── desktop/
│   │   └── config_YYYY-MM-DD_HH-mm-ss.json
│   └── cli/
│       └── config_YYYY-MM-DD_HH-mm-ss.json
```

30 max per mode, 5-minute debounce, hash-based deduplication.

---

## 5. Implementation Order

These are two independent deliverables that can be implemented sequentially:

1. **Desktop/CLI toggle on macOS** — implement first, validates the toggle design and config path logic in the existing codebase
2. **Windows port** — new repo, builds the full app including the toggle from the start

---

## 6. Out of Scope (v1)

- No cross-platform shared library or shared config format
- No auto-update mechanism
- No sync between Desktop and CLI configs (toggle just switches which one you view)
- No Windows Store distribution
- No explicit NTFS ACL hardening (defer to v2)

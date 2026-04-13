# Changelog

## 0.2.5

### Added
- **Project-scoped MCPs tab** -- CLI mode now shows a "User MCPs" / "Project MCPs" segmented control. The Project tab displays MCP servers defined per-project in `~/.claude.json`, grouped by project path with threaded layout. Clicking a project MCP shows its config read-only in the detail panel.
- Update modal now renders release notes as styled markdown (headings, bold, lists, links) instead of raw text.

### Fixed
- MCP view layout no longer overflows -- detail panel, status footer, and action buttons stay clipped within their container at all window sizes.
- Detail panel and columns use proper flex sizing to prevent collapsed columns when the tab bar is present.

## 0.2.4

### Added
- **Custom catalog feeds** -- Add custom feed URLs (forks, private catalogs, VPN-only sources) via the Marketplace sidebar. Feeds merge with the built-in catalog, with custom servers appearing first. Each feed is cached independently for offline resilience.
- Feed manager UI in the Marketplace sidebar with status indicators, toggle switches, and inline add form.
- Feed origin badge on server rows to distinguish custom feed servers from built-in catalog entries.

### Fixed
- **CLI mode reads the correct config file** -- CLI mode now reads MCP servers from `~/.claude.json` (the file `claude mcp add` writes to) instead of `~/.claude/settings.json`, which only contains hooks and permissions.
- Update modal release notes now display in a collapsible accordion instead of raw text.
- Button hover states in the update and about modals no longer break due to CSS specificity conflicts.

## 0.2.3

### Added
- Custom About modal with app icon, version, and "Check for Updates" button.
- Toast feedback when already on the latest version ("You're on the latest version... for now.").

## 0.2.2

### Added
- In-app updater with automatic check during splash screen.
- Manual "Check for Updates" in the native application menu.
- Update modal with download progress bar and "Restart Now" button.
- Native OS menus (macOS app menu, Windows/Linux Help menu).
- Pre-commit hook to block AI process docs and secrets from being committed.

## 0.2.1 and earlier

Initial releases with MCP server management, marketplace, hooks, agents, skills, backups, and Desktop/CLI mode toggle.

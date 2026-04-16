# Changelog

## 0.3.0

### Added
- **One-click MCP auto-install from the marketplace** -- Supported catalog entries now detect runtime prerequisites (Node / uv / Docker), show a typed configuration form (string, secret, path, path array, URL, number), run bounded install steps with streaming progress, and write the final server config automatically. Retrying after an error preserves your form values and detected prerequisites.
- **Bundled `uv` 0.11.6 as a signed sidecar** -- Python-based MCP servers install end-to-end without a system `uv` install. Binary is SHA-256 verified at build time.
- **Catalog schema v1.1** -- Optional `prerequisites`, `install`, and `configFields` per server. The install step DSL is bounded (`npmWarmup` / `uvxWarmup` / `dockerPull` / `none`) so unknown step types can never become arbitrary shell execution. Legacy catalog entries without these fields continue to work unchanged via the existing JSON-edit flow.
- Forward-compat catch-all for unknown install step types — older clients won't crash on future catalog additions, they just fall through to the legacy path.
- Manual smoke-test checklist at `docs/release-checklists/auto-install-smoke.md`.

### Fixed
- **Marketplace Homepage / Repo buttons** now open in the OS default browser. Plain `<a target="_blank">` clicks were silently swallowed by the Tauri webview — routed through the opener plugin instead.
- **MCP Servers view no longer scrolls the entire pane** when the list or editor grows. The list and snippet editor each own their internal scroll (matching every other view); the header, restart banner, and resize handle stay put.
- **`cut-release.sh` now actually bumps `Cargo.toml`** -- the previous `0,/pat/` sed range was a GNU extension that BSD sed (macOS) accepted but silently ignored, leaving the crate version stuck at 0.2.4 through the 0.2.5 and 0.2.6 releases. Switched to a `[package]`-scoped range address.

## 0.2.6

### Added
- **Project-scoped MCPs tab** -- CLI mode now shows a "User MCPs" / "Project MCPs" segmented control. The Project tab displays MCP servers defined per-project in `~/.claude.json`, grouped by project path with threaded layout. Clicking a project MCP shows its config read-only in the detail panel.
- **Auto-unwrap mcpServers wrapper** -- Pasting a full `{ "mcpServers": { ... } }` config into the detail editor auto-extracts the inner server body on paste and on save.
- **Add Custom Feed modal** -- Replaced the inline feed form with a proper modal dialog for adding custom catalog feeds.
- Update modal now renders release notes as styled markdown (headings, bold, lists, links) instead of raw text.
- Test suites: 28 frontend tests (vitest) and 25 Rust tests covering validation, JSON helpers, and config unwrapping.

### Fixed
- MCP view layout no longer overflows -- detail panel, status footer, and action buttons stay clipped within their container at all window sizes.
- Detail panel and columns use proper flex sizing to prevent collapsed columns when the tab bar is present.
- Config validation is now a non-blocking warning -- users can save any valid JSON object, not just configs with `command` or `url`.

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

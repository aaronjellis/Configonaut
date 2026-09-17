# Changelog

## Unreleased

> Rename this section to the version number when cutting a release —
> `.github/workflows/release.yml` uses the first `## ` section as the notes.

### Added
- **All current hook events** in the New Hook picker (33 events, from PreToolUse to TeammateIdle), instead of eight.
- **Non-command hooks are listed** -- prompt, agent, http, and mcp_tool handlers now show in the Hooks view with a summary instead of being hidden.
- **Migrate leftover servers from settings.json** -- versions before 0.2.4 wrote MCP servers to `~/.claude/settings.json`, which Claude Code ignores. CLI mode now shows a banner with a one-click move into `~/.claude.json`. Entries whose name already exists (active or stored) are left alone and reported, and the original block is archived under Configonaut's storage directory before it is removed.

### Changed
- **Remote servers get a `type` in CLI mode** -- Claude Code skips `url`-only entries, so `http` (or `sse`) is filled in when writing to `~/.claude.json`.

### Fixed
- **Escape cancels a server rename** instead of committing the typed name.
- **Unsaved hook, agent, skill, and MCP server edits survive Enable/Disable and background refreshes** -- toggling or refreshing no longer reloads the editor from disk out from under an in-progress edit. The ↻ Reload button now re-reads the open editor's contents, where before it only refreshed the list.
- **About, release-notes and help links open in the browser without risking the app frame** -- they're rendered as buttons instead of `<a href>` links, so middle-click or the context menu's "Open Link" can no longer navigate the Tauri window away from the app.
- **Feed remove/toggle failures show a toast** instead of failing silently; a failed catalog refresh after a successful change is reported separately as a warning.
- **CLI-mode backups and restores touch only `mcpServers`** -- restoring a backup of `~/.claude.json` used to overwrite the whole file, rolling back the OAuth account, project trust decisions, and caches that Claude Code owns. Backups taken in CLI mode now contain just the servers, and restoring an older full-file backup only replaces the servers.
- **Content Security Policy enabled** for the app's webviews, and the webview no longer holds any shell permission (the bundled `uv` sidecar is only ever invoked from Rust). Previously the webview had no CSP and could run the sidecar with arbitrary arguments.
- **Guided Setup installs into the current mode** -- the auto-install flow always wrote to Claude Desktop's config, even in CLI mode. It now targets the active mode, wraps `npx`/`uvx` for Windows, picks a unique name on collision, records the catalog link (so the "Installed" badge and secret checks work), and can install servers from custom feeds. On Windows the managed Node.js PATH is now injected before the `cmd /c` wrap, so servers installed after the in-app Node download can start.
- **Disabling a hook actually disables it** -- Claude Code has no per-rule disable flag, so the old `disabled: true` marker left hooks running. Disabled rules now move to Configonaut's `disabled_hooks.json` and are restored verbatim on re-enable. Existing flagged rules are migrated on first launch.
- **Deleting a hook removes stale disabled copies** so the rule can't resurface as "disabled".
- **New skills are created as `~/.claude/skills/<name>/SKILL.md`** -- the only layout Claude Code loads. Flat `<name>.md` files created by earlier versions are still listed; the app now refuses to create a skill whose legacy file already exists.
- **Plugins are discovered from `installed_plugins.json`** -- every installed plugin from any marketplace now appears in Agents and Skills, and Enable/Disable uses the real `<plugin>@<marketplace>` key. Previously the app scanned the official marketplace's source clone, which listed uninstalled plugins and missed everything else.
- **Every write to `~/.claude/settings.json` is now preceded by a timestamped backup** under Configonaut's storage directory (30 kept).
- **The in-app Node.js download is verified against nodejs.org's SHASUMS256.txt** before extraction. A mismatch discards the archive and reports an error instead of running the binary.
- **Install steps from catalog feeds are sandboxed a little harder** -- package and image names that look like command-line flags are refused, and the `npx` / `uvx` / `docker` warmup processes now receive only an allowlisted environment (PATH, HOME, proxy and cache settings) instead of the whole shell environment. Every step is checked before any of them runs. On Windows the npm warmup now spawns `npx.cmd` directly (it previously failed to start). If your `.npmrc` references an environment variable such as an auth token, the warmup now explains why it can't run instead of failing generically.
- **Download progress shows real numbers** -- the percent/bytes fields were sent in snake_case and rendered as "NaN MB".
- **Python (uvx) servers install without a system uv** -- the warmup and the written config now use the bundled uv (`uv tool run …`) when `uvx` is not on PATH, matching what the prerequisite check reports.
- **Retry is only offered for retryable install errors.**

## 0.4.0

### Added
- **In-app Node.js installer** -- Catalog servers that require Node now offer a one-click "Install" button right in the prerequisites row. Configonaut downloads the latest LTS tarball straight from nodejs.org into the app's storage dir, extracts via system tar, verifies via `node -v`, and shows a real-time progress bar with percentage + bytes-transferred. No system Node required to use npx-based MCP servers. Marked with a "managed by Configonaut" badge once installed.
- **Post-install notes** -- Catalog entries can declare `postInstallNotes: [{ title, body, url? }]` to surface out-of-band setup steps the app can't automate (e.g. running `sf org login web` to authorize a Salesforce org). The Setup flow now ends in a numbered "Next Steps" screen for these entries instead of silently closing.
- **Rename servers from the detail panel** -- Double-click a server name in the MCP view to rename it inline. Enter commits, Escape cancels. Backend re-keys the entry in either active or stored, cross-checks both maps to prevent collisions, and updates the catalog-links sidecar so marketplace linkage survives.
- **Bare-fragment JSON pastes** -- The "Paste JSON" tab now accepts `"server-name": { ... }` fragments, full `{ "mcpServers": { ... } }` wrappers, and snippets with trailing commas. Server names are auto-extracted from the JSON key when possible.
- **Official Salesforce DX MCP** -- New catalog entry for `@salesforce/mcp` from Salesforce CLI team, with post-install notes walking through sf CLI install and org authorization.

### Changed
- **GitHub MCP switched from docker to npx** -- Catalog entry now uses `@github/github-mcp-server` via npx instead of the docker image. Drops the docker prerequisite; users only need Node.
- **Atlassian MCP URL updated** -- Atlassian retired the `/v1/sse` endpoint in favor of `/v1/mcp`.
- **Field defaults are now seeded at load time** -- Setup form now pre-populates `field.default` values into the form state on schema load, so required fields with sensible defaults don't strand the Install button until the user manually re-types each one.

### Fixed
- **Remote-server warning in Desktop mode** -- Turning on a stored server with a `url` field now shows a confirmation modal. Claude Desktop has a known issue where url-based MCP entries can cause it to wipe all configured servers on restart; the warning surfaces that risk before the user proceeds.
- **Setup install events fire exactly once** -- The install progress event listener now handles log streaming only; the Tauri command's return value owns the `done` / `error` signals. Eliminates a path where both the listener and the invoke return dispatched `installDone`.
- **Listener cleanup race in Setup** -- Async listener registration now uses a `cancelled` flag pattern so unmounting the component before `onInstallProgress.then(...)` resolves doesn't leave a stale listener attached.
- **Rename double-commit guard** -- Pressing Enter while renaming a server no longer fires the rename IPC twice (Enter → commitRename → setRenaming(false) → input unmount → blur → commitRename again). A ref short-circuits the second call.
- **Windows `npx` / `uvx` spawning** -- Server configs now wrap `npx`, `uvx`, `python`, `pip`, and their numbered variants in `cmd /c` on Windows. Node's `child_process.spawn` can't execute `.cmd` shims directly, and Claude Desktop / Claude Code spawn MCP server commands without a shell, so the wrapping is needed for these to actually run.

## 0.3.1

> 0.3.0 was tagged but never published — the macOS universal build failed
> at sidecar bundling because Tauri's `universal-apple-darwin` target
> wants a single fat `uv-universal-apple-darwin` binary, and the download
> script only produced per-arch files. 0.3.1 is 0.3.0 plus the lipo fix.

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
- **macOS universal release bundles** -- `download-uv.sh` now lipo's the per-arch uv binaries into `uv-universal-apple-darwin` when running on macOS, so Tauri's universal-target bundler finds the expected sidecar. (Also skipped on Linux/Windows runners that don't have lipo and don't need the universal binary.)

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

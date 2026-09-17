# Configonaut

Desktop app (Tauri 2 + React 19 + Rust) that edits Claude Desktop and Claude Code config files. Not a web service; ignore any Docker/SaaS deployment instructions inherited from parent directories.

## Layout

- `tauri-app/src-tauri/src/` — Rust backend. `config.rs` owns MCP servers + backups, `claude_code.rs` owns hooks/agents/skills/plugins, `catalog.rs` the marketplace, `installer.rs` auto-install, `paths.rs` every on-disk location.
- `tauri-app/src/` — React frontend. `api.ts` is the only place that calls `invoke()`; `types.ts` mirrors `models.rs`.
- `marketplace-catalog/` — mirror of the `aaronjellis/configonaut-catalog` repo. `tauri-app/src-tauri/resources/catalog-baseline.json` must match it semantically (`scripts/sync-catalog-baseline.sh`; CI compares `jq -S` output).

## Commands

- Frontend tests: `cd tauri-app && bun run test`
- Typecheck: `cd tauri-app && bun run typecheck`
- Rust tests: `cd tauri-app/src-tauri && cargo test`
- Clippy (CI uses `-D warnings`): `cd tauri-app/src-tauri && cargo clippy --all-targets -- -D warnings`
- Run the app: `cd tauri-app && bun run tauri dev` (run `src-tauri/binaries/download-uv.sh` once first)
- Release: add a `## <version>` section to `CHANGELOG.md` (rename `## Unreleased`), then `scripts/cut-release.sh <version>`

## Rules

- Claude Code facts (file paths, hook events, MCP `type` field, plugin layout) come from code.claude.com/docs — verify before changing assumptions in `paths.rs` or `claude_code.rs`.
- Never write to `~/.claude.json` outside `config.rs`; in CLI mode only the `mcpServers` key is ours.
- External links are `<ExternalLink>` buttons (`components/ExternalLink.tsx`), never `<a href>`: plain anchors are swallowed by the webview or can navigate the app frame away.
- `window.confirm/alert/prompt` are no-ops in the webview; use in-app modals.
- Keep `types.ts` in sync with `models.rs` when changing a serialized struct; serde enums need `rename_all_fields = "camelCase"` for struct-variant fields.
- `docs/superpowers/` and `.claude/` are local scratch and are blocked by the pre-commit hook.

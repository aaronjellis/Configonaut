# Configonaut — Next Steps

## Features

### Prerequisite detection for stdio MCP servers
The catalog has `transport` ("stdio" | "remote") and `requirements` (["node"] | ["python","uv"] | ["docker"] | []) fields on every entry. Build a `check_runtime` Rust command that detects whether Node.js, Python+uv, and Docker are installed on the user's machine. The marketplace UI should warn before enabling a stdio server if its requirements aren't met, and ideally offer to install missing runtimes.

### Catalog remote refresh
The app bundles `catalog-baseline.json` but has no mechanism to fetch updates. Wire up a remote catalog URL (GitHub Releases asset or Cloudflare Pages) so new MCP servers can be added without shipping a new app version. Consider a fetch-and-merge step on app startup or a manual "Refresh catalog" button in the marketplace UI.

## Housekeeping

- [ ] Merge `feature/desktop-cli-toggle` into `main` — future releases should tag from `main`
- [ ] Delete `commit-session2.sh` from repo root
- [ ] Remove Swift sources (`Sources/`, `Configonaut.xcodeproj`, etc.) — confirmed Tauri-only as of 2026-04-12

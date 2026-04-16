# Auto-Install Smoke Test (manual, per release)

Run on a clean macOS VM (or a fresh user account) with no Node, no uv, no Docker installed system-wide.

## Setup
- [ ] Install the latest Configonaut release.
- [ ] Verify the app launches without errors.

## Sidecar
- [ ] Open Marketplace → pick a uvx-launched server (e.g. `mcp-server-time`) → `+ Add`.
- [ ] Setup step shows uv prereq as ✓ already-installed (sidecar source).

## Node detect-and-instruct
- [ ] Pick `filesystem` from Marketplace → `+ Add`.
- [ ] Setup step shows Node.js as ○ unchecked with "Open install page" link.
- [ ] Click "Open install page" → nodejs.org opens in browser.
- [ ] Install Node externally, return to app, click "Re-check" on the row.
- [ ] Row turns green with detected version.
- [ ] Add at least one path, click Install Server.
- [ ] Progress UI shows "Pre-fetching @modelcontextprotocol/server-filesystem…" with streaming log lines.
- [ ] On success: modal closes, server appears in MCP Servers list.
- [ ] In CLI mode: `claude mcp list` shows the new server.

## Docker detect-and-instruct
- [ ] Pick a docker-launched server → `+ Add`.
- [ ] Setup step shows Docker prereq as ○ unchecked.
- [ ] Install Docker Desktop, start the daemon, click Re-check.
- [ ] Row turns green.
- [ ] Click Install Server → expect `docker pull` progress in the log → success.

## Error path
- [ ] Disable network → click Install Server on a uvx server.
- [ ] Expect "Couldn't reach the registry" error message + Retry button.
- [ ] Click Retry → fails again with same message.
- [ ] Re-enable network → Retry → succeeds.

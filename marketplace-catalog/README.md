# configonaut-catalog

Community catalog of MCP servers that ship in the **Configonaut** marketplace.

Configonaut is a macOS app for managing MCP (Model Context Protocol) servers used
by Claude Desktop and Claude Code. This repository hosts the curated list of
ready-to-install servers that appear inside the in-app "Add Server" marketplace.

## How it works

1. This repo hosts a single `catalog.json` file.
2. Configonaut fetches `https://raw.githubusercontent.com/aaronellis/configonaut-catalog/main/catalog.json` once on launch (and on user refresh).
3. It caches the result locally at `~/Library/Application Support/Configonaut/catalog.json` with a last-good fallback so the marketplace still works offline.
4. Users browse, search, and one-click add servers to their `claude_desktop_config.json` (or `~/.claude/settings.json` in CLI mode).

The app never uploads your tokens anywhere — the catalog only declares *which* environment variables a server needs, not their values.

## Repository layout

```
catalog.json                  # The live catalog consumed by Configonaut
schemas/catalog-v1.json       # JSON schema for catalog.json
scripts/build_catalog.py      # Regenerates catalog.json from the Python source
docs/
  CONTRIBUTING.md             # How to add or update an entry
  SCHEMA.md                   # Field-by-field schema reference
```

## Catalog status

- Format version: **v1**
- Servers: **64** (as of this commit)
- Categories: 15

Coverage priorities, in order:

1. Official Anthropic reference servers (`@modelcontextprotocol/server-*`)
2. First-party vendor MCPs (GitHub, Stripe, Notion, Linear, Figma, Atlassian, Cloudflare, Vercel, etc.)
3. Widely-used community servers with healthy maintenance signals
4. Niche but frequently requested servers

We intentionally do **not** try to mirror the full 1,400+ list from `github.com/modelcontextprotocol/servers`. The marketplace is curated, not comprehensive. If an entry stops working, we remove it rather than leave it broken.

## Adding a server

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md). TL;DR:

1. Add a dict to `SERVERS` in `scripts/build_catalog.py`.
2. Run `python3 scripts/build_catalog.py > catalog.json`.
3. Run `python3 scripts/validate.py` (checks schema + pings npm/PyPI for package existence).
4. Open a PR with a one-line rationale and a link to the server's repo.

## Schema

`catalog.json` is validated against [`schemas/catalog-v1.json`](schemas/catalog-v1.json). Key fields:

| Field | Purpose |
|-------|---------|
| `id` | Stable slug used as the default server key in the user's config |
| `name`, `description`, `category`, `tags` | Marketplace display |
| `publisher` | `official` \| `vendor` \| `community` + `verified` flag |
| `config` | The literal JSON block written to `claude_desktop_config.json`. Supports stdio (command/args/env) and remote (url/headers). |
| `envVars` | Declarative list of env vars. Each has `required`, `secret`, `placeholder`, `helpUrl`. Used to drive the "missing tokens" detector in Configonaut. |
| `setupNotes` | Free-form instructions shown under the add form |

## Secret detection contract

Configonaut uses the `envVars` declarations to decide whether a server is "ready to enable". The rules:

- If any `envVars[].required == true` has a value that is empty, matches `/your[-_]?/i`, contains `<`, or equals the `placeholder`, the server is flagged **Needs setup** and cannot be moved to Active.
- Users can still save the server to Inactive with placeholders in place.
- The placeholder is never written to the saved config file as-is; only the user's actual value is.

## Licensing

The catalog metadata in this repository is licensed under CC0-1.0 (public domain).
Individual server implementations retain their own licenses — check each entry's
`license` field and linked repository before using in your project.

## Not affiliated with Anthropic

Configonaut and this catalog are community projects. "Claude" and "Model Context
Protocol" are trademarks of their respective owners.

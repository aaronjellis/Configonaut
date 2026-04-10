# MCP Marketplace Feature — Implementation Plan

**Status:** Proposed
**Target version:** Configonaut 1.3.0
**Author:** Claude (for Aaron)
**Last updated:** 2026-04-09

## Summary

Add a curated marketplace to Configonaut so users can browse a catalog of known-good MCP servers and add them with one click, instead of hunting down JSON blocks across vendor docs. The catalog lives in a separate public GitHub repo (`configonaut-catalog`) and is fetched as a single `catalog.json` with a local cache and last-good fallback. When a user picks a server, Configonaut inserts its config into the Inactive list, pre-fills placeholders, and flags any required secrets the user still needs to enter before the server can be turned on.

The initial catalog ships with 64 servers covering reference, development, databases, cloud, browser, search, productivity, communication, design, monitoring, finance, AI, media, CRM, and local files. See `marketplace-catalog/catalog.json`.

## Goals

1. **Discovery** — users see a browseable list of popular MCP servers without leaving the app.
2. **One-click add** — picking a server inserts a ready-to-edit config into Inactive.
3. **Secret hygiene** — the app can tell, without running the server, whether any required env vars are still placeholders and block activation until they are filled.
4. **Keep the catalog fresh without app releases** — catalog lives on GitHub and is refreshed at runtime.
5. **Never phone home with user data** — only the catalog URL is fetched; tokens never leave the device.

## Non-goals (for v1)

- Automated install of binaries/Docker images (the user still needs `node`/`uv`/`docker` on PATH).
- OAuth flows handled inside Configonaut. If a vendor MCP uses OAuth, we show the remote URL and Claude Desktop handles auth on first use.
- Publishing our own servers. The catalog is curation only.
- Full-text search across the upstream 1,400+ server list. We ship a curated subset.

## User experience

### Entry point

Today the "+ Add Server" button (top right of `MCPView`) opens a paste-JSON panel. Under this plan, clicking it opens a **two-tab bottom panel** instead:

```
┌──────────────────────────────────────────────────────────────┐
│  ◉ Marketplace         ○ Paste JSON              [ × ]       │
├──────────────────────────────────────────────────────────────┤
│  [Search…]                [All ▾] [Official ✓] [No keys ✓]   │
│                                                              │
│  ┌── GitHub ────────── official ✓ ── needs 1 key ────────┐   │
│  │ Create issues, manage PRs, search code…  [+ Add] [↗] │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌── Notion ────────── official ✓ ── needs 1 key ────────┐   │
│  │ Search and edit pages and databases.     [+ Add] [↗] │   │
│  └──────────────────────────────────────────────────────┘   │
│  …                                                          │
├──────────────────────────────────────────────────────────────┤
│  64 servers · updated 2 hours ago · source: github.com/…    │
└──────────────────────────────────────────────────────────────┘
```

Notes:

- Default tab is **Marketplace**. Power users can flip to the existing paste-JSON flow.
- Search is client-side, filters name/description/tags.
- Filters: category dropdown, publisher type (official/vendor/community), "no keys required" toggle.
- Each row: name, one-line description, publisher chip, license chip, popularity, `+ Add` primary button, `↗` link to homepage.
- Footer shows last fetch timestamp and a refresh button.

### Add flow

Clicking `+ Add` does one of two things:

1. **No required env vars** → server goes straight to **Inactive**, with a toast "Added *GitHub* to Inactive — drag it to Active to turn it on."
2. **Has required env vars** → server goes to **Inactive** *and* the detail panel auto-opens to a new **Setup** section showing each secret with a labeled field, help link, and "Paste value" button. Until all required secrets are filled, the "Turn On" button is disabled and shows a red "Needs setup" badge.

Setup fields:
- Render masked (•••••) if `secret: true`. Eye icon to reveal.
- Show the declared `placeholder` as greyed hint text only — never written to the saved config unless the user explicitly types it.
- Show `description` under the field.
- Show `helpUrl` as a small "How to get this →" link.

### Secret validation contract

A server is considered **ready to enable** when every `envVar` with `required: true` has a concrete value in the saved config's `env` block that does *not* match any of:

- Empty string
- The string declared as `placeholder`
- A regex hit on any of these heuristic patterns:
  - `^<.+>$` (angle-bracket placeholder)
  - `(?i)your[_-]?(api[_-]?)?(key|token|secret)` (YOUR_API_KEY, your-token, etc.)
  - `x{4,}` (xxxx / XXXXXX)
  - `paste[_-]?here`, `insert[_-]?here`, `replace[_-]?me`
  - `example[_-]?(key|token)`

If any required var fails this check, the server is displayed with a red "Needs setup" badge in the Inactive column and cannot be dragged/toggled to Active. The user can still save/edit/delete it.

Servers with a remote `url` (HTTP/SSE) transport and no declared env vars are always considered ready.

### Refresh

- Catalog is fetched on app launch if the cached file is older than 6 hours.
- Manual refresh button in the marketplace footer.
- Fetch timeout: 5s. On failure, we silently fall back to the cached copy and the footer shows a yellow "Last refresh failed" indicator.
- If no cached copy exists (first run, offline), we show the baseline catalog bundled with the app.

## Architecture

### New files

```
Sources/
  Marketplace/
    CatalogModels.swift          # Codable structs for catalog.json
    CatalogStore.swift           # Fetch, cache, refresh, in-memory state
    SecretValidator.swift        # The placeholder-detection logic
    MarketplaceView.swift        # The Marketplace tab (list + search + filters)
    SetupPanel.swift             # The post-add env var setup UI
Resources/
  catalog-baseline.json          # Shipped fallback (copy of catalog.json at release time)
```

### Modified files

- `Sources/MCPView.swift` — split the existing "Add Server" bottom panel into a tabbed container (Marketplace / Paste JSON). Add the `CatalogStore` as a `@StateObject`.
- `Sources/ConfigManager.swift` — add `addFromCatalog(_ entry: CatalogServer)` that:
  - Instantiates the `config` block with env values filled to empty strings.
  - Writes to the Inactive list (Stored).
  - Stores a reference to the `catalogId` in `ServerEntry` so the detail panel can surface `envVars` metadata later.
- `Sources/ConfigManager.swift` — add `isReadyToEnable(_ name: String) -> Bool` used by `moveToActive` and the UI.

### Data model (Swift)

```swift
struct Catalog: Codable {
    let version: String
    let generatedAt: String
    let source: String?
    let categories: [CatalogCategory]
    let servers: [CatalogServer]
}

struct CatalogCategory: Codable, Identifiable {
    let id: String
    let label: String
    let icon: String?
    let description: String?
}

struct CatalogServer: Codable, Identifiable {
    let id: String
    let name: String
    let description: String
    let category: String
    let tags: [String]?
    let publisher: Publisher
    let homepage: String
    let repository: String?
    let license: String?
    let popularity: Int?
    let config: ConfigBlock          // either stdio or remote
    let envVars: [CatalogEnvVar]?
    let setupNotes: String?
    let deprecated: Bool?
}

struct Publisher: Codable {
    let name: String
    let type: String  // "official" | "vendor" | "community"
    let verified: Bool?
}

enum ConfigBlock: Codable {
    case stdio(command: String, args: [String]?, env: [String: String]?)
    case remote(url: String, headers: [String: String]?)

    // Custom init(from:) decoding both shapes based on presence of "url" vs "command"
}

struct CatalogEnvVar: Codable, Identifiable {
    var id: String { name }
    let name: String
    let description: String
    let required: Bool?      // default true
    let secret: Bool?        // default true
    let placeholder: String?
    let helpUrl: String?
    let defaultValue: String?  // maps to "default"
}
```

`ConfigBlock` serializes back to the same JSON shape the user would paste today, so `ConfigManager.addToStored` / `addToActive` already handle it — we just build the `[String: Any]` directly.

### CatalogStore

```swift
@MainActor
final class CatalogStore: ObservableObject {
    @Published private(set) var catalog: Catalog?
    @Published private(set) var lastRefreshed: Date?
    @Published private(set) var refreshError: String?
    @Published private(set) var isRefreshing = false

    private let remoteURL = URL(string: "https://raw.githubusercontent.com/aaronellis/configonaut-catalog/main/catalog.json")!
    private var cacheURL: URL {
        ConfigManager.storageDir.appendingPathComponent("catalog.json")
    }
    private let refreshInterval: TimeInterval = 6 * 3600

    func bootstrap() async { /* load cache → if stale, refresh → else load baseline */ }
    func refresh(force: Bool = false) async { /* URLSession fetch → validate → write cache */ }
    func search(query: String, category: String?, publisherType: String?, noKeys: Bool) -> [CatalogServer]
}
```

Load order on first launch:
1. Try cache at `~/Library/Application Support/Configonaut/catalog.json`.
2. If absent, load bundled `Resources/catalog-baseline.json`.
3. Kick off an async refresh if cache is older than 6 hours.

Refresh writes to a temp file, validates the JSON parses against `Catalog`, then atomically moves into place. On decode error we keep the old cache.

### SecretValidator

```swift
enum SecretValidator {
    static let placeholderPatterns: [NSRegularExpression] = [
        #"^<.+>$"#,
        #"(?i)your[_-]?(api[_-]?)?(key|token|secret)"#,
        #"^x{4,}$"#,
        #"(?i)paste[_-]?here"#,
        #"(?i)insert[_-]?here"#,
        #"(?i)replace[_-]?me"#,
        #"(?i)example[_-]?(key|token)"#,
    ].compactMap { try? NSRegularExpression(pattern: $0) }

    /// Returns true if `value` appears to be a real secret, not a placeholder.
    static func looksReal(_ value: String, placeholder: String?) -> Bool {
        let trimmed = value.trimmingCharacters(in: .whitespaces)
        if trimmed.isEmpty { return false }
        if let p = placeholder, trimmed == p { return false }
        for re in placeholderPatterns {
            let r = NSRange(trimmed.startIndex..., in: trimmed)
            if re.firstMatch(in: trimmed, range: r) != nil { return false }
        }
        return true
    }

    /// Evaluate an entire ServerEntry against its catalog envVars.
    /// Returns .ready or .needsSetup([missingVarName]).
    static func evaluate(_ entry: ServerEntry, envVars: [CatalogEnvVar]) -> ReadinessState { ... }
}
```

### ConfigManager extensions

```swift
extension ConfigManager {
    func addFromCatalog(_ server: CatalogServer) {
        let cfg = server.config.asDictionary()   // [String: Any]
        addToStored([(server.id, cfg)])
        catalogLinks[server.id] = server.id       // so detail panel can find envVars later
    }

    func isReadyToEnable(_ name: String, source: ServerSource) -> Bool {
        guard let catalogId = catalogLinks[name],
              let catalogEntry = CatalogStore.shared.catalog?.servers.first(where: { $0.id == catalogId }),
              let envVars = catalogEntry.envVars else { return true }

        let entry: ServerEntry? = (source == .active
            ? activeServers : storedServers).first { $0.name == name }
        guard let entry else { return true }

        return SecretValidator.evaluate(entry, envVars: envVars) == .ready
    }
}
```

`catalogLinks` is a `[String: String]` persisted next to `stored_servers_<mode>.json` so we remember which user-added server came from which catalog entry. (Key = server name in the user's config. Value = catalog id.)

### moveToActive guard

`moveToActive(_ name:)` gains a ready check:

```swift
func moveToActive(_ name: String) {
    guard isReadyToEnable(name, source: .stored) else {
        setStatus("\"\(name)\" is missing required tokens — fill them in first.", isError: true)
        return
    }
    // existing body…
}
```

The UI button is also disabled when not ready, so this is belt-and-suspenders.

## MarketplaceView (outline)

```swift
struct MarketplaceView: View {
    @EnvironmentObject var config: ConfigManager
    @StateObject private var store = CatalogStore.shared
    @State private var query = ""
    @State private var categoryFilter: String? = nil
    @State private var onlyOfficial = false
    @State private var onlyNoKeys = false

    var body: some View {
        VStack(spacing: 0) {
            header           // search + filters + refresh button
            Divider()
            serverList       // LazyVStack of CatalogRow
            Divider()
            footer           // count, last refresh, status
        }
        .task { await store.bootstrap() }
    }
}
```

`CatalogRow` is a simple card — name, description, publisher chip, license chip, `+ Add`, `↗`. Clicking `+ Add` calls `config.addFromCatalog(server)`, closes the marketplace panel, selects the server in the Inactive column, and scrolls the detail panel to the Setup section.

## Testing strategy

- **Unit** — `SecretValidatorTests`: placeholders, regex hits, empty strings, realistic tokens all classified correctly.
- **Unit** — `CatalogStoreTests`: decode a pinned copy of `catalog.json`, ensure all stdio + remote variants decode, ensure atomic refresh writes a temp file then renames.
- **Snapshot** — `MarketplaceViewTests`: render the list with a fixture catalog, verify filters narrow results.
- **Manual QA checklist** — documented in `docs/QA_MARKETPLACE.md`:
  - Fresh install fetches remote catalog.
  - Offline install falls back to bundled baseline.
  - Add GitHub → server appears in Inactive with "Needs setup" badge → fill token → badge clears → toggle to Active succeeds.
  - Add a zero-env server (Playwright) → can toggle to Active immediately.
  - Add a remote server (Figma) → immediately ready, no Setup panel.

## Security / privacy

- Catalog JSON is fetched over HTTPS from raw.githubusercontent.com (no auth headers).
- We do a schema decode before replacing the cache — a malformed/hostile catalog fails closed to the previous copy.
- We never post any user config or tokens anywhere; the catalog is read-only.
- We do not execute any code from the catalog — it is data only. The `command` field is passed verbatim into the *user's* config file, the same thing a user pastes today.
- We should consider shipping a SHA256 of the catalog baseline and comparing against a signature on the refresh (out of scope for v1, note for v2).

## Rollout plan

1. **Sprint 1 (catalog repo + schema)** — Create the public `configonaut-catalog` GitHub repo using `marketplace-catalog/` as the seed. Set up GitHub Actions to run `validate.py` on every PR.
2. **Sprint 2 (Swift data + store)** — Implement `CatalogModels`, `CatalogStore`, and unit tests. Wire an async bootstrap in `ConfigonautApp`.
3. **Sprint 3 (UI)** — Build `MarketplaceView`, split `MCPView`'s add panel into tabs, add the Setup section to the detail panel.
4. **Sprint 4 (SecretValidator + ready state)** — Add the validator, gate `moveToActive`, surface the "Needs setup" badge, persist `catalogLinks`.
5. **Sprint 5 (polish)** — Loading states, error banners, telemetry-free analytics (local counter of "adds from marketplace" for the footer), docs updates.
6. **Release** — Ship as 1.3.0 with a short changelog and a 30-second demo gif in the README.

## Open questions

1. **Catalog repo ownership** — do you want the repo under your personal account or under a new org? The plan assumes `github.com/aaronellis/configonaut-catalog`. Update `CatalogStore.remoteURL` when the repo is created.
2. **Bundled baseline** — should we ship `catalog-baseline.json` at build time (always in sync with the app release) or embed it via a build script that pulls the latest `main` at build time?
3. **Extend to CLI mode** — this plan only talks about Desktop mode paths. The CLI mode uses `~/.claude/settings.json` and the same add logic should work since `ConfigManager` already abstracts the file. Double-check no CLI-specific edge cases.
4. **Popularity sort** — popularity is hand-maintained in the catalog today. Later we could sort by GitHub stars fetched via a nightly Action, but that's a v2.

## Appendix — initial catalog summary

Generated 2026-04-09. 64 servers, 15 categories.

```
reference     ████        4
development   ████        4
database      ███████████ 11
cloud         █████       5
browser       ███████     7
search        █████       5
productivity  █████████   9
communication █████       5
design        ██          2
monitoring    ████        4
finance       ██          2
ai            ██          2
media         █           1
crm           ██          2
filesystem    █           1
```

All reference servers from `github.com/modelcontextprotocol/servers` are included. Vendor MCPs include GitHub, GitLab, Notion, Linear, Atlassian, Sentry, Stripe, Cloudflare, Vercel, Netlify, AWS, Azure, Azure DevOps, Databricks, MongoDB, Redis, Neon, Supabase, Snowflake, DuckDB (MotherDuck), Brave Search, Tavily, Exa, Kagi, Perplexity, Playwright, Chrome DevTools, Browserbase, Firecrawl, Apify, Twilio, HubSpot, Figma, PagerDuty, Grafana, Hugging Face, ElevenLabs. Community entries fill in Discord, Gmail, Intercom, Asana, ClickUp, Trello, Obsidian, Google Drive, Google Maps, Salesforce, Figma (Framelink), Datadog, Replicate, Plaid, Postgres, SQLite, Puppeteer, Turso, and the archived-reference entries.

Every package reference has been pinged against `registry.npmjs.org` / `pypi.org` and returned HTTP 200 (`scripts/validate.py` in the catalog repo enforces this on every PR).

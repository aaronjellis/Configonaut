// Marketplace tab — ported from Sources/Marketplace/MarketplaceView.swift.
//
// Layout (fills the modal body below the tab strip):
//
//   ┌─────────────────────────────────────────────────────────────┐
//   │ [🔎 Search…]                            [↻]   [count pill]  │
//   ├────────────────┬────────────────────────────────────────────┤
//   │ Categories     │ Server rows (click to expand + edit JSON)  │
//   │  All       (64)│ ● GitHub      vendor ✓       [Install/✓]   │
//   │  Reference  (8)│ ● Notion      vendor ✓                     │
//   │  Development(12)│                                            │
//   │  ...            │                                            │
//   └────────────────┴────────────────────────────────────────────┘
//
// Clicking a row expands an inline JSON editor pre-populated with the
// catalog template. "Save to list" parses the edited JSON, calls
// `installFromCatalog`, and collapses the row.

import { useMemo, useState, type ReactNode } from "react";
import type {
  Catalog,
  CatalogCategory,
  CatalogServer,
  FeedEntry,
  FeedStatus,
  RuntimeStatus,
} from "../types";

interface Props {
  catalog: Catalog | null;
  catalogError: string | null;
  isRefreshing: boolean;
  /// Map of installed-server-name → catalog-id for "Installed" badges.
  links: Record<string, string>;
  runtimeStatus: RuntimeStatus | null;
  feeds: FeedEntry[];
  feedStatuses: FeedStatus[];
  onRefresh: () => void | Promise<void>;
  onInstall: (
    server: CatalogServer,
    customConfig: Record<string, unknown>,
    customName: string
  ) => Promise<void>;
  onAddFeed: (label: string, url: string) => Promise<void>;
  onRemoveFeed: (feedId: string) => Promise<void>;
  onToggleFeed: (feedId: string, enabled: boolean) => Promise<void>;
}

export function MarketplaceTab({
  catalog,
  catalogError,
  isRefreshing,
  links,
  runtimeStatus,
  feeds,
  feedStatuses,
  onRefresh,
  onInstall,
  onAddFeed,
  onRemoveFeed,
  onToggleFeed,
}: Props) {
  const [searchText, setSearchText] = useState("");
  const [selectedCategoryId, setSelectedCategoryId] = useState<string | null>(
    null
  );
  const [expandedServerId, setExpandedServerId] = useState<string | null>(null);
  const [editedJson, setEditedJson] = useState("");
  const [editError, setEditError] = useState<string | null>(null);
  const [busyInstalling, setBusyInstalling] = useState(false);

  // Feed manager state
  const [showFeedModal, setShowFeedModal] = useState(false);

  // Feed status lookup by id.
  const feedStatusMap = useMemo(() => {
    const m = new Map<string, FeedStatus>();
    for (const s of feedStatuses) m.set(s.id, s);
    return m;
  }, [feedStatuses]);

  // Set of installed catalog ids, for the "Installed" pill. Values of `links`
  // are catalog ids, keys are server names — either match marks a row as
  // installed (a user could rename the server on install, so we check both).
  const installedIds = useMemo(() => {
    return new Set(Object.values(links));
  }, [links]);

  // Server → category counts, computed off the unfiltered list so the sidebar
  // counts stay stable when the user changes the search box (matches Swift).
  const categoriesWithCounts = useMemo(() => {
    if (!catalog) return [] as Array<{ category: CatalogCategory; count: number }>;
    const counts = new Map<string, number>();
    for (const s of catalog.servers) {
      counts.set(s.category, (counts.get(s.category) ?? 0) + 1);
    }
    return catalog.categories.map((c) => ({
      category: c,
      count: counts.get(c.id) ?? 0,
    }));
  }, [catalog]);

  // Filtered + sorted server list — search across name/description/id/tags/
  // publisher, then category filter, then sort by popularity desc then name.
  const filteredServers = useMemo(() => {
    if (!catalog) return [] as CatalogServer[];
    const q = searchText.trim().toLowerCase();
    return catalog.servers
      .filter((s) => {
        if (selectedCategoryId && s.category !== selectedCategoryId) return false;
        if (!q) return true;
        if (s.name.toLowerCase().includes(q)) return true;
        if (s.description.toLowerCase().includes(q)) return true;
        if (s.id.toLowerCase().includes(q)) return true;
        if (s.tags.some((t) => t.toLowerCase().includes(q))) return true;
        if (s.publisher.name.toLowerCase().includes(q)) return true;
        return false;
      })
      .sort((a, b) => {
        if (b.popularity !== a.popularity) return b.popularity - a.popularity;
        return a.name.localeCompare(b.name);
      });
  }, [catalog, searchText, selectedCategoryId]);

  function isInstalled(server: CatalogServer): boolean {
    if (installedIds.has(server.id)) return true;
    // Also covers the case where the user installed something named the same
    // as the catalog id but we missed the link record for some reason.
    return Object.prototype.hasOwnProperty.call(links, server.id);
  }

  function handleToggleExpand(server: CatalogServer) {
    if (expandedServerId === server.id) {
      setExpandedServerId(null);
      setEditedJson("");
      setEditError(null);
      return;
    }
    setExpandedServerId(server.id);
    setEditedJson(defaultSnippet(server));
    setEditError(null);
  }

  function handleResetEdit(server: CatalogServer) {
    setEditedJson(defaultSnippet(server));
    setEditError(null);
  }

  async function handleInstall(server: CatalogServer) {
    setEditError(null);
    const parsed = parseEditedSnippet(editedJson, server.id);
    if (!parsed) {
      setEditError(
        "JSON looks invalid. Check for trailing commas or missing quotes."
      );
      return;
    }
    const { name, dict } = parsed;
    if (dict.command === undefined && dict.url === undefined) {
      setEditError(
        'Config must include either a "command" (stdio) or a "url" (http).'
      );
      return;
    }

    setBusyInstalling(true);
    try {
      await onInstall(server, dict, name);
      setExpandedServerId(null);
      setEditedJson("");
    } catch (e) {
      setEditError(String(e));
    } finally {
      setBusyInstalling(false);
    }
  }

  if (!catalog) {
    return (
      <div className="modal-body marketplace-body">
        <div className="empty">
          {catalogError ? (
            <>
              <div style={{ fontWeight: 500, marginBottom: 6 }}>
                Marketplace catalog not loaded
              </div>
              <div style={{ color: "var(--red)" }}>{catalogError}</div>
              <button
                className="primary"
                style={{ marginTop: 10 }}
                onClick={onRefresh}
              >
                Retry
              </button>
            </>
          ) : (
            "Loading catalog…"
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="modal-body marketplace-body">
      {/* Search + refresh header */}
      <div className="marketplace-header">
        <div className="search-box">
          <span className="icon">🔎</span>
          <input
            type="text"
            value={searchText}
            onChange={(e) => setSearchText(e.currentTarget.value)}
            placeholder="Search servers, tags, publishers…"
            spellCheck={false}
          />
          {searchText && (
            <button
              className="ghost clear"
              onClick={() => setSearchText("")}
              aria-label="Clear search"
            >
              ✕
            </button>
          )}
        </div>
        <button
          className="icon"
          onClick={onRefresh}
          title="Refresh from GitHub"
          disabled={isRefreshing}
        >
          {isRefreshing ? "…" : "↻"}
        </button>
        <span
          className="count-pill"
          style={{ color: "var(--blue)", background: "rgba(0, 180, 255, 0.12)" }}
        >
          {catalog.servers.length}
        </span>
      </div>

      <div className="marketplace-columns">
        {/* Category sidebar */}
        <div className="marketplace-categories">
          <CategoryRow
            label="All"
            iconName="grid"
            count={catalog.servers.length}
            isSelected={selectedCategoryId === null}
            onClick={() => setSelectedCategoryId(null)}
          />
          {categoriesWithCounts.map(({ category, count }) => (
            <CategoryRow
              key={category.id}
              label={category.label}
              iconName={category.icon ?? "dot"}
              count={count}
              isSelected={selectedCategoryId === category.id}
              onClick={() => setSelectedCategoryId(category.id)}
            />
          ))}

          {/* Feed manager */}
          <div className="feed-manager">
            <div className="feed-manager-header">
              <span className="feed-manager-title">Feeds</span>
            </div>

            <button
              className="feed-add-button"
              onClick={() => setShowFeedModal(true)}
            >
              + Add Custom Feed
            </button>

            {feeds.map((feed) => {
              const status = feedStatusMap.get(feed.id);
              const hasError = status?.error && !status.usingCache;
              const degraded = status?.error && status.usingCache;
              return (
                <div key={feed.id} className="feed-row">
                  <span
                    className="feed-status-dot"
                    title={
                      hasError
                        ? "Unreachable"
                        : degraded
                          ? "Using cache"
                          : feed.enabled
                            ? "Active"
                            : "Disabled"
                    }
                    style={{
                      background: hasError
                        ? "var(--red)"
                        : degraded
                          ? "var(--amber)"
                          : feed.enabled
                            ? "var(--green)"
                            : "var(--text-quaternary)",
                    }}
                  />
                  <span className="feed-label" title={feed.url}>
                    {feed.label}
                  </span>
                  {status && status.serverCount > 0 && (
                    <span className="feed-count">{status.serverCount}</span>
                  )}
                  <button
                    className="ghost small feed-toggle"
                    onClick={() => onToggleFeed(feed.id, !feed.enabled)}
                    title={feed.enabled ? "Disable" : "Enable"}
                  >
                    {feed.enabled ? "on" : "off"}
                  </button>
                  <button
                    className="ghost small feed-remove"
                    onClick={() => onRemoveFeed(feed.id)}
                    title="Remove feed"
                  >
                    ✕
                  </button>
                </div>
              );
            })}
          </div>
        </div>

        {/* Server list */}
        <div className="marketplace-servers">
          {filteredServers.length === 0 ? (
            <div className="empty">No servers match your filters</div>
          ) : (
            filteredServers.map((server) => (
              <ServerRow
                key={server.id}
                server={server}
                isSelected={expandedServerId === server.id}
                isInstalled={isInstalled(server)}
                runtimeStatus={runtimeStatus}
                editedJson={editedJson}
                editError={editError}
                busy={busyInstalling}
                onToggle={() => handleToggleExpand(server)}
                onEdit={setEditedJson}
                onReset={() => handleResetEdit(server)}
                onInstall={() => handleInstall(server)}
              />
            ))
          )}
        </div>
      </div>

      {showFeedModal && (
        <AddFeedModal
          onClose={() => setShowFeedModal(false)}
          onAdd={onAddFeed}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Add Feed modal
// ---------------------------------------------------------------------------

function AddFeedModal({
  onClose,
  onAdd,
}: {
  onClose: () => void;
  onAdd: (label: string, url: string) => Promise<void>;
}) {
  const [label, setLabel] = useState("");
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit() {
    if (!label.trim() || !url.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await onAdd(label.trim(), url.trim());
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div
        className="modal feed-modal"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <h3>Add Custom Feed</h3>
          <button className="ghost" onClick={onClose}>
            ✕
          </button>
        </div>
        <div className="modal-body feed-modal-body">
          <p className="feed-modal-hint">
            Point to a JSON catalog hosted on your network, a GitHub fork,
            or any URL that serves the same schema as the official catalog.
          </p>
          <label className="feed-modal-label">
            Label
            <input
              type="text"
              value={label}
              onChange={(e) => setLabel(e.currentTarget.value)}
              placeholder="My team catalog"
              disabled={busy}
              autoFocus
            />
          </label>
          <label className="feed-modal-label">
            Feed URL
            <input
              type="url"
              value={url}
              onChange={(e) => setUrl(e.currentTarget.value)}
              placeholder="https://example.com/catalog.json"
              disabled={busy}
            />
          </label>
          {error && <div className="banner error">{error}</div>}
        </div>
        <div className="modal-footer">
          <button className="ghost" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button
            className="primary"
            onClick={handleSubmit}
            disabled={busy || !label.trim() || !url.trim()}
          >
            {busy ? "Adding…" : "Add Feed"}
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Category row
// ---------------------------------------------------------------------------

interface CategoryRowProps {
  label: string;
  /// Lucide-style icon name from the catalog (`sparkles`, `code`, ...).
  /// Falls back to a neutral dot if we don't have a mapping.
  iconName: string;
  count: number;
  isSelected: boolean;
  onClick: () => void;
}

function CategoryRow({
  label,
  iconName,
  count,
  isSelected,
  onClick,
}: CategoryRowProps) {
  return (
    <button
      className={`category-row ${isSelected ? "selected" : ""}`}
      onClick={onClick}
    >
      <span className="icon" aria-hidden="true">
        <CategoryIcon name={iconName} />
      </span>
      <span className="label">{label}</span>
      <span className="count">{count}</span>
    </button>
  );
}

// ---------------------------------------------------------------------------
// Category icon — inline SVG map
//
// The catalog names icons using Lucide conventions (`sparkles`, `code`,
// `database`, …). Rather than pulling in lucide-react as a dependency we
// hand-roll the paths we need. Stroke width and viewBox are chosen to match
// Lucide's defaults so the visual weight lines up with the rest of the UI.
// Missing names render as a small dot so an unknown category still has
// *something* visible rather than an empty slot.
// ---------------------------------------------------------------------------

const CATEGORY_ICON_PATHS: Record<string, ReactNode> = {
  grid: (
    <>
      <rect x="3" y="3" width="7" height="7" rx="1" />
      <rect x="14" y="3" width="7" height="7" rx="1" />
      <rect x="3" y="14" width="7" height="7" rx="1" />
      <rect x="14" y="14" width="7" height="7" rx="1" />
    </>
  ),
  sparkles: (
    <>
      <path d="M12 3l1.7 4.8L18.5 9.5l-4.8 1.7L12 16l-1.7-4.8L5.5 9.5l4.8-1.7L12 3z" />
      <path d="M19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8L19 15z" />
    </>
  ),
  code: (
    <>
      <path d="M16 18l6-6-6-6" />
      <path d="M8 6l-6 6 6 6" />
    </>
  ),
  database: (
    <>
      <ellipse cx="12" cy="5" rx="8" ry="3" />
      <path d="M4 5v6c0 1.7 3.6 3 8 3s8-1.3 8-3V5" />
      <path d="M4 11v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6" />
    </>
  ),
  cloud: (
    <>
      <path d="M17.5 19a4.5 4.5 0 1 0-1.4-8.8A6 6 0 1 0 6.5 19h11z" />
    </>
  ),
  globe: (
    <>
      <circle cx="12" cy="12" r="9" />
      <path d="M3 12h18" />
      <path d="M12 3a14 14 0 0 1 0 18" />
      <path d="M12 3a14 14 0 0 0 0 18" />
    </>
  ),
  search: (
    <>
      <circle cx="11" cy="11" r="7" />
      <path d="M21 21l-4.3-4.3" />
    </>
  ),
  briefcase: (
    <>
      <rect x="3" y="7" width="18" height="13" rx="2" />
      <path d="M9 7V5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2" />
      <path d="M3 13h18" />
    </>
  ),
  "message-circle": (
    <>
      <path d="M21 12a9 9 0 1 1-3.6-7.2L21 3l-1.2 4.4A9 9 0 0 1 21 12z" />
    </>
  ),
  palette: (
    <>
      <path d="M12 3a9 9 0 1 0 0 18 1.8 1.8 0 0 0 1.3-3.1 1.8 1.8 0 0 1 1.3-3.1H17a4 4 0 0 0 4-4c0-5-4-7.8-9-7.8z" />
      <circle cx="7.5" cy="11" r="1" fill="currentColor" />
      <circle cx="10.5" cy="7" r="1" fill="currentColor" />
      <circle cx="15" cy="7" r="1" fill="currentColor" />
      <circle cx="17.5" cy="11" r="1" fill="currentColor" />
    </>
  ),
  activity: (
    <>
      <path d="M3 12h4l3-8 4 16 3-8h4" />
    </>
  ),
  "dollar-sign": (
    <>
      <path d="M12 2v20" />
      <path d="M17 7H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
    </>
  ),
  cpu: (
    <>
      <rect x="5" y="5" width="14" height="14" rx="2" />
      <rect x="9" y="9" width="6" height="6" rx="1" />
      <path d="M9 2v3M15 2v3M9 19v3M15 19v3M2 9h3M2 15h3M19 9h3M19 15h3" />
    </>
  ),
  music: (
    <>
      <path d="M9 18V5l12-2v13" />
      <circle cx="6" cy="18" r="3" />
      <circle cx="18" cy="16" r="3" />
    </>
  ),
  users: (
    <>
      <path d="M17 20v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
      <circle cx="9.5" cy="7" r="4" />
      <path d="M22 20v-2a4 4 0 0 0-3-3.9" />
      <path d="M16 3.1a4 4 0 0 1 0 7.8" />
    </>
  ),
  folder: (
    <>
      <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
    </>
  ),
  dot: <circle cx="12" cy="12" r="3" fill="currentColor" />,
};

function CategoryIcon({ name }: { name: string }) {
  const contents = CATEGORY_ICON_PATHS[name] ?? CATEGORY_ICON_PATHS.dot;
  return (
    <svg
      viewBox="0 0 24 24"
      width="15"
      height="15"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {contents}
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Server row — header + expandable detail block
// ---------------------------------------------------------------------------

interface ServerRowProps {
  server: CatalogServer;
  isSelected: boolean;
  isInstalled: boolean;
  runtimeStatus: RuntimeStatus | null;
  editedJson: string;
  editError: string | null;
  busy: boolean;
  onToggle: () => void;
  onEdit: (val: string) => void;
  onReset: () => void;
  onInstall: () => void;
}

interface MissingRuntime {
  label: string;
  downloadUrl: string | null;
}

const RUNTIME_DOWNLOADS: Record<string, string> = {
  "Node.js": "https://nodejs.org/",
  Python: "https://www.python.org/downloads/",
  uv: "https://docs.astral.sh/uv/getting-started/installation/",
  Docker: "https://www.docker.com/products/docker-desktop/",
};

/// Check which of a server's requirements are missing from the user's machine.
function missingRequirements(
  requirements: string[],
  rt: RuntimeStatus | null
): MissingRuntime[] {
  if (!rt || requirements.length === 0) return [];
  const missing: MissingRuntime[] = [];
  for (const req of requirements) {
    const key = req.toLowerCase();
    if (key === "node" && !rt.node)
      missing.push({ label: "Node.js", downloadUrl: RUNTIME_DOWNLOADS["Node.js"] });
    else if (key === "python" && !rt.python)
      missing.push({ label: "Python", downloadUrl: RUNTIME_DOWNLOADS["Python"] });
    else if (key === "uv" && !rt.uv)
      missing.push({ label: "uv", downloadUrl: RUNTIME_DOWNLOADS["uv"] });
    else if (key === "docker" && !rt.docker)
      missing.push({ label: "Docker", downloadUrl: RUNTIME_DOWNLOADS["Docker"] });
  }
  return missing;
}

function ServerRow({
  server,
  isSelected,
  isInstalled,
  runtimeStatus,
  editedJson,
  editError,
  busy,
  onToggle,
  onEdit,
  onReset,
  onInstall,
}: ServerRowProps) {
  const publisherAccent =
    server.publisher.type === "official"
      ? "var(--green)"
      : server.publisher.type === "vendor"
        ? "var(--blue)"
        : "var(--purple)";

  const requiredCount = (server.envVars ?? []).filter((v) => v.required).length;

  return (
    <div className={`server-card ${isSelected ? "selected" : ""}`}>
      <div
        className="server-card-header"
        onClick={onToggle}
        role="button"
        tabIndex={0}
      >
        <span
          className="glow-dot"
          style={{
            background: publisherAccent,
            boxShadow: `0 0 6px ${publisherAccent}`,
          }}
        />
        <div className="labels">
          <div className="title-row">
            <span className="name">{server.name}</span>
            {server.publisher.verified && (
              <span className="verified" title="Verified publisher">
                ✓
              </span>
            )}
            <span className="publisher">{server.publisher.name}</span>
            {requiredCount > 0 && (
              <span className="keys-pill" title="Required env vars">
                🔑 {requiredCount} key{requiredCount === 1 ? "" : "s"}
              </span>
            )}
            {server.feedOrigin && server.feedOrigin !== "built-in" && (
              <span className="feed-origin-pill" title={`From feed: ${server.feedOrigin}`}>
                {server.feedOrigin}
              </span>
            )}
          </div>
          <div className="description">{server.description}</div>
        </div>
        {isInstalled ? (
          <span className="installed-pill">✓ Installed</span>
        ) : (
          <span className="chevron">{isSelected ? "▴" : "▾"}</span>
        )}
      </div>

      {isSelected && !isInstalled && (
        <div className="server-card-detail">
          {server.setupNotes && (
            <div className="setup-notes">{server.setupNotes}</div>
          )}

          {(() => {
            const missing = missingRequirements(server.requirements, runtimeStatus);
            if (missing.length === 0) return null;
            return (
              <div className="banner warning">
                <strong>Missing:</strong>{" "}
                {missing.map((m, i) => (
                  <span key={m.label}>
                    {i > 0 && ", "}
                    {m.downloadUrl ? (
                      <a href={m.downloadUrl} target="_blank" rel="noreferrer noopener">
                        {m.label}
                      </a>
                    ) : (
                      m.label
                    )}
                  </span>
                ))}
                {" "}— this server requires{" "}
                {missing.length === 1 ? "it" : "them"} to run.
                You can still install it, but it won't start until{" "}
                {missing.length === 1 ? "it's" : "they're"} available on
                your PATH.
              </div>
            );
          })()}

          {(server.envVars?.filter((v) => v.required).length ?? 0) > 0 && (
            <div className="env-vars-required">
              <div className="required-label">Required</div>
              {server.envVars!
                .filter((v) => v.required)
                .map((env) => (
                  <div key={env.name} className="env-var-row">
                    <span className="key-icon">{env.secret ? "🔑" : "●"}</span>
                    <code>{env.name}</code>
                    {env.description && (
                      <span className="env-var-desc">— {env.description}</span>
                    )}
                    {env.helpUrl && (
                      <a
                        href={env.helpUrl}
                        target="_blank"
                        rel="noreferrer noopener"
                      >
                        ↗
                      </a>
                    )}
                  </div>
                ))}
            </div>
          )}

          <div className="editor-label-row">
            <span className="editor-label">Config JSON</span>
            <span className="editor-hint">— edit then save to install</span>
            <span className="spacer" />
            <button className="ghost small" onClick={onReset}>
              ↺ Reset
            </button>
          </div>

          <textarea
            className="marketplace-editor"
            value={editedJson}
            onChange={(e) => onEdit(e.currentTarget.value)}
            spellCheck={false}
          />

          {editError && <div className="banner error">{editError}</div>}

          <div className="detail-footer">
            {server.homepage && (
              <a
                className="link-chip"
                href={server.homepage}
                target="_blank"
                rel="noreferrer noopener"
              >
                Homepage
              </a>
            )}
            {server.repository && (
              <a
                className="link-chip"
                href={server.repository}
                target="_blank"
                rel="noreferrer noopener"
              >
                Repo
              </a>
            )}
            {server.license && (
              <span className="license-chip">{server.license}</span>
            )}
            <span className="spacer" />
            <button
              className="primary"
              onClick={onInstall}
              disabled={busy}
            >
              {busy ? "Installing…" : "Save to list"}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// JSON snippet helpers — ported from the Swift version to keep editor UX
// identical: the default template is the pretty-printed keyed wrapper form
// with every declared env var pre-injected as an empty slot.
// ---------------------------------------------------------------------------

/// Pretty-printed keyed snippet for a catalog server:
///
///   {
///     "github": {
///       "command": "docker",
///       "args": [ ... ],
///       "env": { "GITHUB_PERSONAL_ACCESS_TOKEN": "" }
///     }
///   }
function defaultSnippet(server: CatalogServer): string {
  const dict = configToDict(server);
  mergeEnvVarPlaceholders(dict, server);
  const wrapped: Record<string, unknown> = { [server.id]: dict };
  try {
    return JSON.stringify(wrapped, null, 2);
  } catch {
    return "{}";
  }
}

/// Mirror of Rust's CatalogConfig::to_config_dict — only includes keys that
/// the catalog author actually set.
function configToDict(server: CatalogServer): Record<string, unknown> {
  const c = server.config;
  const dict: Record<string, unknown> = {};
  if (c.url != null) {
    // HTTP transport.
    dict.url = c.url;
    if (c.headers) dict.headers = { ...c.headers };
  } else {
    // Stdio transport.
    if (c.command != null) dict.command = c.command;
    if (c.args && c.args.length > 0) dict.args = [...c.args];
    if (c.env) dict.env = { ...c.env };
  }
  return dict;
}

/// Mutate a config dict so that every env var the catalog declares has a
/// corresponding slot the user can edit. Stdio → env dict; HTTP is left
/// alone since we can't guess which header maps to which env var.
function mergeEnvVarPlaceholders(
  dict: Record<string, unknown>,
  server: CatalogServer
): void {
  const declared = server.envVars ?? [];
  if (declared.length === 0) return;
  if (dict.command !== undefined) {
    const envBlock: Record<string, string> = {
      ...((dict.env as Record<string, string>) ?? {}),
    };
    // Required first, then optional — matches the Swift ordering.
    for (const v of declared) {
      if (envBlock[v.name] === undefined) {
        envBlock[v.name] = v.placeholder ?? "";
      }
    }
    if (Object.keys(envBlock).length > 0) {
      dict.env = envBlock;
    }
  }
}

/// Parse the editor's current text. Accepts both the keyed wrapper form
/// (`{"name": {config}}`) and a raw config dict. Returns `{name, dict}` on
/// success, `null` if the JSON is invalid or doesn't look like an MCP server.
function parseEditedSnippet(
  text: string,
  fallbackName: string
): { name: string; dict: Record<string, unknown> } | null {
  const trimmed = text.trim();
  if (!trimmed) return null;
  let obj: unknown;
  try {
    obj = JSON.parse(trimmed);
  } catch {
    return null;
  }
  if (typeof obj !== "object" || obj === null || Array.isArray(obj)) return null;
  const record = obj as Record<string, unknown>;

  // Keyed wrapper: exactly one top-level key whose value is a config dict.
  const keys = Object.keys(record);
  if (keys.length === 1) {
    const k = keys[0];
    const inner = record[k];
    if (
      typeof inner === "object" &&
      inner !== null &&
      !Array.isArray(inner)
    ) {
      const innerRec = inner as Record<string, unknown>;
      if (innerRec.command !== undefined || innerRec.url !== undefined) {
        const cleanName = k.trim();
        return {
          name: cleanName || fallbackName,
          dict: innerRec,
        };
      }
    }
  }

  // Bare config dict (user stripped the outer key).
  if (record.command !== undefined || record.url !== undefined) {
    return { name: fallbackName, dict: record };
  }

  return null;
}

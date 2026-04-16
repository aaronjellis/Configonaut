// Add Server modal — two tabs matching the Swift version:
//
//   • Marketplace — browse the catalog, pick a server, edit its template,
//                   click "Save to list" to install into the Inactive column.
//   • Paste JSON  — the old free-form entry flow for power users who already
//                   have a config snippet.
//
// Marketplace is the default tab because the primary use case is discovery,
// not pasting. Users who know exactly what they want can still hit "Paste JSON"
// without any extra clicks.
//
// The catalog is loaded on mount via `getCatalog` (cache-first, instant) and
// then a background `refreshCatalog` is fired to pull the latest from GitHub
// — failures there are silent because the user already has a usable catalog.

import { useEffect, useMemo, useState } from "react";
import {
  addFeed,
  checkRuntime,
  getCatalogLinks,
  getCatalogWithFeeds,
  installFromCatalog,
  listFeeds,
  parseServerInput,
  refreshAllFeeds,
  removeFeed,
  toggleFeed,
} from "../api";
import { validatePasteInput } from "../lib/validateServerJson";
import type {
  AppMode,
  Catalog,
  CatalogServer,
  FeedEntry,
  FeedStatus,
  CatalogRuntimeStatus,
  ServerTuple,
} from "../types";
import { MarketplaceTab } from "./MarketplaceTab";
import { SetupStep } from "./SetupStep";
import { useToast } from "./Toast";

interface Props {
  mode: AppMode;
  onClose: () => void;
  /// Used by the "Paste JSON" tab — the parent runs the actual add_to_active /
  /// add_to_stored command and refreshes its lists.
  onCommit: (entries: ServerTuple[], target: "active" | "stored") => void;
  /// Called after the Marketplace tab installs a server. The Marketplace flow
  /// writes directly to disk (so it can record the catalog link), so the parent
  /// only needs to refresh its lists — no entries to pass back.
  onCatalogInstalled: (installedName: string) => void;
}

type Tab = "marketplace" | "paste";

export function AddServerModal({
  mode,
  onClose,
  onCommit,
  onCatalogInstalled,
}: Props) {
  const toast = useToast();
  const [tab, setTab] = useState<Tab>("marketplace");

  // When set, the modal shows the guided SetupStep for that server id
  // instead of the tabs. Used for catalog 1.1.0 entries with prerequisites
  // or configFields. Null means "show tabs as usual".
  const [setupServerId, setSetupServerId] = useState<string | null>(null);

  // Catalog state — shared across tabs so switching back and forth doesn't
  // re-fetch. Once loaded, the Paste JSON tab can still read it for the
  // "installed?" badge if we ever want to show one there.
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [catalogError, setCatalogError] = useState<string | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [links, setLinks] = useState<Record<string, string>>({});
  const [runtimeStatus, setRuntimeStatus] = useState<CatalogRuntimeStatus | null>(null);
  const [feeds, setFeeds] = useState<FeedEntry[]>([]);
  const [feedStatuses, setFeedStatuses] = useState<FeedStatus[]>([]);

  // Bootstrap on mount. getCatalogWithFeeds is instant (caches + baseline)
  // so we show the UI immediately, then refreshAllFeeds in the background.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [[cat, statuses], lnk, feedList] = await Promise.all([
          getCatalogWithFeeds(),
          getCatalogLinks(mode),
          listFeeds(),
        ]);
        if (cancelled) return;
        setCatalog(cat);
        setFeedStatuses(statuses);
        setFeeds(feedList);
        setLinks(lnk);
      } catch (e) {
        if (!cancelled) setCatalogError(String(e));
      }

      // Runtime detection + feed refresh in parallel.
      const runtimePromise = checkRuntime()
        .then((rt) => { if (!cancelled) setRuntimeStatus(rt); })
        .catch(() => {});

      const refreshPromise = (async () => {
        try {
          setIsRefreshing(true);
          const [fresh, statuses] = await refreshAllFeeds();
          if (!cancelled) {
            setCatalog(fresh);
            setFeedStatuses(statuses);
          }
        } catch {
          // Offline is fine, we have cached feeds + baseline.
        } finally {
          if (!cancelled) setIsRefreshing(false);
        }
      })();

      await Promise.all([runtimePromise, refreshPromise]);
    })();
    return () => {
      cancelled = true;
    };
  }, [mode]);

  // Show toast for feeds that fell back to cache.
  useEffect(() => {
    const degraded = feedStatuses.filter((s) => s.error && s.usingCache);
    if (degraded.length > 0) {
      toast.show(
        `${degraded.length} feed(s) unreachable \u2014 using cached versions.`,
        "warning"
      );
    }
  }, [feedStatuses, toast]);

  async function handleMarketplaceInstall(
    server: CatalogServer,
    customConfig: Record<string, unknown>,
    customName: string
  ) {
    // New-style catalog entries (1.1.0+) with prerequisites or configFields
    // route through the guided SetupStep. This gives the user a chance to
    // satisfy runtime prerequisites and fill per-field configuration before
    // the install fires.
    const hasNewSchema =
      (server.prerequisites && server.prerequisites.length > 0) ||
      (server.configFields && server.configFields.length > 0);
    if (hasNewSchema) {
      setSetupServerId(server.id);
      return;
    }

    // Legacy flow: install directly via installFromCatalog.
    // Servers with required env vars are parked inactive until the user
    // fills in the secrets. Everything else goes straight to active.
    const needsSetup = (server.envVars ?? []).some((v) => v.required);
    const target = needsSetup ? "stored" : "active";
    const installedName = await installFromCatalog(
      mode,
      server.id,
      target,
      customConfig,
      customName
    );
    // Update the local "installed" map so the row flips to the checkmark
    // immediately without a round trip.
    setLinks((prev) => ({ ...prev, [installedName]: server.id }));
    onCatalogInstalled(installedName);
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div
        className="modal modal-wide"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          {setupServerId ? (
            <>
              <h2 className="modal-title">Install MCP Server</h2>
              <button className="ghost" onClick={onClose} aria-label="Close">
                ✕
              </button>
            </>
          ) : (
            <>
              <div className="modal-tabs" role="tablist">
                <button
                  role="tab"
                  aria-selected={tab === "marketplace"}
                  className={tab === "marketplace" ? "active" : ""}
                  onClick={() => setTab("marketplace")}
                >
                  Marketplace
                </button>
                <button
                  role="tab"
                  aria-selected={tab === "paste"}
                  className={tab === "paste" ? "active" : ""}
                  onClick={() => setTab("paste")}
                >
                  Add manually
                </button>
              </div>
              <button className="ghost" onClick={onClose} aria-label="Close">
                ✕
              </button>
            </>
          )}
        </div>

        {setupServerId ? (
          <div className="modal-body">
            <SetupStep
              serverId={setupServerId}
              onDone={() => {
                // Reflect the newly-installed server in the parent list.
                // The catalog id doubles as the installed name by convention,
                // so we update the links map and notify the parent.
                setLinks((prev) => ({
                  ...prev,
                  [setupServerId]: setupServerId,
                }));
                onCatalogInstalled(setupServerId);
                setSetupServerId(null);
                onClose();
              }}
              onCancel={() => setSetupServerId(null)}
            />
          </div>
        ) : tab === "marketplace" ? (
          <MarketplaceTab
            catalog={catalog}
            catalogError={catalogError}
            isRefreshing={isRefreshing}
            links={links}
            runtimeStatus={runtimeStatus}
            feeds={feeds}
            feedStatuses={feedStatuses}
            onRefresh={async () => {
              setIsRefreshing(true);
              try {
                const [fresh, statuses] = await refreshAllFeeds();
                setCatalog(fresh);
                setFeedStatuses(statuses);
                setCatalogError(null);
                toast.show("Catalog refreshed.", "success");
              } catch (e) {
                setCatalogError(String(e));
                toast.show("Catalog refresh failed.", "error");
              } finally {
                setIsRefreshing(false);
              }
            }}
            onInstall={handleMarketplaceInstall}
            onAddFeed={async (label, url) => {
              await addFeed(label, url);
              setFeeds(await listFeeds());
              // Refresh to fetch the new feed's catalog.
              setIsRefreshing(true);
              try {
                const [fresh, statuses] = await refreshAllFeeds();
                setCatalog(fresh);
                setFeedStatuses(statuses);
              } finally {
                setIsRefreshing(false);
              }
            }}
            onRemoveFeed={async (feedId) => {
              await removeFeed(feedId);
              setFeeds(await listFeeds());
              const [fresh, statuses] = await refreshAllFeeds();
              setCatalog(fresh);
              setFeedStatuses(statuses);
            }}
            onToggleFeed={async (feedId, enabled) => {
              await toggleFeed(feedId, enabled);
              setFeeds(await listFeeds());
              const [fresh, statuses] = await refreshAllFeeds();
              setCatalog(fresh);
              setFeedStatuses(statuses);
            }}
          />
        ) : (
          <PasteTab onClose={onClose} onCommit={onCommit} />
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Paste JSON tab — the old manual-entry flow, now scoped inside a tab.
// ---------------------------------------------------------------------------

interface PasteProps {
  onClose: () => void;
  onCommit: (entries: ServerTuple[], target: "active" | "stored") => void;
}

function PasteTab({ onClose, onCommit }: PasteProps) {
  const [raw, setRaw] = useState("");
  const [name, setName] = useState("");
  const [saveError, setSaveError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Realtime mirror of the Rust `parse_server_input` rules. We hide the error
  // while the textarea is empty so a fresh modal doesn't scream at the user,
  // but any keystroke after that gets live feedback — matching the detail
  // editor on the MCP Servers view.
  const pasteError = useMemo(() => {
    if (!raw.trim()) return null;
    return validatePasteInput(raw, name);
  }, [raw, name]);

  // Banner precedence: realtime parse errors win over save-time backend
  // errors, since any backend failure is stale the moment the user edits.
  const displayedError = pasteError ?? saveError;

  async function handleSubmit(target: "active" | "stored") {
    setSaveError(null);
    setBusy(true);
    try {
      const entries = await parseServerInput(raw, name || undefined);
      if (entries.length === 0) {
        throw new Error("no servers found in input");
      }
      onCommit(entries, target);
      onClose();
    } catch (e) {
      setSaveError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const canSubmit = !busy && !pasteError && raw.trim().length > 0;

  return (
    <>
      <div className="modal-body">
        <label htmlFor="server-name">
          Name (only needed for a single server body)
        </label>
        <input
          id="server-name"
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="e.g. filesystem"
        />

        <label htmlFor="server-json">JSON</label>
        <textarea
          id="server-json"
          value={raw}
          onChange={(e) => setRaw(e.currentTarget.value)}
          placeholder={`{\n  "mcpServers": {\n    "filesystem": {\n      "command": "npx",\n      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"]\n    }\n  }\n}`}
          spellCheck={false}
          className={pasteError ? "invalid" : ""}
        />

        {displayedError && (
          <div className="banner error">{displayedError}</div>
        )}
      </div>
      <div className="modal-footer">
        <button className="ghost" onClick={onClose} disabled={busy}>
          Cancel
        </button>
        <button
          onClick={() => handleSubmit("stored")}
          disabled={!canSubmit}
          title={pasteError ?? undefined}
        >
          Save to Inactive
        </button>
        <button
          className="primary"
          onClick={() => handleSubmit("active")}
          disabled={!canSubmit}
          title={pasteError ?? undefined}
        >
          Add to Active
        </button>
      </div>
    </>
  );
}


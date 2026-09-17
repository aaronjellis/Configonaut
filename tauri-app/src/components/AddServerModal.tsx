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
import {
  extractServerNames,
  hasRemoteEntries,
  normalizePasteInput,
  validatePasteInput,
} from "../lib/validateServerJson";
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

  // Pending catalog install that needs user confirmation (remote + Desktop).
  const [pendingCatalogInstall, setPendingCatalogInstall] = useState<{
    server: CatalogServer;
    customConfig: Record<string, unknown>;
    customName: string;
  } | null>(null);

  async function handleMarketplaceInstall(
    server: CatalogServer,
    customConfig: Record<string, unknown>,
    customName: string
  ) {
    // Warn if installing a remote server in Desktop mode.
    if (mode === "desktop" && server.transport === "remote") {
      setPendingCatalogInstall({ server, customConfig, customName });
      return;
    }
    await doMarketplaceInstall(server, customConfig, customName);
  }

  async function doMarketplaceInstall(
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

        {pendingCatalogInstall ? (
          <>
            <div className="modal-body">
              <div className="banner warning">
                <strong>Remote server in Desktop mode</strong>
                <p style={{ margin: "8px 0 0" }}>
                  <strong>{pendingCatalogInstall.server.name}</strong> is a
                  remote MCP server. Claude Desktop has a known issue where{" "}
                  <code>url</code>-based entries can cause it to remove all
                  your MCP servers on restart. Consider adding remote servers
                  through Claude Desktop's Settings &rarr; Integrations instead.
                </p>
              </div>
            </div>
            <div className="modal-footer">
              <button
                className="ghost"
                onClick={() => setPendingCatalogInstall(null)}
              >
                Go Back
              </button>
              <button
                className="danger"
                onClick={async () => {
                  const { server, customConfig, customName } =
                    pendingCatalogInstall;
                  setPendingCatalogInstall(null);
                  await doMarketplaceInstall(server, customConfig, customName);
                }}
              >
                Install Anyway
              </button>
            </div>
          </>
        ) : setupServerId ? (
          <div className="modal-body">
            <SetupStep
              mode={mode}
              serverId={setupServerId}
              onDone={(installedName) => {
                setLinks((prev) => ({ ...prev, [installedName]: setupServerId }));
                onCatalogInstalled(installedName);
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
            // Deliberately unguarded: AddFeedModal renders its own inline
            // error for add failures, so there's no separate toast to show
            // here (a refresh failure below is still reported).
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
              try {
                await removeFeed(feedId);
              } catch (e) {
                toast.show(`Couldn't remove feed: ${String(e)}`, "error");
                return;
              }
              try {
                setFeeds(await listFeeds());
                const [fresh, statuses] = await refreshAllFeeds();
                setCatalog(fresh);
                setFeedStatuses(statuses);
              } catch (e) {
                toast.show(`Feed removed, but the catalog didn't refresh: ${String(e)}`, "warning");
              }
            }}
            onToggleFeed={async (feedId, enabled) => {
              try {
                await toggleFeed(feedId, enabled);
              } catch (e) {
                toast.show(`Couldn't update feed: ${String(e)}`, "error");
                return;
              }
              try {
                setFeeds(await listFeeds());
                const [fresh, statuses] = await refreshAllFeeds();
                setCatalog(fresh);
                setFeedStatuses(statuses);
              } catch (e) {
                toast.show(`Feed updated, but the catalog didn't refresh: ${String(e)}`, "warning");
              }
            }}
          />
        ) : (
          <PasteTab mode={mode} onClose={onClose} onCommit={onCommit} />
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Paste JSON tab — the old manual-entry flow, now scoped inside a tab.
// ---------------------------------------------------------------------------

interface PasteProps {
  mode: AppMode;
  onClose: () => void;
  onCommit: (entries: ServerTuple[], target: "active" | "stored") => void;
}

function PasteTab({ mode, onClose, onCommit }: PasteProps) {
  const [raw, setRaw] = useState("");
  const [name, setName] = useState("");
  const [nameAutoFilled, setNameAutoFilled] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // Pending submission that needs user confirmation (e.g. remote server in Desktop mode).
  const [pendingConfirm, setPendingConfirm] = useState<{
    entries: ServerTuple[];
    target: "active" | "stored";
  } | null>(null);

  // Realtime mirror of the Rust `parse_server_input` rules. We hide the error
  // while the textarea is empty so a fresh modal doesn't scream at the user,
  // but any keystroke after that gets live feedback — matching the detail
  // editor on the MCP Servers view.
  const pasteError = useMemo(() => {
    if (!raw.trim()) return null;
    return validatePasteInput(raw, name);
  }, [raw, name]);

  // Auto-extract server name(s) from pasted JSON. When the input is a bare
  // map like `"click-insights": { ... }`, we can pull the name out and
  // populate the field automatically — no need for the user to type it.
  const detectedNames = useMemo(() => extractServerNames(raw), [raw]);

  // Auto-fill the name field when we detect exactly one server name and the
  // user hasn't manually edited the name.
  useEffect(() => {
    if (detectedNames.length === 1 && (name === "" || nameAutoFilled)) {
      setName(detectedNames[0]);
      setNameAutoFilled(true);
    }
  }, [detectedNames, name, nameAutoFilled]);

  // Banner precedence: realtime parse errors win over save-time backend
  // errors, since any backend failure is stale the moment the user edits.
  const displayedError = pasteError ?? saveError;

  async function handleSubmit(target: "active" | "stored") {
    setSaveError(null);
    setBusy(true);
    try {
      const normalized = normalizePasteInput(raw);
      const entries = await parseServerInput(normalized, name || undefined);
      if (entries.length === 0) {
        throw new Error("no servers found in input");
      }
      // Warn if adding remote (url-based) servers to Desktop mode config.
      if (mode === "desktop" && target === "active" && hasRemoteEntries(entries)) {
        setPendingConfirm({ entries, target });
        setBusy(false);
        return;
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

  // Show the name field only when the input looks like a single server body
  // (no name embedded in the JSON). For bare maps and mcpServers wrappers,
  // the name is in the JSON itself.
  const needsName = useMemo(() => {
    const trimmed = raw.trim();
    if (!trimmed) return true;
    const normalized = normalizePasteInput(trimmed);
    try {
      const parsed = JSON.parse(normalized);
      if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
        return true;
      }
      return "command" in parsed || "url" in parsed || "type" in parsed;
    } catch {
      return true;
    }
  }, [raw]);

  if (pendingConfirm) {
    return (
      <>
        <div className="modal-body">
          <div className="banner warning">
            <strong>Remote server in Desktop mode</strong>
            <p style={{ margin: "8px 0 0" }}>
              Claude Desktop has a known issue where <code>url</code>-based
              server entries can cause it to remove all your MCP servers on
              restart. Consider adding remote servers through Claude Desktop's
              Settings &rarr; Integrations instead.
            </p>
          </div>
        </div>
        <div className="modal-footer">
          <button
            className="ghost"
            onClick={() => setPendingConfirm(null)}
          >
            Go Back
          </button>
          <button
            className="danger"
            onClick={() => {
              onCommit(pendingConfirm.entries, pendingConfirm.target);
              onClose();
            }}
          >
            Install Anyway
          </button>
        </div>
      </>
    );
  }

  return (
    <>
      <div className="modal-body">
        {needsName && (
          <>
            <label htmlFor="server-name">
              Name
            </label>
            <input
              id="server-name"
              value={name}
              onChange={(e) => {
                setName(e.currentTarget.value);
                setNameAutoFilled(false);
              }}
              placeholder="e.g. filesystem"
            />
          </>
        )}

        <label htmlFor="server-json">JSON</label>
        <textarea
          id="server-json"
          value={raw}
          onChange={(e) => setRaw(e.currentTarget.value)}
          placeholder={`Paste any format:\n\n  "server-name": { "command": "npx", ... }\n\n  { "mcpServers": { "name": { ... } } }\n\n  { "command": "npx", "args": [...] }`}
          spellCheck={false}
          className={pasteError ? "invalid" : ""}
        />

        {detectedNames.length > 0 && !pasteError && (
          <div className="banner success">
            Detected: {detectedNames.join(", ")}
          </div>
        )}

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


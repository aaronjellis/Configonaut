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
  getCatalog,
  getCatalogLinks,
  installFromCatalog,
  parseServerInput,
  refreshCatalog,
} from "../api";
import { validatePasteInput } from "../lib/validateServerJson";
import type {
  AppMode,
  Catalog,
  CatalogServer,
  ServerTuple,
} from "../types";
import { MarketplaceTab } from "./MarketplaceTab";

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
  const [tab, setTab] = useState<Tab>("marketplace");

  // Catalog state — shared across tabs so switching back and forth doesn't
  // re-fetch. Once loaded, the Paste JSON tab can still read it for the
  // "installed?" badge if we ever want to show one there.
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [catalogError, setCatalogError] = useState<string | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [links, setLinks] = useState<Record<string, string>>({});

  // Bootstrap on mount. `getCatalog` is instant (cache or embedded baseline)
  // so we show the UI immediately, then hit `refreshCatalog` in the background.
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [cat, lnk] = await Promise.all([
          getCatalog(),
          getCatalogLinks(mode),
        ]);
        if (cancelled) return;
        setCatalog(cat);
        setLinks(lnk);
      } catch (e) {
        if (!cancelled) setCatalogError(String(e));
      }

      // Background refresh — don't block the UI or surface transient errors,
      // but update the list if a newer catalog comes back.
      try {
        setIsRefreshing(true);
        const fresh = await refreshCatalog();
        if (!cancelled) setCatalog(fresh);
      } catch {
        // Swallow — offline or rate-limited is fine, we have a baseline.
      } finally {
        if (!cancelled) setIsRefreshing(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [mode]);

  async function handleMarketplaceInstall(
    server: CatalogServer,
    customConfig: Record<string, unknown>,
    customName: string
  ) {
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
        </div>

        {tab === "marketplace" ? (
          <MarketplaceTab
            catalog={catalog}
            catalogError={catalogError}
            isRefreshing={isRefreshing}
            links={links}
            onRefresh={async () => {
              setIsRefreshing(true);
              try {
                const fresh = await refreshCatalog();
                setCatalog(fresh);
                setCatalogError(null);
              } catch (e) {
                setCatalogError(String(e));
              } finally {
                setIsRefreshing(false);
              }
            }}
            onInstall={handleMarketplaceInstall}
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


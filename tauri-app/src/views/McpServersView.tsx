// MCP Servers view — ported from Sources/MCPView.swift.
//
// Layout:
//   ┌──────────────────────────────────────────────┐
//   │ MCP Servers  (N)       ~/Library/…/config    │
//   │                               [+ Add Server] │
//   ├──────────────────────────────────────────────┤
//   │ ┌─── Active (green) ──┬─── Inactive (red) ──┐│
//   │ │ Running in Claude…  │ Saved for later…    ││
//   │ │ rows                │ rows                ││
//   │ └─────────────────────┴─────────────────────┘│
//   │ ┌───── Detail panel (editable JSON) ───────┐│
//   │ └───────────────────────────────────────────┘│
//   │ ● status message          N active, M inactive│
//   └──────────────────────────────────────────────┘
//
// Drag-and-drop between columns is implemented with Pointer Events instead
// of HTML5 drag/drop. Pointer Events give us full control over the drag
// preview and placeholder, which HTML5 DnD doesn't really support in a
// Tauri webview (the native drag image is janky and cross-webview custom
// MIME tricks are flaky). The flow:
//
//   1. pointerdown on a row  → capture the pointer, snapshot the row's
//      bounding rect, stash the server name + source
//   2. pointermove (past a 4px threshold) → enter "dragging" state:
//      show a floating preview under the cursor and render a placeholder
//      gap in the target column at the row the cursor is closest to
//   3. pointerup → if the cursor is in the *other* column, move the server;
//      either way, tear down the overlay

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  addServersToActive,
  addServersToStored,
  deleteServer as apiDeleteServer,
  listServers,
  moveServerToActive,
  moveServerToStored,
  restartClaudeDesktop,
  updateServerConfig,
} from "../api";
import { AddServerModal } from "../components/AddServerModal";
import { displayPath } from "../lib/displayPath";
import { validateServerConfigJson } from "../lib/validateServerJson";
import type {
  AppMode,
  ServerEntry,
  ServerListing,
  ServerSource,
  ServerTuple,
} from "../types";

interface Props {
  mode: AppMode;
  /// Called after any successful mutation so the shell can refresh
  /// badge counts. We don't use it for local state.
  onMutated: () => void;
}

interface Selection {
  source: ServerSource;
  name: string;
}

/// State the drag overlay needs to render. Lives in McpServersView so both
/// columns can read it (one will render the placeholder, the other the
/// "ghost" for the origin row).
interface DragState {
  name: string;
  source: ServerSource;
  /// Where the pointer currently is, in viewport coords. Used to position
  /// the floating preview card.
  pointerX: number;
  pointerY: number;
  /// Width/height of the preview, captured from the origin row so the
  /// preview matches it.
  previewWidth: number;
  previewHeight: number;
  /// Offset from the pointer to the top-left of the preview, so the card
  /// stays "grabbed" at the same spot the user pressed down.
  offsetX: number;
  offsetY: number;
  /// Which column the pointer is currently hovering — drives the
  /// placeholder highlight.
  hoverSource: ServerSource | null;
}

const DRAG_THRESHOLD = 4;

/// localStorage key for the user's preferred detail-pane height (px).
const DETAIL_HEIGHT_KEY = "configonaut.mcp.detailHeight";
const DETAIL_MIN_HEIGHT = 180;
/// We always leave at least this much space above the detail pane for the
/// columns, even if the user drags the handle all the way to the top.
const COLUMNS_MIN_HEIGHT = 200;

export function McpServersView({ mode, onMutated }: Props) {
  const [listing, setListing] = useState<ServerListing | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [editedJson, setEditedJson] = useState("");
  const [editError, setEditError] = useState<string | null>(null);
  const [showAdd, setShowAdd] = useState(false);
  const [needsRestart, setNeedsRestart] = useState(false);
  const [statusMessage, setStatusMessage] = useState("Ready.");
  const [statusIsError, setStatusIsError] = useState(false);
  const [drag, setDrag] = useState<DragState | null>(null);

  /// Height (in px) of the JSON editor pane. The user can drag the
  /// resize handle between the columns and the detail panel to grow or
  /// shrink it — the preference is persisted in localStorage so the next
  /// session starts in the same configuration.
  const [detailHeight, setDetailHeight] = useState<number>(() => {
    const stored = localStorage.getItem(DETAIL_HEIGHT_KEY);
    if (stored) {
      const n = parseInt(stored, 10);
      if (Number.isFinite(n) && n >= DETAIL_MIN_HEIGHT) return n;
    }
    return 320;
  });

  /// Refs to each column body so pointer-move can hit-test them without
  /// relying on `document.elementFromPoint` (which gets confused by the
  /// floating preview). Keyed by source.
  const columnRefs = useRef<Record<ServerSource, HTMLDivElement | null>>({
    active: null,
    stored: null,
  });

  /// Ref to the `.mcp-view` container so the resize handler can measure
  /// the available space and clamp the detail height to it.
  const mcpViewRef = useRef<HTMLDivElement | null>(null);

  const setStatus = useCallback((msg: string, error = false) => {
    setStatusMessage(msg);
    setStatusIsError(error);
  }, []);

  const refresh = useCallback(async () => {
    try {
      const data = await listServers(mode);
      setListing(data);
      setLoadError(null);
    } catch (e) {
      setLoadError(String(e));
    }
  }, [mode]);

  useEffect(() => {
    setSelection(null);
    setEditedJson("");
    setEditError(null);
    setNeedsRestart(false);
    refresh();
  }, [mode, refresh]);

  const selectedEntry: ServerEntry | null = useMemo(() => {
    if (!selection || !listing) return null;
    const pool =
      selection.source === "active"
        ? listing.activeServers
        : listing.storedServers;
    return pool.find((s) => s.name === selection.name) ?? null;
  }, [selection, listing]);

  useEffect(() => {
    if (selectedEntry) {
      setEditedJson(selectedEntry.configJson);
      setEditError(null);
    }
  }, [selectedEntry]);

  /// Realtime shape-check of the JSON editor. Re-runs on every keystroke
  /// so the user gets feedback before hitting Save — matches the same
  /// rules the Rust side enforces (`validate_server_entries`), so if
  /// this passes, the backend save will almost always succeed (the only
  /// remaining failure mode is I/O errors writing the file).
  const jsonError = useMemo(() => {
    if (!selectedEntry) return null;
    return validateServerConfigJson(editedJson);
  }, [selectedEntry, editedJson]);

  /// Track whether the editor contents differ from what's on disk so we
  /// can reflect "dirty" state in the Save button.
  const isDirty = useMemo(() => {
    if (!selectedEntry) return false;
    return editedJson !== selectedEntry.configJson;
  }, [selectedEntry, editedJson]);

  /// While a drag is in progress we tag `<body>` with `is-dragging` so the
  /// global stylesheet can kill text selection everywhere. Without this,
  /// moving the cursor through the column titles during a drag ends up
  /// highlighting them, which looks broken.
  useEffect(() => {
    if (!drag) return;
    document.body.classList.add("is-dragging");
    const blockSelect = (e: Event) => e.preventDefault();
    window.addEventListener("selectstart", blockSelect);
    return () => {
      document.body.classList.remove("is-dragging");
      window.removeEventListener("selectstart", blockSelect);
    };
  }, [drag]);

  // ---------- Resize handle ----------

  /// Drag the thin bar between the columns and the detail panel to resize
  /// the JSON editor. We clamp so the columns always keep at least
  /// COLUMNS_MIN_HEIGHT px of space, and the detail panel keeps at least
  /// DETAIL_MIN_HEIGHT. The final value is persisted to localStorage on
  /// pointerup so the next launch opens at the same size.
  function handleResizeHandlePointerDown(
    e: ReactPointerEvent<HTMLDivElement>
  ) {
    if (e.button !== 0) return;
    e.preventDefault();
    const startY = e.clientY;
    const startHeight = detailHeight;
    const container = mcpViewRef.current;
    const containerRect = container?.getBoundingClientRect();

    const onMove = (ev: PointerEvent) => {
      const dy = ev.clientY - startY;
      // Dragging UP should grow the detail pane, so subtract dy.
      let next = startHeight - dy;
      const maxByContainer = containerRect
        ? containerRect.height - COLUMNS_MIN_HEIGHT
        : Number.POSITIVE_INFINITY;
      const max = Math.max(DETAIL_MIN_HEIGHT, maxByContainer);
      if (next < DETAIL_MIN_HEIGHT) next = DETAIL_MIN_HEIGHT;
      if (next > max) next = max;
      setDetailHeight(next);
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
      document.body.classList.remove("is-resizing");
      // Persist whatever the height is now.
      setDetailHeight((h) => {
        try {
          localStorage.setItem(DETAIL_HEIGHT_KEY, String(Math.round(h)));
        } catch {
          // Ignore quota / privacy errors.
        }
        return h;
      });
    };
    document.body.classList.add("is-resizing");
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }

  // ---------- Mutations ----------

  async function handleMove(entry: ServerEntry, source: ServerSource) {
    try {
      if (source === "active") {
        await moveServerToStored(mode, entry.name);
      } else {
        await moveServerToActive(mode, entry.name);
      }
      setNeedsRestart(true);
      setStatus(
        source === "active"
          ? `Moved "${entry.name}" to Inactive.`
          : `Turned "${entry.name}" on.`
      );
      onMutated();
      await refresh();
      setSelection({
        source: source === "active" ? "stored" : "active",
        name: entry.name,
      });
    } catch (e) {
      setStatus(String(e), true);
    }
  }

  async function handleDelete(entry: ServerEntry, source: ServerSource) {
    const ok = window.confirm(
      `Permanently delete "${entry.name}"? This can't be undone.`
    );
    if (!ok) return;
    try {
      await apiDeleteServer(mode, entry.name, source);
      if (source === "active") setNeedsRestart(true);
      if (
        selection &&
        selection.name === entry.name &&
        selection.source === source
      ) {
        setSelection(null);
      }
      setStatus(`Deleted "${entry.name}".`);
      onMutated();
      await refresh();
    } catch (e) {
      setStatus(String(e), true);
    }
  }

  async function handleSaveEdit() {
    if (!selection) return;
    // Defense in depth — the Save button is already disabled when
    // jsonError is set, but if the user hits ⌘S or otherwise bypasses
    // the click handler we still want to block the write.
    if (jsonError) {
      setEditError(jsonError);
      return;
    }
    setEditError(null);
    try {
      await updateServerConfig(
        mode,
        selection.name,
        selection.source,
        editedJson
      );
      if (selection.source === "active") setNeedsRestart(true);
      setStatus(`Saved "${selection.name}".`);
      onMutated();
      await refresh();
    } catch (e) {
      setEditError(String(e));
    }
  }

  function handleResetEdit() {
    if (selectedEntry) {
      setEditedJson(selectedEntry.configJson);
      setEditError(null);
    }
  }

  async function handleAddCommit(
    entries: ServerTuple[],
    target: "active" | "stored"
  ) {
    try {
      if (target === "active") {
        await addServersToActive(mode, entries);
        setNeedsRestart(true);
      } else {
        await addServersToStored(mode, entries);
      }
      setStatus(
        `Added ${entries.length} server${entries.length === 1 ? "" : "s"}.`
      );
      onMutated();
      await refresh();
    } catch (e) {
      setStatus(String(e), true);
    }
  }

  // ---------- Drag handling ----------

  /// Figure out which column the pointer is over by hit-testing our cached
  /// column refs. Returns null if the pointer is outside both.
  function hitTestColumn(x: number, y: number): ServerSource | null {
    for (const source of ["active", "stored"] as ServerSource[]) {
      const el = columnRefs.current[source];
      if (!el) continue;
      const rect = el.getBoundingClientRect();
      if (
        x >= rect.left &&
        x <= rect.right &&
        y >= rect.top &&
        y <= rect.bottom
      ) {
        return source;
      }
    }
    return null;
  }

  function handleRowPointerDown(
    e: ReactPointerEvent<HTMLDivElement>,
    entry: ServerEntry,
    source: ServerSource
  ) {
    // Only left button. Ignore interactions on inner buttons.
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (target.closest("button")) return;
    // Stop the browser's default "I'm starting a text selection" behaviour
    // right away. We'll still receive pointermove/up because listeners are
    // on `window`, not the row.
    e.preventDefault();

    const row = e.currentTarget;
    const rect = row.getBoundingClientRect();
    const startX = e.clientX;
    const startY = e.clientY;
    const offsetX = startX - rect.left;
    const offsetY = startY - rect.top;
    const previewWidth = rect.width;
    const previewHeight = rect.height;

    let armed = false;

    const onMove = (ev: PointerEvent) => {
      if (!armed) {
        const dx = ev.clientX - startX;
        const dy = ev.clientY - startY;
        if (Math.hypot(dx, dy) < DRAG_THRESHOLD) return;
        armed = true;
        setDrag({
          name: entry.name,
          source,
          pointerX: ev.clientX,
          pointerY: ev.clientY,
          previewWidth,
          previewHeight,
          offsetX,
          offsetY,
          hoverSource: hitTestColumn(ev.clientX, ev.clientY),
        });
        return;
      }
      setDrag((prev) =>
        prev
          ? {
              ...prev,
              pointerX: ev.clientX,
              pointerY: ev.clientY,
              hoverSource: hitTestColumn(ev.clientX, ev.clientY),
            }
          : prev
      );
    };

    const onUp = async (ev: PointerEvent) => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
      if (!armed) {
        // Treat as a click → select the row.
        setSelection({ source, name: entry.name });
        return;
      }
      const hover = hitTestColumn(ev.clientX, ev.clientY);
      setDrag(null);
      if (hover && hover !== source) {
        await handleMove(entry, source);
      }
    };

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }

  // ---------- Render ----------

  if (loadError) {
    return (
      <div className="main-body">
        <div className="banner error">Failed to load: {loadError}</div>
      </div>
    );
  }
  if (!listing) {
    return (
      <div className="main-body">
        <div className="empty">Loading…</div>
      </div>
    );
  }

  const totalCount = listing.activeServers.length + listing.storedServers.length;
  const activeCount = listing.activeServers.length;
  const storedCount = listing.storedServers.length;

  return (
    <>
      <header className="main-header" data-tauri-drag-region>
        <div className="title-block" data-tauri-drag-region>
          <div className="title-row">
            <h2>MCP Servers</h2>
            <span className="count-pill">{totalCount}</span>
          </div>
          <div className="config-path" title={listing.configPath}>
            {displayPath(listing.configPath)}
          </div>
        </div>
        {/* Icons left, primary CTA anchored to the far right — keeps
            "+ Add Server" in the same pixel position across every view. */}
        <div className="header-actions">
          <button className="icon" onClick={refresh} title="Reload">
            ↻
          </button>
          <button
            className="icon"
            onClick={async () => {
              try {
                await revealItemInDir(listing.configPath);
              } catch (e) {
                setStatus(`Couldn't reveal config: ${String(e)}`, true);
              }
            }}
            title="Reveal config file in Finder"
            aria-label="Reveal config file in Finder"
          >
            <svg
              viewBox="0 0 24 24"
              width="15"
              height="15"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.75"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
            </svg>
          </button>
          <button className="primary" onClick={() => setShowAdd(true)}>
            + Add Server
          </button>
        </div>
      </header>

      <div className="main-body">
        {needsRestart && (
          <div className="banner">
            Restart Claude {mode === "desktop" ? "Desktop" : "Code"} to apply
            your changes.
            <span style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
              {mode === "desktop" && (
                <button
                  className="primary"
                  onClick={async () => {
                    try {
                      await restartClaudeDesktop();
                      setNeedsRestart(false);
                    } catch (e) {
                      // Surface the error in the edit-error banner slot
                      // — cheap and visible without adding a dedicated
                      // toast system.
                      setEditError(`Restart failed: ${String(e)}`);
                    }
                  }}
                >
                  Restart now
                </button>
              )}
              <button
                className="ghost"
                onClick={() => setNeedsRestart(false)}
              >
                Dismiss
              </button>
            </span>
          </div>
        )}

        <div className="mcp-view" ref={mcpViewRef}>
          <div className="mcp-columns">
            <Column
              tone="active"
              title="Active"
              icon="⚡"
              subtitle={
                mode === "desktop"
                  ? "Running in Claude Desktop right now"
                  : "Running in Claude Code right now"
              }
              source="active"
              servers={listing.activeServers}
              selection={selection}
              onMove={(s) => handleMove(s, "active")}
              onDelete={(s) => handleDelete(s, "active")}
              onRowPointerDown={handleRowPointerDown}
              drag={drag}
              bodyRef={(el) => {
                columnRefs.current.active = el;
              }}
              emptyTitle="No active servers"
              emptyHint="Click + Add Server above, or drag one over from Inactive."
            />
            <Column
              tone="inactive"
              title="Inactive"
              icon="☾"
              subtitle="Saved for later — not running"
              source="stored"
              servers={listing.storedServers}
              selection={selection}
              onMove={(s) => handleMove(s, "stored")}
              onDelete={(s) => handleDelete(s, "stored")}
              onRowPointerDown={handleRowPointerDown}
              drag={drag}
              bodyRef={(el) => {
                columnRefs.current.stored = el;
              }}
              emptyTitle="Nothing saved"
              emptyHint="Drag a server here to turn it off without losing its config."
            />
          </div>

          <div
            className="detail-resize-handle"
            onPointerDown={handleResizeHandlePointerDown}
            role="separator"
            aria-orientation="horizontal"
            aria-label="Resize detail panel"
            title="Drag to resize"
          >
            <div className="detail-resize-grip" />
          </div>

          <div
            className="detail-panel"
            style={{ height: detailHeight, flex: "none" }}
          >
            <div className="detail-header">
              <div>
                <span className="title">
                  {selectedEntry ? selectedEntry.name : "Select a server"}
                </span>
                {selection && (
                  <span className="source-tag">
                    {selection.source === "active" ? "ACTIVE" : "INACTIVE"}
                  </span>
                )}
              </div>
            </div>
            <div className="detail-body">
              <textarea
                className={jsonError ? "invalid" : ""}
                value={editedJson}
                onChange={(e) => setEditedJson(e.currentTarget.value)}
                placeholder="Select a server to view and edit its JSON config."
                spellCheck={false}
                disabled={!selectedEntry}
              />
              {/* Realtime validation banner. The parse/shape error
                  wins over the save error — once the JSON is valid
                  we fall back to showing any backend rejection. */}
              {jsonError ? (
                <div className="banner error">{jsonError}</div>
              ) : (
                editError && <div className="banner error">{editError}</div>
              )}
              <div className="detail-actions">
                <button
                  className="ghost"
                  onClick={handleResetEdit}
                  disabled={!selectedEntry || !isDirty}
                >
                  Reset
                </button>
                <button
                  className="primary"
                  onClick={handleSaveEdit}
                  disabled={!selectedEntry || !!jsonError || !isDirty}
                  title={
                    jsonError
                      ? "Fix the errors above before saving"
                      : !isDirty
                        ? "No changes to save"
                        : undefined
                  }
                >
                  Save
                </button>
              </div>
            </div>
          </div>

          <div className="status-footer">
            <span
              className="glow-dot"
              style={{
                background: statusIsError ? "var(--red)" : "var(--green)",
                boxShadow: `0 0 6px ${
                  statusIsError ? "var(--red)" : "var(--green)"
                }`,
              }}
            />
            <span>{statusMessage}</span>
            <span className="spacer" />
            <span className="counts">
              {activeCount} active, {storedCount} inactive
            </span>
          </div>
        </div>
      </div>

      {drag && (
        <div
          className="drag-preview"
          style={{
            position: "fixed",
            left: drag.pointerX - drag.offsetX,
            top: drag.pointerY - drag.offsetY,
            width: drag.previewWidth,
            height: drag.previewHeight,
            pointerEvents: "none",
            zIndex: 9999,
          }}
        >
          <div
            className={`server-row drag-preview-card ${
              drag.source === "active" ? "from-active" : "from-inactive"
            }`}
          >
            <span className="name">{drag.name}</span>
          </div>
        </div>
      )}

      {showAdd && (
        <AddServerModal
          mode={mode}
          onClose={() => setShowAdd(false)}
          onCommit={handleAddCommit}
          onCatalogInstalled={async (name) => {
            setNeedsRestart(true);
            setStatus(`Installed "${name}" from the marketplace.`);
            onMutated();
            await refresh();
          }}
        />
      )}
    </>
  );
}

// ---------- Column ----------

interface ColumnProps {
  tone: "active" | "inactive";
  title: string;
  icon: string;
  subtitle: string;
  source: ServerSource;
  servers: ServerEntry[];
  selection: Selection | null;
  onMove: (entry: ServerEntry) => void;
  onDelete: (entry: ServerEntry) => void;
  onRowPointerDown: (
    e: ReactPointerEvent<HTMLDivElement>,
    entry: ServerEntry,
    source: ServerSource
  ) => void;
  drag: DragState | null;
  bodyRef: (el: HTMLDivElement | null) => void;
  emptyTitle: string;
  emptyHint: string;
}

function Column({
  tone,
  title,
  icon,
  subtitle,
  source,
  servers,
  selection,
  onMove,
  onDelete,
  onRowPointerDown,
  drag,
  bodyRef,
  emptyTitle,
  emptyHint,
}: ColumnProps) {
  const isDragOrigin = drag?.source === source;
  const isDropTarget =
    drag && drag.source !== source && drag.hoverSource === source;

  return (
    <div
      className={`column ${tone}-column ${isDropTarget ? "drop-target" : ""}`}
    >
      <div className="column-header">
        <div className="column-header-row">
          <span className="icon">{icon}</span>
          <h3>{title}</h3>
          <span className="count-mini">{servers.length}</span>
        </div>
      </div>
      <div className="column-subtitle">{subtitle}</div>
      <div className="column-divider" />
      <div className="column-body" ref={bodyRef}>
        {servers.length === 0 && !isDropTarget ? (
          <div className="empty">
            <div style={{ fontWeight: 500, marginBottom: 4 }}>{emptyTitle}</div>
            <div>{emptyHint}</div>
          </div>
        ) : (
          <>
            {servers.map((s) => {
              const isSelected =
                selection?.source === source && selection.name === s.name;
              // If this row is the one being dragged, collapse it into a
              // "ghost" slot so the origin column doesn't jump when the
              // card lifts off.
              const isOriginRow =
                isDragOrigin && drag?.name === s.name;
              if (isOriginRow) {
                return (
                  <div
                    key={s.name}
                    className="server-row origin-ghost"
                    style={{ height: drag?.previewHeight }}
                  />
                );
              }
              return (
                <div
                  key={s.name}
                  className={`server-row ${isSelected ? "selected" : ""}`}
                  onPointerDown={(e) => onRowPointerDown(e, s, source)}
                >
                  <span className="name">{s.name}</span>
                  <div className="actions">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onMove(s);
                      }}
                      title={
                        source === "active"
                          ? "Move to Inactive"
                          : "Move to Active"
                      }
                    >
                      {source === "active" ? "Turn Off" : "Turn On"}
                    </button>
                    <button
                      className="danger"
                      onClick={(e) => {
                        e.stopPropagation();
                        onDelete(s);
                      }}
                    >
                      Delete
                    </button>
                  </div>
                </div>
              );
            })}
            {isDropTarget && (
              <div
                className="server-row drop-placeholder"
                style={{ height: drag?.previewHeight }}
              >
                <span className="placeholder-text">
                  Drop to {source === "active" ? "turn on" : "turn off"}
                </span>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

// ---------- Path tidying ----------
// `displayPath` lives in src/lib/displayPath.ts — imported above.

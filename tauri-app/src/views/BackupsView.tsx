// Backups view — ported from Sources/SupportViews.swift BackupsView.
//
// Master/detail layout: the list of timestamped snapshots on the left,
// a preview pane on the right showing the selected backup's raw JSON plus
// a summary of which server names were added or removed between this
// snapshot and the next-newer one (or the current live config, if it's
// the newest backup).
//
// The "click a backup to see what changed" flow is the whole reason this
// view exists — without it, backups are opaque and users have to restore-
// blind if they want to get something back.

import { useCallback, useEffect, useState } from "react";
import {
  deleteBackup,
  forceBackup,
  getConfigPath,
  listBackups,
  readBackupContent,
  restoreBackup,
} from "../api";
import type { AppMode, BackupFile } from "../types";

interface Props {
  mode: AppMode;
  onMutated: () => void;
}

interface DiffSummary {
  added: string[];
  removed: string[];
}

export function BackupsView({ mode, onMutated }: Props) {
  const [backups, setBackups] = useState<BackupFile[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [previewText, setPreviewText] = useState<string>("");
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [diff, setDiff] = useState<DiffSummary | null>(null);

  const refresh = useCallback(async () => {
    try {
      const data = await listBackups(mode);
      setBackups(data);
      setError(null);
      // If the currently-selected backup was deleted / doesn't exist in
      // this mode, drop the selection rather than showing stale content.
      if (selectedPath && !data.find((b) => b.path === selectedPath)) {
        setSelectedPath(null);
        setPreviewText("");
        setDiff(null);
      }
    } catch (e) {
      setError(String(e));
    }
    // We intentionally depend only on `mode` — re-running this on every
    // selection change would wipe the preview.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode]);

  // Reset selection when the user flips Desktop ↔ CLI.
  useEffect(() => {
    setSelectedPath(null);
    setPreviewText("");
    setPreviewError(null);
    setDiff(null);
    refresh();
  }, [mode, refresh]);

  /// When the selection changes, fetch the file content and compute the
  /// added/removed diff. We compare against the next-newer backup in the
  /// list, or — if the selected backup *is* the newest — against the live
  /// config on disk. That mirrors what the SwiftUI version does.
  useEffect(() => {
    if (!selectedPath) {
      setPreviewText("");
      setDiff(null);
      return;
    }
    let cancelled = false;
    setPreviewLoading(true);
    setPreviewError(null);
    (async () => {
      try {
        const text = await readBackupContent(selectedPath);
        if (cancelled) return;
        setPreviewText(text);

        const idx = backups.findIndex((b) => b.path === selectedPath);
        let compareText: string | null = null;
        if (idx === -1) {
          compareText = null;
        } else if (idx > 0) {
          compareText = await readBackupContent(backups[idx - 1].path);
        } else {
          try {
            const livePath = await getConfigPath(mode);
            compareText = await readBackupContent(livePath);
          } catch {
            compareText = null;
          }
        }
        if (cancelled) return;
        setDiff(compareText ? computeDiff(text, compareText) : null);
      } catch (e) {
        if (cancelled) return;
        setPreviewError(String(e));
        setPreviewText("");
        setDiff(null);
      } finally {
        if (!cancelled) setPreviewLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [selectedPath, backups, mode]);

  async function handleRestore(b: BackupFile) {
    const ok = window.confirm(
      `Restore backup from ${formatDate(b.createdAt)}?\n\n` +
        `Your current config will be backed up first.`
    );
    if (!ok) return;
    try {
      await restoreBackup(mode, b.path);
      onMutated();
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleDelete(b: BackupFile) {
    const ok = window.confirm(`Delete backup ${b.fileName}?`);
    if (!ok) return;
    try {
      await deleteBackup(b.path);
      if (selectedPath === b.path) {
        setSelectedPath(null);
        setPreviewText("");
        setDiff(null);
      }
      onMutated();
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleForce() {
    try {
      await forceBackup(mode);
      onMutated();
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <>
      <header className="main-header" data-tauri-drag-region>
        <div className="title-block" data-tauri-drag-region>
          <div className="title-row">
            <h2>Backups</h2>
            <span
              className="count-pill"
              style={{
                color: "var(--cyan)",
                background: "rgba(0, 229, 255, 0.12)",
              }}
            >
              {backups.length}
            </span>
          </div>
          <div className="config-path">
            Automatic snapshots before every change · 30 kept
          </div>
        </div>
        {/* Icons left, primary CTA anchored to the far right — keeps
            "Snapshot now" in the same pixel position across every view. */}
        <div className="header-actions">
          <button className="icon" onClick={refresh} title="Reload">
            ↻
          </button>
          <button className="primary" onClick={handleForce}>
            Snapshot now
          </button>
        </div>
      </header>

      <div className="main-body main-body--flex">
        {error && <div className="banner error">{error}</div>}

        {backups.length === 0 ? (
          <div className="empty" style={{ padding: "60px 20px" }}>
            <div style={{ fontWeight: 600, marginBottom: 6, fontSize: 14 }}>
              No backups yet
            </div>
            <div>
              Backups are created automatically whenever you add, remove, or
              move a server. Up to 30 are kept per mode.
            </div>
          </div>
        ) : (
          <div className="backups-layout">
            <div className="backup-list">
              {backups.map((b, idx) => {
                const isSelected = selectedPath === b.path;
                return (
                  <div
                    key={b.path}
                    className={`backup-row ${isSelected ? "selected" : ""}`}
                    onClick={() => setSelectedPath(b.path)}
                  >
                    <div className="meta">
                      <div className="filename">
                        {formatDate(b.createdAt)}
                        {idx === 0 && (
                          <span className="latest-tag">LATEST</span>
                        )}
                      </div>
                      <div className="sub">
                        {formatBytes(b.sizeBytes)} · {b.fileName}
                      </div>
                    </div>
                    <div className="actions">
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleRestore(b);
                        }}
                      >
                        Restore
                      </button>
                      <button
                        className="danger"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDelete(b);
                        }}
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>

            <div className="backup-preview">
              {!selectedPath ? (
                <div className="backup-preview-empty">
                  <div className="hint-glyph">◎</div>
                  <div>Click a backup to preview it.</div>
                  <div className="sub">
                    You'll see the full config plus a summary of which
                    servers changed.
                  </div>
                </div>
              ) : previewLoading ? (
                <div className="backup-preview-empty">Loading preview…</div>
              ) : previewError ? (
                <div className="banner error" style={{ margin: 12 }}>
                  {previewError}
                </div>
              ) : (
                <>
                  <div className="backup-preview-header">
                    <span className="label">PREVIEW</span>
                    <span className="sub">
                      {backups.find((b) => b.path === selectedPath)?.fileName}
                    </span>
                  </div>
                  {diff &&
                    (diff.added.length > 0 || diff.removed.length > 0) && (
                      <div className="diff-summary">
                        <div className="diff-label">
                          Changes after this snapshot
                        </div>
                        {diff.removed.map((name) => (
                          <div key={`r-${name}`} className="diff-row removed">
                            <span className="glyph">−</span>
                            <span className="name">{name}</span>
                          </div>
                        ))}
                        {diff.added.map((name) => (
                          <div key={`a-${name}`} className="diff-row added">
                            <span className="glyph">+</span>
                            <span className="name">{name}</span>
                          </div>
                        ))}
                      </div>
                    )}
                  {diff &&
                    diff.added.length === 0 &&
                    diff.removed.length === 0 && (
                      <div className="diff-summary">
                        <div className="diff-label muted">
                          No server additions or removals after this snapshot.
                        </div>
                      </div>
                    )}
                  <pre className="backup-preview-body">{previewText}</pre>
                </>
              )}
            </div>
          </div>
        )}
      </div>
    </>
  );
}

// ---------- Diff ----------

/// Parse two JSON blobs and return the set of `mcpServers` keys that were
/// added and removed going from `backupText` → `newerText`. Quietly returns
/// an empty diff if either side fails to parse, so a malformed backup
/// doesn't blow up the whole view.
function computeDiff(backupText: string, newerText: string): DiffSummary {
  const backupKeys = extractServerKeys(backupText);
  const newerKeys = extractServerKeys(newerText);
  if (!backupKeys || !newerKeys) return { added: [], removed: [] };
  const backupSet = new Set(backupKeys);
  const newerSet = new Set(newerKeys);
  const added = [...newerSet].filter((k) => !backupSet.has(k)).sort();
  const removed = [...backupSet].filter((k) => !newerSet.has(k)).sort();
  return { added, removed };
}

function extractServerKeys(text: string): string[] | null {
  try {
    const parsed = JSON.parse(text);
    if (
      parsed &&
      typeof parsed === "object" &&
      "mcpServers" in parsed &&
      parsed.mcpServers &&
      typeof parsed.mcpServers === "object"
    ) {
      return Object.keys(parsed.mcpServers);
    }
    return [];
  } catch {
    return null;
  }
}

// ---------- Formatting ----------

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(iso: string): string {
  if (!iso) return "unknown date";
  try {
    const d = new Date(iso);
    return d.toLocaleString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

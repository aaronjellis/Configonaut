import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  apiCheckRuntime, apiDownloadNode, apiInspectInstall, apiInstallServer,
  onInstallProgress, onRuntimeInstallProgress,
} from "../api";
import {
  initialSetupState, installEnabled, setupReducer, validateFields,
} from "../lib/setupStepReducer";
import type { AppMode, RuntimeInstallProgress, RuntimeName } from "../types";
import { ConfigField } from "./ConfigField";
import { InstallProgress } from "./InstallProgress";
import { PrerequisiteRow } from "./PrerequisiteRow";

interface Props {
  mode: AppMode;
  serverId: string;
  /// Receives the name the server was installed under (may differ from
  /// serverId when a collision was resolved with a -2 / -3 suffix).
  onDone: (installedName: string) => void;
  onCancel: () => void;
}

export function SetupStep({ mode, serverId, onDone, onCancel }: Props) {
  const [state, dispatch] = useReducer(setupReducer, initialSetupState);
  const [runtimeProgress, setRuntimeProgress] = useState<Record<RuntimeName, RuntimeInstallProgress | null>>({
    node: null, uv: null, docker: null,
  });
  const downloadingRef = useRef(false);

  const handleCheck = useCallback(async (runtime: RuntimeName) => {
    const status = await apiCheckRuntime(runtime);
    dispatch({ type: "prereqStatus", runtime, status });
  }, []);

  // Load schema once.
  useEffect(() => {
    let cancelled = false;
    apiInspectInstall(serverId).then((schema) => {
      if (!cancelled) dispatch({ type: "loaded", schema });
    });
    return () => { cancelled = true; };
  }, [serverId]);

  // Subscribe to install progress events.
  // The event listener handles log streaming only — the invoke return
  // in handleInstall owns the done/error signals so they fire exactly once.
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    onInstallProgress((p) => {
      if (p.kind === "log") dispatch({ type: "installLog", line: p.line });
    }).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => { cancelled = true; unlisten?.(); };
  }, []);

  // Subscribe to runtime download progress events.
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    onRuntimeInstallProgress((p) => {
      if (cancelled) return;
      setRuntimeProgress((prev) => ({ ...prev, node: p }));
      if (p.kind === "done") {
        handleCheck("node");
      }
    }).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => { cancelled = true; unlisten?.(); };
  }, [handleCheck]);

  const handleDownload = async (runtime: RuntimeName) => {
    if (runtime !== "node" || downloadingRef.current) return;
    downloadingRef.current = true;
    setRuntimeProgress((prev) => ({
      ...prev,
      node: { kind: "downloading", percent: 0, downloadedBytes: 0, totalBytes: 0 },
    }));
    try {
      await apiDownloadNode();
    } catch (err) {
      setRuntimeProgress((prev) => ({
        ...prev,
        node: { kind: "error", message: String(err) },
      }));
    } finally {
      downloadingRef.current = false;
    }
  };

  const handleInstall = async () => {
    dispatch({ type: "installStarted" });
    try {
      const name = await apiInstallServer(mode, serverId, state.fieldValues);
      dispatch({ type: "installDone", installedName: name });
      if (!state.schema?.postInstallNotes?.length) {
        onDone(name);
      }
    } catch (err) {
      dispatch({ type: "installError", message: String(err), canRetry: true });
    }
  };

  if (!state.schema) return <div className="setup-step setup-step--loading">Loading…</div>;

  if (state.phase === "done" && state.schema.postInstallNotes.length > 0) {
    return (
      <div className="setup-step">
        <div className="post-install">
          <h3>Server installed — next steps</h3>
          <p className="post-install-sub">
            This server needs a bit of setup outside of Configonaut before it will work.
          </p>
          <ol className="post-install-notes">
            {state.schema.postInstallNotes.map((note, i) => (
              <li key={i} className="post-install-note">
                <strong>{note.title}</strong>
                <p>{note.body}</p>
                {note.url && (
                  <button
                    className="link-btn"
                    onClick={() => openUrl(note.url!)}
                  >
                    Open guide
                  </button>
                )}
              </li>
            ))}
          </ol>
        </div>
        <div className="setup-actions">
          <button className="primary" onClick={() => onDone(state.installedName ?? serverId)}>Done</button>
        </div>
      </div>
    );
  }

  const fieldErrors = state.phase === "fieldsPending"
    ? Object.fromEntries(
        validateFields(state.schema, state.fieldValues).missing.map((n) => [n, "Required"]),
      )
    : {};

  return (
    <div className="setup-step">
      {state.schema.prerequisites.length > 0 && (() => {
        const allGreen = state.schema!.prerequisites.every(
          (p) => state.prereqStatus[p.type]?.installed,
        );
        return (
          <section className="setup-section">
            {allGreen ? (
              <h3>Prerequisites</h3>
            ) : (
              <div className="prereq-intro">
                <p className="prereq-intro-heading">
                  This MCP server needs a few things before it can run.
                </p>
                <p className="prereq-intro-sub">
                  Install what's missing below, or hit re-check if you've already set it up.
                </p>
              </div>
            )}
            {state.schema!.prerequisites.map((p) => (
              <PrerequisiteRow
                key={p.type}
                entry={{ ...p, status: state.prereqStatus[p.type] ?? null }}
                onCheck={handleCheck}
                onOpenUrl={(url) => openUrl(url)}
                onInstall={handleDownload}
                downloadProgress={runtimeProgress[p.type]}
              />
            ))}
          </section>
        );
      })()}

      {state.schema.configFields.length > 0 && (
        <section className="setup-section">
          <h3>Configuration</h3>
          {state.schema.configFields.map((f) => (
            <ConfigField
              key={f.name}
              field={f}
              value={state.fieldValues[f.name]}
              onChange={(value) => dispatch({ type: "fieldChange", name: f.name, value })}
              error={fieldErrors[f.name]}
            />
          ))}
        </section>
      )}

      {state.schema.hasUnknownInstallStep && (
        <div className="setup-banner">
          This server's catalog uses install steps Configonaut doesn't know about.
          You may need to install it manually after saving.
        </div>
      )}

      {state.phase === "installing" && (
        <InstallProgress label="Installing…" log={state.log} />
      )}

      {state.phase === "error" && (
        <div className="setup-error">
          <p>{state.errorMessage}</p>
          {state.errorCanRetry && (
            <button className="primary" onClick={() => { dispatch({ type: "installRetry" }); handleInstall(); }}>
              Retry
            </button>
          )}
        </div>
      )}

      <div className="setup-actions">
        <button className="secondary" onClick={onCancel}>Cancel</button>
        <button className="primary" disabled={!installEnabled(state)} onClick={handleInstall}>
          Install Server
        </button>
      </div>
    </div>
  );
}

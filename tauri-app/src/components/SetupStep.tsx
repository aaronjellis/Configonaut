import { useEffect, useReducer, useRef } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  apiCheckRuntime, apiInspectInstall, apiInstallServer, onInstallProgress,
} from "../api";
import {
  initialSetupState, installEnabled, setupReducer, validateFields,
} from "../lib/setupStepReducer";
import { ConfigField } from "./ConfigField";
import { InstallProgress } from "./InstallProgress";
import { PrerequisiteRow } from "./PrerequisiteRow";

interface Props {
  serverId: string;
  onDone: () => void;
  onCancel: () => void;
}

export function SetupStep({ serverId, onDone, onCancel }: Props) {
  const [state, dispatch] = useReducer(setupReducer, initialSetupState);
  const unlistenRef = useRef<(() => void) | null>(null);

  // Load schema once.
  useEffect(() => {
    let cancelled = false;
    apiInspectInstall(serverId).then((schema) => {
      if (!cancelled) dispatch({ type: "loaded", schema });
    });
    return () => { cancelled = true; };
  }, [serverId]);

  // Subscribe to install progress events for the lifetime of the component.
  useEffect(() => {
    onInstallProgress((p) => {
      if (p.kind === "log") dispatch({ type: "installLog", line: p.line });
      if (p.kind === "step" && p.step === "done") dispatch({ type: "installDone" });
      if (p.kind === "error") dispatch({ type: "installError", message: p.message, canRetry: p.canRetry });
    }).then((unlisten) => { unlistenRef.current = unlisten; });
    return () => { unlistenRef.current?.(); };
  }, []);

  const handleCheck = async (runtime: "node" | "uv" | "docker") => {
    const status = await apiCheckRuntime(runtime);
    dispatch({ type: "prereqStatus", runtime, status });
  };

  const handleInstall = async () => {
    dispatch({ type: "installStarted" });
    try {
      await apiInstallServer(serverId, state.fieldValues);
      dispatch({ type: "installDone" });
      onDone();
    } catch (err) {
      // Error event already dispatched by the listener; this catch is a backstop.
      dispatch({ type: "installError", message: String(err), canRetry: true });
    }
  };

  if (!state.schema) return <div className="setup-step setup-step--loading">Loading…</div>;

  const fieldErrors = state.phase === "fieldsPending"
    ? Object.fromEntries(
        validateFields(state.schema, state.fieldValues).missing.map((n) => [n, "Required"]),
      )
    : {};

  return (
    <div className="setup-step">
      {state.schema.prerequisites.length > 0 && (
        <section className="setup-section">
          <h3>Prerequisites</h3>
          {state.schema.prerequisites.map((p) => (
            <PrerequisiteRow
              key={p.type}
              entry={{ ...p, status: state.prereqStatus[p.type] ?? null }}
              onCheck={handleCheck}
              onOpenUrl={(url) => openUrl(url)}
            />
          ))}
        </section>
      )}

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

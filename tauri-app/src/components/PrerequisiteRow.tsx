import type { PrerequisiteEntry, RuntimeName } from "../types";

interface Props {
  entry: PrerequisiteEntry;
  onCheck: (name: RuntimeName) => void;
  onOpenUrl: (url: string) => void;
}

const RUNTIME_LABEL: Record<RuntimeName, string> = {
  node: "Node.js",
  uv: "uv (Python)",
  docker: "Docker Desktop",
};

export function PrerequisiteRow({ entry, onCheck, onOpenUrl }: Props) {
  const { type, status, installUrl } = entry;
  const installed = status?.installed === true;

  return (
    <div className={`prereq-row ${installed ? "prereq-row--ok" : "prereq-row--missing"}`}>
      <span className="prereq-icon" aria-hidden>
        {installed ? "✓" : "○"}
      </span>
      <span className="prereq-label">
        {RUNTIME_LABEL[type]}
        {installed && status?.version && (
          <span className="prereq-version"> {status.version}</span>
        )}
      </span>
      <div className="prereq-actions">
        {!installed && installUrl && (
          <button className="link-button" onClick={() => onOpenUrl(installUrl)}>
            Open install page
          </button>
        )}
        <button className="link-button" onClick={() => onCheck(type)}>
          Re-check
        </button>
      </div>
    </div>
  );
}

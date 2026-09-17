import type { PrerequisiteEntry, RuntimeInstallProgress, RuntimeName } from "../types";

interface Props {
  entry: PrerequisiteEntry;
  onCheck: (name: RuntimeName) => void;
  onOpenUrl: (url: string) => void;
  onInstall: (name: RuntimeName) => void;
  downloadProgress: RuntimeInstallProgress | null;
}

const RUNTIME_LABEL: Record<RuntimeName, string> = {
  node: "Node.js",
  uv: "uv (Python)",
  docker: "Docker Desktop",
};

const RUNTIME_EXPLAINER: Record<RuntimeName, string> = {
  node: "Node.js is a JavaScript runtime that this MCP server needs to run. Configonaut can install it for you — it only takes a moment.",
  uv: "uv is a fast Python package manager. It's bundled with Configonaut, so this should resolve automatically.",
  docker: "Docker runs this MCP server in an isolated container. You'll need to install Docker Desktop separately.",
};

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function progressLabel(p: RuntimeInstallProgress): string {
  switch (p.kind) {
    case "downloading": {
      const pct = p.percent.toFixed(0);
      if (p.totalBytes > 0) {
        return `Downloading… ${pct}% (${formatBytes(p.downloadedBytes)} / ${formatBytes(p.totalBytes)})`;
      }
      return `Downloading… ${formatBytes(p.downloadedBytes)}`;
    }
    case "verifyingDownload":
      return "Verifying download…";
    case "extracting":
      return "Extracting…";
    case "verifying":
      return "Verifying installation…";
    case "done":
      return `Installed v${p.version}`;
    case "error":
      return p.message;
  }
}

export function PrerequisiteRow({ entry, onCheck, onOpenUrl, onInstall, downloadProgress }: Props) {
  const { type, status, installUrl } = entry;
  const installed = status?.installed === true;
  const isDownloading = downloadProgress != null
    && downloadProgress.kind !== "done"
    && downloadProgress.kind !== "error";
  const downloadFailed = downloadProgress?.kind === "error";
  const canDownload = entry.type === "node";

  return (
    <div className={`prereq-row ${installed ? "prereq-row--ok" : "prereq-row--missing"}`}>
      <span className="prereq-icon" aria-hidden>
        {installed ? "✓" : isDownloading ? "⟳" : "○"}
      </span>
      <div className="prereq-content">
        <div className="prereq-header">
          <span className="prereq-label">
            {RUNTIME_LABEL[type]}
            {installed && status?.version && (
              <span className="prereq-version"> {status.version}</span>
            )}
            {installed && status?.source === "managed" && (
              <span className="prereq-source"> (managed by Configonaut)</span>
            )}
          </span>
          <div className="prereq-actions">
            {!installed && !isDownloading && canDownload && (
              <button className="primary prereq-install-btn" onClick={() => onInstall(type)}>
                {downloadFailed ? "Retry" : "Install"}
              </button>
            )}
            {!installed && !isDownloading && !canDownload && installUrl && (
              <button className="link-button" onClick={() => onOpenUrl(installUrl)}>
                Open install page
              </button>
            )}
            {!isDownloading && (
              <button className="link-button" onClick={() => onCheck(type)}>
                Re-check
              </button>
            )}
          </div>
        </div>

        {!installed && !isDownloading && (
          <p className="prereq-explainer">{RUNTIME_EXPLAINER[type]}</p>
        )}

        {isDownloading && downloadProgress.kind === "downloading" && (
          <div className="prereq-progress">
            <div className="prereq-progress-bar">
              <div
                className="prereq-progress-fill"
                style={{ width: `${Math.min(downloadProgress.percent, 100)}%` }}
              />
            </div>
            <span className="prereq-progress-text">
              {progressLabel(downloadProgress)}
            </span>
          </div>
        )}
        {isDownloading && downloadProgress.kind !== "downloading" && (
          <div className="prereq-progress">
            <span className="prereq-progress-text">
              <span className="prereq-progress-spinner" aria-hidden>⟳</span>
              {progressLabel(downloadProgress)}
            </span>
          </div>
        )}
        {downloadFailed && (
          <div className="prereq-download-error">
            {downloadProgress.message}
          </div>
        )}
      </div>
    </div>
  );
}

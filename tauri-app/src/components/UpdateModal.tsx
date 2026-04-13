// Modal overlay that appears when a new version is available.
// Shows version + release notes, lets the user download & install in-place,
// and offers a "Restart Now" button once the install completes.

import { useState } from "react";
import type { Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

interface Props {
  update: Update;
  onDismiss: () => void;
}

export function UpdateModal({ update, onDismiss }: Props) {
  const [phase, setPhase] = useState<"prompt" | "downloading" | "done">(
    "prompt"
  );
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const handleInstall = async () => {
    setPhase("downloading");
    setError(null);

    let totalBytes = 0;
    let downloaded = 0;

    try {
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            totalBytes = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            if (totalBytes > 0) {
              setProgress(Math.min(100, (downloaded / totalBytes) * 100));
            }
            break;
          case "Finished":
            setProgress(100);
            break;
        }
      });
      setPhase("done");
    } catch (err) {
      setError(String(err));
      setPhase("prompt");
    }
  };

  return (
    <div className="update-overlay" onClick={onDismiss}>
      <div className="update-modal" onClick={(e) => e.stopPropagation()}>
        <div className="update-icon">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none">
            <path
              d="M12 2v14m0 0l-5-5m5 5l5-5M4 20h16"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              transform="rotate(180 12 12)"
            />
          </svg>
        </div>

        <h2 className="update-title">Update Available</h2>
        <p className="update-version">
          v{update.version}
        </p>

        {update.body && (
          <p className="update-notes">{update.body}</p>
        )}

        {error && <p className="update-error">{error}</p>}

        {phase === "prompt" && (
          <div className="update-actions">
            <button className="btn update-btn-primary" onClick={handleInstall}>
              Update Now
            </button>
            <button className="btn update-btn-secondary" onClick={onDismiss}>
              Later
            </button>
          </div>
        )}

        {phase === "downloading" && (
          <div className="update-progress">
            <div className="update-progress-bar">
              <div
                className="update-progress-fill"
                style={{ width: `${progress}%` }}
              />
            </div>
            <span className="update-progress-text">
              {progress < 100
                ? `Downloading... ${Math.round(progress)}%`
                : "Installing..."}
            </span>
          </div>
        )}

        {phase === "done" && (
          <div className="update-actions">
            <p className="update-done-text">
              Update installed successfully.
            </p>
            <button
              className="btn update-btn-primary"
              onClick={() => relaunch()}
            >
              Restart Now
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

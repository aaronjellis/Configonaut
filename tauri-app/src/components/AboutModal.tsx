// Custom About modal with app info and a "Check for Updates" button.

import { useState } from "react";

interface Props {
  version: string;
  onCheckForUpdates: () => Promise<void>;
  onDismiss: () => void;
}

export function AboutModal({ version, onCheckForUpdates, onDismiss }: Props) {
  const [checking, setChecking] = useState(false);

  const handleCheck = async () => {
    setChecking(true);
    try {
      await onCheckForUpdates();
    } finally {
      setChecking(false);
    }
    onDismiss();
  };

  return (
    <div className="update-overlay" onClick={onDismiss}>
      <div className="about-modal" onClick={(e) => e.stopPropagation()}>
        <div className="about-icon">
          <img src="/icon.png" alt="" draggable={false} />
        </div>
        <h2 className="about-title">Configonaut</h2>
        <p className="about-version">v{version}</p>
        <p className="about-desc">
          MCP server, hook, and skill manager for
          <br />
          Claude Desktop and Claude Code.
        </p>
        <div className="about-actions">
          <button
            className="update-btn-primary"
            onClick={handleCheck}
            disabled={checking}
          >
            {checking ? "Checking\u2026" : "Check for Updates"}
          </button>
          <a
            className="about-link"
            href="https://github.com/aaronjellis/Configonaut"
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub
          </a>
        </div>
      </div>
    </div>
  );
}

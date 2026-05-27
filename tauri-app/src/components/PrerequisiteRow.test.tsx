import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { PrerequisiteRow } from "./PrerequisiteRow";

const ENTRY = {
  type: "node" as const,
  status: null,
  installUrl: "https://nodejs.org",
};

const DOCKER_ENTRY = {
  type: "docker" as const,
  status: null,
  installUrl: "https://docker.com",
};

const defaults = {
  onCheck: () => {},
  onOpenUrl: () => {},
  onInstall: () => {},
  downloadProgress: null,
};

describe("PrerequisiteRow", () => {
  it("renders Install button for node when not installed", () => {
    render(<PrerequisiteRow entry={ENTRY} {...defaults} />);
    expect(screen.getByText("Node.js")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Install/i })).toBeInTheDocument();
  });

  it("shows explainer text when not installed", () => {
    render(<PrerequisiteRow entry={ENTRY} {...defaults} />);
    expect(screen.getByText(/JavaScript runtime/i)).toBeInTheDocument();
  });

  it("renders Open install page for docker when not installed", () => {
    render(<PrerequisiteRow entry={DOCKER_ENTRY} {...defaults} />);
    expect(screen.getByText("Docker Desktop")).toBeInTheDocument();
    expect(screen.getByText(/Open install page/i)).toBeInTheDocument();
  });

  it("renders green state with version when installed", () => {
    render(<PrerequisiteRow
      entry={{ ...ENTRY, status: { installed: true, version: "v20.11", source: "system" } }}
      {...defaults}
    />);
    expect(screen.getByText(/v20.11/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Install/i })).not.toBeInTheDocument();
    expect(screen.queryByText(/JavaScript runtime/i)).not.toBeInTheDocument();
  });

  it("shows managed badge for managed installs", () => {
    render(<PrerequisiteRow
      entry={{ ...ENTRY, status: { installed: true, version: "22.13.1", source: "managed" } }}
      {...defaults}
    />);
    expect(screen.getByText(/managed by Configonaut/i)).toBeInTheDocument();
  });

  it("calls onCheck when re-check button clicked", () => {
    const onCheck = vi.fn();
    render(<PrerequisiteRow entry={ENTRY} {...defaults} onCheck={onCheck} />);
    fireEvent.click(screen.getByText(/Re-check/i));
    expect(onCheck).toHaveBeenCalledWith("node");
  });

  it("calls onInstall when Install button clicked", () => {
    const onInstall = vi.fn();
    render(<PrerequisiteRow entry={ENTRY} {...defaults} onInstall={onInstall} />);
    fireEvent.click(screen.getByRole("button", { name: /Install/i }));
    expect(onInstall).toHaveBeenCalledWith("node");
  });

  it("calls onOpenUrl with the install URL when link clicked (docker)", () => {
    const onOpenUrl = vi.fn();
    render(<PrerequisiteRow entry={DOCKER_ENTRY} {...defaults} onOpenUrl={onOpenUrl} />);
    fireEvent.click(screen.getByText(/Open install page/i));
    expect(onOpenUrl).toHaveBeenCalledWith("https://docker.com");
  });

  it("shows progress bar when downloading", () => {
    render(<PrerequisiteRow
      entry={ENTRY}
      {...defaults}
      downloadProgress={{ kind: "downloading", percent: 45, downloadedBytes: 15_000_000, totalBytes: 33_000_000 }}
    />);
    expect(screen.getByText(/Downloading/i)).toBeInTheDocument();
    expect(screen.getByText(/45%/)).toBeInTheDocument();
    // Explainer should be hidden during download
    expect(screen.queryByText(/JavaScript runtime/i)).not.toBeInTheDocument();
  });

  it("shows extracting state", () => {
    render(<PrerequisiteRow
      entry={ENTRY}
      {...defaults}
      downloadProgress={{ kind: "extracting" }}
    />);
    expect(screen.getByText(/Extracting/i)).toBeInTheDocument();
  });

  it("shows error and retry button after download failure", () => {
    render(<PrerequisiteRow
      entry={ENTRY}
      {...defaults}
      downloadProgress={{ kind: "error", message: "Network error" }}
    />);
    expect(screen.getAllByText(/Network error/i)).toHaveLength(1);
    expect(screen.getByRole("button", { name: /Retry/i })).toBeInTheDocument();
  });

  it("shows install button in done state before re-check resolves", () => {
    render(<PrerequisiteRow
      entry={ENTRY}
      {...defaults}
      downloadProgress={{ kind: "done", version: "22.13.1" }}
    />);
    // Download finished but status hasn't updated yet (re-check pending).
    // Progress UI is hidden, Install button still shows (status not yet installed).
    expect(screen.queryByText(/Downloading/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/Extracting/i)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Re-check/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Install/i })).toBeInTheDocument();
  });

  it("hides explainer and progress when downloading is complete and installed", () => {
    render(<PrerequisiteRow
      entry={{ ...ENTRY, status: { installed: true, version: "22.13.1", source: "managed" } }}
      {...defaults}
      downloadProgress={{ kind: "done", version: "22.13.1" }}
    />);
    expect(screen.queryByText(/JavaScript runtime/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/Downloading/i)).not.toBeInTheDocument();
    expect(screen.getByText(/22.13.1/)).toBeInTheDocument();
  });
});

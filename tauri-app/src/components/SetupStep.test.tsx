import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SetupStep } from "./SetupStep";

vi.mock("../api", () => ({
  apiInspectInstall: vi.fn(),
  apiCheckRuntime: vi.fn(),
  apiInstallRuntime: vi.fn(),
  apiInstallServer: vi.fn(),
  apiDownloadNode: vi.fn(() => Promise.resolve()),
  onInstallProgress: vi.fn(() => Promise.resolve(() => {})),
  onRuntimeInstallProgress: vi.fn(() => Promise.resolve(() => {})),
}));

// The tauri opener plugin is also mocked so the test doesn't try to spawn a browser.
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

import {
  apiInspectInstall, apiCheckRuntime, apiInstallServer,
} from "../api";

const SCHEMA = {
  prerequisites: [{ type: "node", status: null, installUrl: "https://nodejs.org" }],
  configFields: [{
    name: "paths", kind: "argSpread", type: "pathArray", label: "Paths", required: true,
  }],
  installStepCount: 1,
  hasUnknownInstallStep: false,
  postInstallNotes: [],
};

const SCHEMA_WITH_NOTES = {
  ...SCHEMA,
  prerequisites: [],
  configFields: [],
  postInstallNotes: [
    { title: "Authorize your org", body: "Run sf org login web first.", url: "https://example.com/guide" },
    { title: "Restart Claude", body: "Restart so it picks up the server." },
  ],
};

describe("SetupStep", () => {
  beforeEach(() => {
    vi.mocked(apiInspectInstall).mockResolvedValue(SCHEMA as any);
    vi.mocked(apiCheckRuntime).mockResolvedValue(
      { installed: false, version: null, source: null } as any,
    );
    vi.mocked(apiInstallServer).mockResolvedValue(undefined as any);
  });

  it("loads schema on mount and renders prereq + field rows", async () => {
    render(<SetupStep serverId="filesystem" onDone={() => {}} onCancel={() => {}} />);
    await waitFor(() => expect(screen.getByText(/needs a few things/i)).toBeInTheDocument());
    expect(screen.getByText("Node.js")).toBeInTheDocument();
    expect(screen.getByText(/Paths/i)).toBeInTheDocument();
  });

  it("shows post-install notes after successful install", async () => {
    vi.mocked(apiInspectInstall).mockResolvedValue(SCHEMA_WITH_NOTES as any);
    const onDone = vi.fn();
    render(<SetupStep serverId="salesforce-dx" onDone={onDone} onCancel={() => {}} />);
    await waitFor(() => screen.getByRole("button", { name: /Install Server/i }));
    fireEvent.click(screen.getByRole("button", { name: /Install Server/i }));
    await waitFor(() => screen.getByText(/next steps/i));
    expect(screen.getByText("Authorize your org")).toBeInTheDocument();
    expect(screen.getByText("Restart Claude")).toBeInTheDocument();
    expect(screen.getByText(/Open guide/i)).toBeInTheDocument();
    expect(onDone).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: /Done/i }));
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it("calls onDone immediately when no post-install notes", async () => {
    vi.mocked(apiInspectInstall).mockResolvedValue({
      ...SCHEMA_WITH_NOTES,
      postInstallNotes: [],
    } as any);
    const onDone = vi.fn();
    render(<SetupStep serverId="test" onDone={onDone} onCancel={() => {}} />);
    await waitFor(() => screen.getByRole("button", { name: /Install Server/i }));
    fireEvent.click(screen.getByRole("button", { name: /Install Server/i }));
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
  });

  it("install button disabled until prereqs and fields satisfied", async () => {
    vi.mocked(apiCheckRuntime).mockResolvedValueOnce(
      { installed: true, version: "v20", source: "system" } as any,
    );
    render(<SetupStep serverId="filesystem" onDone={() => {}} onCancel={() => {}} />);
    await waitFor(() => screen.getByText("Node.js"));
    fireEvent.click(screen.getByRole("button", { name: /Re-check/i }));
    await waitFor(() => screen.getByText(/v20/));

    const btn = screen.getByRole("button", { name: /Install Server/i });
    expect(btn).toBeDisabled();
    fireEvent.click(screen.getByText(/\+ Add path/i));
    fireEvent.change(screen.getAllByPlaceholderText(/path/i)[0], { target: { value: "/a" } });
    expect(btn).toBeEnabled();
  });
});

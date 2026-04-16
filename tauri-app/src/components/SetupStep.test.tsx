import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SetupStep } from "./SetupStep";

vi.mock("../api", () => ({
  apiInspectInstall: vi.fn(),
  apiCheckRuntime: vi.fn(),
  apiInstallRuntime: vi.fn(),
  apiInstallServer: vi.fn(),
  onInstallProgress: vi.fn(() => Promise.resolve(() => {})),
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
    await waitFor(() => expect(screen.getByText(/Node\.js/i)).toBeInTheDocument());
    expect(screen.getByText(/Paths/i)).toBeInTheDocument();
  });

  it("install button disabled until prereqs and fields satisfied", async () => {
    vi.mocked(apiCheckRuntime).mockResolvedValueOnce(
      { installed: true, version: "v20", source: "system" } as any,
    );
    render(<SetupStep serverId="filesystem" onDone={() => {}} onCancel={() => {}} />);
    await waitFor(() => screen.getByText(/Node\.js/i));
    fireEvent.click(screen.getByText(/Re-check/i));
    await waitFor(() => screen.getByText(/v20/));

    const btn = screen.getByRole("button", { name: /Install Server/i });
    expect(btn).toBeDisabled();
    fireEvent.click(screen.getByText(/\+ Add path/i));
    fireEvent.change(screen.getAllByPlaceholderText(/path/i)[0], { target: { value: "/a" } });
    expect(btn).toBeEnabled();
  });
});

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { HooksView } from "./HooksView";

vi.mock("../api", () => ({
  listHooks: vi.fn(),
  getHookRuleJson: vi.fn(),
  toggleHook: vi.fn(),
  updateHookRule: vi.fn(),
  createHook: vi.fn(),
  deleteHook: vi.fn(),
  getClaudeCodeSettingsPath: vi.fn(() => Promise.resolve("/x/settings.json")),
  getStorageDir: vi.fn(() => Promise.resolve("/x/storage")),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  revealItemInDir: vi.fn(),
  openUrl: vi.fn(),
}));

import { listHooks, getHookRuleJson, toggleHook } from "../api";

const HOOK_ON = {
  id: "PreToolUse::*",
  event: "PreToolUse",
  matcher: "*",
  commands: ["lint"],
  handlerTypes: ["command"],
  isEnabled: true,
};

const HOOK_OFF = { ...HOOK_ON, isEnabled: false };

const INITIAL_JSON = '{"matcher":"*","hooks":[]}';
const EDITED_JSON = '{"matcher":"*","hooks":[{"type":"command","command":"edited"}]}';

describe("HooksView", () => {
  beforeEach(() => {
    vi.mocked(listHooks).mockReset();
    vi.mocked(getHookRuleJson).mockReset();
    vi.mocked(toggleHook).mockReset();
    vi.mocked(listHooks).mockResolvedValue([HOOK_ON] as any);
    vi.mocked(getHookRuleJson).mockResolvedValue(INITIAL_JSON);
    vi.mocked(toggleHook).mockResolvedValue(undefined as any);
  });

  it("keeps unsaved edits in the editor after a Disable refresh", async () => {
    render(<HooksView mode="cli" onMutated={() => {}} />);

    // Select the rule — this triggers the one-and-only getHookRuleJson fetch.
    await waitFor(() => screen.getByText("PreToolUse"));
    fireEvent.click(screen.getByText("PreToolUse"));

    const textarea = await screen.findByDisplayValue(INITIAL_JSON);

    // Make an unsaved edit.
    fireEvent.change(textarea, { target: { value: EDITED_JSON } });
    expect(textarea).toHaveValue(EDITED_JSON);

    // Simulate the backend reporting the rule disabled once refresh() re-lists.
    vi.mocked(listHooks).mockResolvedValue([HOOK_OFF] as any);

    fireEvent.click(screen.getByRole("button", { name: "Disable" }));

    // Wait for the refresh to land (button flips to "Enable").
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Enable" })).toBeInTheDocument()
    );

    // The unsaved edit must survive the refresh, and the JSON must not have
    // been re-fetched a second time (the rule's identity didn't change).
    expect(textarea).toHaveValue(EDITED_JSON);
    expect(getHookRuleJson).toHaveBeenCalledTimes(1);
  });
});

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { PrerequisiteRow } from "./PrerequisiteRow";

const ENTRY = {
  type: "node" as const,
  status: null,
  installUrl: "https://nodejs.org",
};

describe("PrerequisiteRow", () => {
  it("renders unchecked state with install link when not installed", () => {
    render(<PrerequisiteRow entry={ENTRY} onCheck={() => {}} onOpenUrl={() => {}} />);
    expect(screen.getByText(/node/i)).toBeInTheDocument();
    expect(screen.getByText(/Open install page/i)).toBeInTheDocument();
  });

  it("renders green state with version when installed", () => {
    render(<PrerequisiteRow
      entry={{ ...ENTRY, status: { installed: true, version: "v20.11", source: "system" } }}
      onCheck={() => {}} onOpenUrl={() => {}}
    />);
    expect(screen.getByText(/v20.11/)).toBeInTheDocument();
    expect(screen.queryByText(/Open install page/i)).not.toBeInTheDocument();
  });

  it("calls onCheck when re-check button clicked", () => {
    const onCheck = vi.fn();
    render(<PrerequisiteRow entry={ENTRY} onCheck={onCheck} onOpenUrl={() => {}} />);
    fireEvent.click(screen.getByText(/Re-check/i));
    expect(onCheck).toHaveBeenCalledWith("node");
  });

  it("calls onOpenUrl with the install URL when link clicked", () => {
    const onOpenUrl = vi.fn();
    render(<PrerequisiteRow entry={ENTRY} onCheck={() => {}} onOpenUrl={onOpenUrl} />);
    fireEvent.click(screen.getByText(/Open install page/i));
    expect(onOpenUrl).toHaveBeenCalledWith("https://nodejs.org");
  });
});

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { InstallProgress } from "./InstallProgress";

describe("InstallProgress", () => {
  it("shows the current step label", () => {
    render(<InstallProgress label="Pulling image…" log={[]} />);
    expect(screen.getByText(/Pulling image/)).toBeInTheDocument();
  });

  it("renders log lines in order, last 10 only", () => {
    const lines = Array.from({ length: 15 }, (_, i) => `line-${i}`);
    render(<InstallProgress label="…" log={lines} />);
    expect(screen.queryByText("line-0")).not.toBeInTheDocument();
    expect(screen.getByText("line-14")).toBeInTheDocument();
    expect(screen.getByText("line-5")).toBeInTheDocument();
  });
});

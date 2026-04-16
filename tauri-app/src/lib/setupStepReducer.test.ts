import { describe, it, expect } from "vitest";
import {
  initialSetupState, setupReducer, installEnabled, type SetupState,
} from "./setupStepReducer";
import type { InstallSchema } from "../types";

const SCHEMA: InstallSchema = {
  prerequisites: [{ type: "node", status: null, installUrl: "https://nodejs.org" }],
  configFields: [{
    name: "paths", kind: "argSpread", type: "pathArray", label: "Paths", required: true,
  }],
  installStepCount: 1,
  hasUnknownInstallStep: false,
};

describe("setupReducer", () => {
  it("starts in idle state", () => {
    expect(initialSetupState.phase).toBe("idle");
  });

  it("loads schema and moves to prereqsPending", () => {
    const next = setupReducer(initialSetupState, { type: "loaded", schema: SCHEMA });
    expect(next.phase).toBe("prereqsPending");
    expect(next.schema).toEqual(SCHEMA);
  });

  it("skips prereqsPending when schema has no prerequisites", () => {
    const schemaNoPrereqs = { ...SCHEMA, prerequisites: [] };
    const next = setupReducer(initialSetupState, { type: "loaded", schema: schemaNoPrereqs });
    expect(next.phase).toBe("fieldsPending");
  });

  it("marks prereq satisfied and transitions to fieldsPending when all green", () => {
    const loaded = setupReducer(initialSetupState, { type: "loaded", schema: SCHEMA });
    const next = setupReducer(loaded, {
      type: "prereqStatus", runtime: "node",
      status: { installed: true, version: "v20", source: "system" },
    });
    expect(next.phase).toBe("fieldsPending");
  });

  it("install button enables only when fields valid AND prereqs green", () => {
    let s: SetupState = setupReducer(initialSetupState, { type: "loaded", schema: SCHEMA });
    expect(installEnabled(s)).toBe(false);

    s = setupReducer(s, {
      type: "prereqStatus", runtime: "node",
      status: { installed: true, version: "v20", source: "system" },
    });
    expect(installEnabled(s)).toBe(false); // no paths yet

    s = setupReducer(s, { type: "fieldChange", name: "paths", value: ["/a"] });
    expect(installEnabled(s)).toBe(true);
  });

  it("retry after error preserves prereqs and field values", () => {
    let s: SetupState = setupReducer(initialSetupState, { type: "loaded", schema: SCHEMA });
    s = setupReducer(s, {
      type: "prereqStatus", runtime: "node",
      status: { installed: true, version: "v20", source: "system" },
    });
    s = setupReducer(s, { type: "fieldChange", name: "paths", value: ["/a"] });
    s = setupReducer(s, { type: "installStarted" });
    s = setupReducer(s, { type: "installError", message: "boom", canRetry: true });

    const retried = setupReducer(s, { type: "installRetry" });
    expect(retried.phase).toBe("ready");
    expect(retried.fieldValues.paths).toEqual(["/a"]);
    expect(retried.prereqStatus.node?.installed).toBe(true);
  });
});

import type { InstallSchema, RuntimeName, RuntimeStatus } from "../types";

export type SetupPhase =
  | "idle"
  | "loading"
  | "prereqsPending"
  | "fieldsPending"
  | "ready"
  | "installing"
  | "error"
  | "done";

export interface SetupState {
  phase: SetupPhase;
  schema: InstallSchema | null;
  prereqStatus: Partial<Record<RuntimeName, RuntimeStatus>>;
  fieldValues: Record<string, unknown>;
  errorMessage: string | null;
  errorCanRetry: boolean;
  log: string[];
}

export const initialSetupState: SetupState = {
  phase: "idle",
  schema: null,
  prereqStatus: {},
  fieldValues: {},
  errorMessage: null,
  errorCanRetry: false,
  log: [],
};

export type SetupAction =
  | { type: "load" }
  | { type: "loaded"; schema: InstallSchema }
  | { type: "prereqStatus"; runtime: RuntimeName; status: RuntimeStatus }
  | { type: "fieldChange"; name: string; value: unknown }
  | { type: "installStarted" }
  | { type: "installLog"; line: string }
  | { type: "installError"; message: string; canRetry: boolean }
  | { type: "installRetry" }
  | { type: "installDone" };

export function setupReducer(state: SetupState, action: SetupAction): SetupState {
  switch (action.type) {
    case "load":
      return { ...state, phase: "loading" };
    case "loaded":
      return {
        ...state,
        phase: action.schema.prerequisites.length > 0 ? "prereqsPending" : "fieldsPending",
        schema: action.schema,
      };
    case "prereqStatus": {
      const next = { ...state.prereqStatus, [action.runtime]: action.status };
      const allGreen = (state.schema?.prerequisites ?? []).every(
        (p) => next[p.type]?.installed,
      );
      return {
        ...state,
        prereqStatus: next,
        phase: allGreen ? phaseAfterPrereqs(state, next) : "prereqsPending",
      };
    }
    case "fieldChange": {
      const fieldValues = { ...state.fieldValues, [action.name]: action.value };
      return { ...state, fieldValues, phase: phaseAfterFieldChange(state, fieldValues) };
    }
    case "installStarted":
      return { ...state, phase: "installing", log: [], errorMessage: null };
    case "installLog":
      return { ...state, log: [...state.log, action.line] };
    case "installError":
      return { ...state, phase: "error", errorMessage: action.message, errorCanRetry: action.canRetry };
    case "installRetry":
      return { ...state, phase: "ready", errorMessage: null, errorCanRetry: false, log: [] };
    case "installDone":
      return { ...state, phase: "done" };
  }
}

function phaseAfterPrereqs(state: SetupState, _prereqStatus: SetupState["prereqStatus"]): SetupPhase {
  return validateFields(state.schema, state.fieldValues).valid ? "ready" : "fieldsPending";
}

function phaseAfterFieldChange(state: SetupState, fieldValues: Record<string, unknown>): SetupPhase {
  if (state.phase !== "fieldsPending" && state.phase !== "ready") return state.phase;
  const allPrereqsGreen = (state.schema?.prerequisites ?? []).every(
    (p) => state.prereqStatus[p.type]?.installed,
  );
  if (!allPrereqsGreen) return "prereqsPending";
  return validateFields(state.schema, fieldValues).valid ? "ready" : "fieldsPending";
}

export function validateFields(
  schema: InstallSchema | null,
  values: Record<string, unknown>,
): { valid: boolean; missing: string[] } {
  const missing: string[] = [];
  for (const f of schema?.configFields ?? []) {
    if (!f.required) continue;
    const v = values[f.name];
    if (v === undefined || v === null || v === "") missing.push(f.name);
    if (Array.isArray(v) && v.length === 0) missing.push(f.name);
  }
  return { valid: missing.length === 0, missing };
}

export function installEnabled(state: SetupState): boolean {
  return state.phase === "ready";
}

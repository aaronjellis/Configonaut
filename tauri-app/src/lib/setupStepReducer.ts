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
  /// The name the server was installed under (may differ from the catalog
  /// id when a collision was resolved with a -2 / -3 suffix). Carried in
  /// the installDone action so phase and name can't diverge.
  installedName: string | null;
}

export const initialSetupState: SetupState = {
  phase: "idle",
  schema: null,
  prereqStatus: {},
  fieldValues: {},
  errorMessage: null,
  errorCanRetry: false,
  log: [],
  installedName: null,
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
  | { type: "installDone"; installedName: string };

export function setupReducer(state: SetupState, action: SetupAction): SetupState {
  switch (action.type) {
    case "load":
      return { ...state, phase: "loading" };
    case "loaded": {
      const hasPrereqs = action.schema.prerequisites.length > 0;
      // Seed fieldValues with any schema-provided defaults so required
      // fields with sensible defaults don't strand the Install button in
      // "fieldsPending" purgatory waiting for the user to manually
      // re-type a value the catalog already supplied.
      const seededValues: Record<string, unknown> = { ...state.fieldValues };
      for (const f of action.schema.configFields) {
        if (seededValues[f.name] === undefined && f.default !== undefined && f.default !== null) {
          seededValues[f.name] = f.default;
        }
      }
      const fieldsReady = validateFields(action.schema, seededValues).valid;
      return {
        ...state,
        phase: hasPrereqs ? "prereqsPending" : fieldsReady ? "ready" : "fieldsPending",
        schema: action.schema,
        fieldValues: seededValues,
      };
    }
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
      return { ...state, phase: "done", installedName: action.installedName };
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

import type { ConfigField as Field } from "../types";

interface Props {
  field: Field;
  value: unknown;
  onChange: (next: unknown) => void;
  error?: string;
}

export function ConfigField({ field, value, onChange, error }: Props) {
  if (field.type === "pathArray") {
    return <PathArrayField field={field} value={(value as string[]) ?? []} onChange={onChange} error={error} />;
  }

  const inputType =
    field.type === "secret" ? "password" :
    field.type === "number" ? "number" :
    field.type === "url" ? "url" : "text";

  return (
    <label className={`config-field ${error ? "config-field--error" : ""}`}>
      <span className="config-field-label">
        {field.label}
        {field.required && <span className="config-field-req"> *</span>}
      </span>
      {field.description && <span className="config-field-desc">{field.description}</span>}
      <input
        type={inputType}
        aria-label={field.label}
        placeholder={field.placeholder ?? ""}
        value={(value as string) ?? ""}
        onChange={(e) => onChange(e.target.value)}
      />
      {error && <span className="config-field-error">{error}</span>}
    </label>
  );
}

function PathArrayField({
  field, value, onChange, error,
}: {
  field: Field; value: string[]; onChange: (v: unknown) => void; error?: string;
}) {
  const update = (idx: number, v: string) => {
    const next = [...value];
    next[idx] = v;
    onChange(next);
  };
  const add = () => onChange([...value, ""]);
  const remove = (idx: number) => onChange(value.filter((_, i) => i !== idx));

  return (
    <div className={`config-field config-field--array ${error ? "config-field--error" : ""}`}>
      <div className="config-field-label">
        {field.label}
        {field.required && <span className="config-field-req"> *</span>}
      </div>
      {field.description && <span className="config-field-desc">{field.description}</span>}
      {value.map((v, i) => (
        <div key={i} className="path-row">
          <input
            type="text"
            value={v}
            placeholder={field.placeholder ?? "/path/to/dir"}
            onChange={(e) => update(i, e.target.value)}
          />
          <button
            type="button"
            aria-label="Remove path"
            className="link-button"
            onClick={() => remove(i)}
          >
            ×
          </button>
        </div>
      ))}
      <button type="button" className="link-button" onClick={add}>
        + Add path
      </button>
      {error && <span className="config-field-error">{error}</span>}
    </div>
  );
}

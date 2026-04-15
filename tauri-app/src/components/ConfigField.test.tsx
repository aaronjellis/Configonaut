import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ConfigField } from "./ConfigField";
import type { ConfigField as ConfigFieldType } from "../types";

const STRING_FIELD: ConfigFieldType = {
  name: "api_key", kind: "env", type: "string", label: "API Key", required: true,
};
const SECRET_FIELD: ConfigFieldType = {
  name: "token", kind: "env", type: "secret", label: "Token", required: true,
};
const PATH_ARRAY: ConfigFieldType = {
  name: "paths", kind: "argSpread", type: "pathArray", label: "Paths", required: true,
};

describe("ConfigField", () => {
  it("renders a text input for type=string", () => {
    render(<ConfigField field={STRING_FIELD} value="" onChange={() => {}} />);
    expect(screen.getByLabelText(/API Key/i)).toHaveAttribute("type", "text");
  });

  it("renders a password input for type=secret", () => {
    render(<ConfigField field={SECRET_FIELD} value="" onChange={() => {}} />);
    expect(screen.getByLabelText(/Token/i)).toHaveAttribute("type", "password");
  });

  it("emits onChange with new value on input", () => {
    const onChange = vi.fn();
    render(<ConfigField field={STRING_FIELD} value="" onChange={onChange} />);
    fireEvent.change(screen.getByLabelText(/API Key/i), { target: { value: "abc" } });
    expect(onChange).toHaveBeenCalledWith("abc");
  });

  it("renders pathArray as add/remove rows", () => {
    const onChange = vi.fn();
    render(<ConfigField field={PATH_ARRAY} value={["/a"]} onChange={onChange} />);
    expect(screen.getByDisplayValue("/a")).toBeInTheDocument();
    fireEvent.click(screen.getByText(/\+ Add path/i));
    expect(onChange).toHaveBeenCalledWith(["/a", ""]);
  });

  it("removes a row from pathArray", () => {
    const onChange = vi.fn();
    render(<ConfigField field={PATH_ARRAY} value={["/a", "/b"]} onChange={onChange} />);
    fireEvent.click(screen.getAllByLabelText(/Remove path/i)[0]);
    expect(onChange).toHaveBeenCalledWith(["/b"]);
  });

  it("renders a number input for type=number", () => {
    const field: ConfigFieldType = {
      name: "port", kind: "env", type: "number", label: "Port", required: false,
    };
    render(<ConfigField field={field} value="3000" onChange={() => {}} />);
    expect(screen.getByLabelText(/Port/i)).toHaveAttribute("type", "number");
  });

  it("renders a url input for type=url", () => {
    const field: ConfigFieldType = {
      name: "endpoint", kind: "env", type: "url", label: "Endpoint", required: false,
    };
    render(<ConfigField field={field} value="" onChange={() => {}} />);
    expect(screen.getByLabelText(/Endpoint/i)).toHaveAttribute("type", "url");
  });
});

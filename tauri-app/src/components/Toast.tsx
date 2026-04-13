// Global toast notification system.
//
// Usage:
//   const toast = useToast();
//   toast.show("Server added.", "success");
//
// Wraps the app via <ToastProvider> in App.tsx. Toasts stack at the
// bottom-right, auto-dismiss after 3s, and can be clicked to dismiss early.

import {
  createContext,
  useCallback,
  useContext,
  useRef,
  useState,
  type ReactNode,
} from "react";

export type ToastVariant = "success" | "error" | "warning" | "info";

interface ToastItem {
  id: number;
  message: string;
  variant: ToastVariant;
}

interface ToastContext {
  show: (message: string, variant?: ToastVariant) => void;
}

const Ctx = createContext<ToastContext>({ show: () => {} });

export function useToast() {
  return useContext(Ctx);
}

const AUTO_DISMISS_MS = 3000;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const nextId = useRef(0);

  const show = useCallback((message: string, variant: ToastVariant = "info") => {
    const id = nextId.current++;
    setToasts((prev) => [...prev, { id, message, variant }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, AUTO_DISMISS_MS);
  }, []);

  const dismiss = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  return (
    <Ctx.Provider value={{ show }}>
      {children}
      {toasts.length > 0 && (
        <div className="toast-stack">
          {toasts.map((t) => (
            <div
              key={t.id}
              className={`toast toast-${t.variant}`}
              onClick={() => dismiss(t.id)}
              role="status"
            >
              <span className="toast-icon">{iconFor(t.variant)}</span>
              <span className="toast-msg">{t.message}</span>
            </div>
          ))}
        </div>
      )}
    </Ctx.Provider>
  );
}

function iconFor(v: ToastVariant): string {
  switch (v) {
    case "success": return "✓";
    case "error":   return "✕";
    case "warning": return "⚠";
    case "info":    return "ℹ";
  }
}

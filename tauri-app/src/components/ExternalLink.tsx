// A link that opens in the OS browser. Rendered as a <button>, not an <a>:
// a real anchor's href can navigate the Tauri main frame away from the app
// via middle-click or the context menu's "Open Link", and plain
// target="_blank" clicks are swallowed by the webview anyway.
import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

interface Props {
  href: string;
  className?: string;
  title?: string;
  children: ReactNode;
}

export function ExternalLink({ href, className, title, children }: Props) {
  return (
    <button
      type="button"
      className={className ?? "link-button"}
      title={title ?? href}
      onClick={(e) => {
        e.stopPropagation();
        void openUrl(href);
      }}
    >
      {children}
    </button>
  );
}

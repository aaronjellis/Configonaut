// App shell. Responsibilities:
//   • Holds the current sidebar selection and AppMode.
//   • Renders the macOS titlebar drag strip (the window has
//     titleBarStyle=Overlay + hiddenTitle so we need our own drag handle).
//   • Tracks simple badge counts to show in the sidebar, refreshed when
//     the mode changes or a view pokes us via the `onBadgesChange` prop.
//
// Each view is stateless and re-fetches from Rust when the mode flips.

import { useCallback, useEffect, useMemo, useState } from "react";
import "./App.css";
import { Sidebar } from "./components/Sidebar";
import { AboutModal } from "./components/AboutModal";
import { useToast } from "./components/Toast";
import { UpdateModal } from "./components/UpdateModal";
import {
  listAgents,
  listBackups,
  listHooks,
  listServers,
  listSkills,
} from "./api";
import { AgentsView } from "./views/AgentsView";
import { BackupsView } from "./views/BackupsView";
import { HooksView } from "./views/HooksView";
import { McpServersView } from "./views/McpServersView";
import { SkillsView } from "./views/SkillsView";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { listen } from "@tauri-apps/api/event";
import type { AppMode, ViewKey } from "./types";

const MODE_KEY = "configonaut.mode";
const VIEW_KEY = "configonaut.view";

function loadMode(): AppMode {
  return localStorage.getItem(MODE_KEY) === "cli" ? "cli" : "desktop";
}
function loadView(): ViewKey {
  const v = localStorage.getItem(VIEW_KEY);
  if (v === "mcp" || v === "hooks" || v === "agents" || v === "skills" || v === "backups") {
    return v;
  }
  return "mcp";
}

function App() {
  const [mode, setMode] = useState<AppMode>(loadMode);
  const [view, setView] = useState<ViewKey>(loadView);
  const [badges, setBadges] = useState<Partial<Record<ViewKey, number>>>({});
  const [refreshTick, setRefreshTick] = useState(0);
  const [pendingUpdate, setPendingUpdate] = useState<Update | null>(null);
  const [showUpdateModal, setShowUpdateModal] = useState(false);
  const [showAbout, setShowAbout] = useState(false);
  const toast = useToast();

  useEffect(() => {
    localStorage.setItem(MODE_KEY, mode);
  }, [mode]);
  useEffect(() => {
    localStorage.setItem(VIEW_KEY, view);
  }, [view]);

  // Refresh badge counts whenever the mode changes or a view signals that
  // something on disk moved. Each fetch is wrapped in its own catch so one
  // failing endpoint (e.g. Claude Code settings missing) doesn't knock out
  // the badges for other sections.
  const refreshBadges = useCallback(async () => {
    const [listingR, backupsR, hooksR, agentsR, skillsR] = await Promise.all([
      listServers(mode).catch(() => null),
      listBackups(mode).catch(() => null),
      listHooks().catch(() => null),
      listAgents().catch(() => null),
      listSkills().catch(() => null),
    ]);

    setBadges({
      mcp: listingR?.activeServers.length,
      backups: backupsR?.length,
      // Hooks: count enabled rules only.
      hooks: hooksR?.filter((h) => h.isEnabled).length,
      // Agents: personal agents plus enabled plugin agents.
      agents: agentsR?.filter(
        (a) => a.source === "personal" || a.isPluginEnabled
      ).length,
      // Skills: enabled commands/skills (including plugin skills whose
      // plugin is on).
      skills: skillsR?.filter((s) => s.isEnabled).length,
    });
  }, [mode]);

  useEffect(() => {
    refreshBadges();
  }, [refreshBadges, refreshTick]);

  const bumpRefresh = useCallback(
    () => setRefreshTick((t) => t + 1),
    []
  );

  // Check for app updates on startup (fires during the splash phase since
  // the main webview loads in the background).
  useEffect(() => {
    check()
      .then((update) => {
        if (update) {
          setPendingUpdate(update);
          setShowUpdateModal(true);
        }
      })
      .catch(console.error);
  }, []);

  const handleCheckForUpdates = useCallback(async () => {
    try {
      const update = await check();
      if (update) {
        setPendingUpdate(update);
        setShowUpdateModal(true);
      } else {
        toast.show("You\u2019re on the latest version\u2026 for now.", "success");
      }
    } catch (err) {
      console.error("Update check failed:", err);
      toast.show("Couldn\u2019t reach the update server.", "warning");
    }
  }, [toast]);

  // Listen for native menu events.
  useEffect(() => {
    const unlistenUpdate = listen("check-for-updates", () => {
      handleCheckForUpdates();
    });
    const unlistenAbout = listen("show-about", () => {
      setShowAbout(true);
    });
    return () => {
      unlistenUpdate.then((fn) => fn());
      unlistenAbout.then((fn) => fn());
    };
  }, [handleCheckForUpdates]);

  const body = useMemo(() => {
    switch (view) {
      case "mcp":
        return <McpServersView mode={mode} onMutated={bumpRefresh} />;
      case "backups":
        return <BackupsView mode={mode} onMutated={bumpRefresh} />;
      case "hooks":
        return <HooksView mode={mode} onMutated={bumpRefresh} />;
      case "agents":
        return <AgentsView mode={mode} onMutated={bumpRefresh} />;
      case "skills":
        return <SkillsView mode={mode} onMutated={bumpRefresh} />;
    }
  }, [view, mode, bumpRefresh]);

  return (
    <div className="app-root">
      {/* Full-width invisible drag strip across the top of the window.
          Sits above both the sidebar and the main pane so the user can
          grab ANY part of the titlebar zone — not just the narrow inset
          next to the traffic lights — and drag the window around. The
          macOS traffic-light buttons still receive clicks because
          they're drawn by the OS on top of the webview.

          Gotcha: `data-tauri-drag-region` is silently a no-op unless
          `core:window:allow-start-dragging` is in the capability file.
          `core:default` does NOT include it. See
          src-tauri/capabilities/default.json. */}
      <div className="app-titlebar-drag" data-tauri-drag-region />
      <Sidebar
        currentView={view}
        onViewChange={setView}
        mode={mode}
        onModeChange={setMode}
        badges={badges}
        version="0.3.1"
      />
      <main className="main">{body}</main>
      {showUpdateModal && pendingUpdate && (
        <UpdateModal
          update={pendingUpdate}
          onDismiss={() => setShowUpdateModal(false)}
        />
      )}
      {showAbout && (
        <AboutModal
          version="0.3.1"
          onCheckForUpdates={handleCheckForUpdates}
          onDismiss={() => setShowAbout(false)}
        />
      )}
    </div>
  );
}

export default App;

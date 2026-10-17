import { useEffect, useMemo, useState } from "react";

import { type AppInfo, getAppInfo } from "./appInfo";
import {
  appManagerStatusText,
  getAppManagerStatus,
  syncAppManager,
  type AppManagerRuntimeStatus,
} from "./appManager";
import { AppsPage } from "./AppsPage";
import { DriverCenterPage } from "./DriverCenterPage";
import { DuplicateFinderPage } from "./DuplicateFinderPage";
import { HealthCheckPage } from "./HealthCheckPage";
import { InventorySummary } from "./InventorySummary";
import { MonitorPage } from "./MonitorPage";
import { PerformanceOptimizerPage } from "./PerformanceOptimizerPage";
import { SmartCleanPage } from "./SmartCleanPage";
import { RestorePage } from "./RestorePage";
import { StartupPage } from "./StartupPage";
import { StoragePage } from "./StoragePage";
import { SoftwareUpdaterPage } from "./SoftwareUpdaterPage";
import { NAV_ITEMS, getNavItem, type PageId } from "./navigation";
import {
  readManagedModeEnabled,
  readSidebarCollapsed,
  readThemePreference,
  resolveTheme,
  type ThemePreference,
  writeManagedModeEnabled,
  writeSidebarCollapsed,
  writeThemePreference,
} from "./preferences";
import "./styles.css";

type BridgeState =
  | { status: "loading" }
  | { status: "ready"; info: AppInfo }
  | { status: "error"; message: string };

const healthCategories = [
  {
    name: "Storage",
    text: "Health Check now evaluates real free-capacity evidence.",
  },
  {
    name: "Performance",
    text: "Health Check now evaluates real memory-pressure evidence.",
  },
  {
    name: "Security",
    text: "Unavailable in P3 until a supported security check is implemented.",
  },
  {
    name: "Updates",
    text: "Unavailable in P3 until trusted update checks are implemented.",
  },
  {
    name: "Privacy",
    text: "Unavailable in P3 until evidence-backed privacy checks are implemented.",
  },
];

function BridgeBadge({ state }: { state: BridgeState }) {
  if (state.status === "loading") {
    return (
      <span className="status-badge neutral">Checking native bridge…</span>
    );
  }

  if (state.status === "error") {
    return (
      <span className="status-badge warning" title={state.message}>
        Native bridge unavailable
      </span>
    );
  }

  return <span className="status-badge good">Native bridge connected</span>;
}

function AppManagerBadge({
  state,
  managedEnabled,
}: {
  state: AppManagerRuntimeStatus | null;
  managedEnabled: boolean;
}) {
  if (!managedEnabled) {
    return <span className="status-badge neutral">Standalone · Offline</span>;
  }
  if (!state) {
    return <span className="status-badge neutral">App Manager checking…</span>;
  }

  const label = appManagerStatusText(state);
  const className =
    state.connection === "online" &&
    state.device?.status === "approved" &&
    state.device?.entitlementState !== "disabled"
      ? "status-badge good"
      : state.connection === "notConfigured"
        ? "status-badge neutral"
        : "status-badge warning";

  return (
    <span className={className} title={state.lastError ?? undefined}>
      App Manager: {label}
    </span>
  );
}

function OverviewPage({
  onNavigate,
  bridgeState,
}: {
  onNavigate: (page: PageId) => void;
  bridgeState: BridgeState;
}) {
  const appInfo = bridgeState.status === "ready" ? bridgeState.info : null;

  return (
    <div className="page-stack">
      <section className="hero-card" aria-labelledby="overview-heading">
        <div>
          <p className="eyebrow">PC health</p>
          <h2 id="overview-heading">Health Check V1 is ready</h2>
          <p className="muted">
            Run an explainable, read-only scan using the real Windows inventory.
            Unsupported categories stay unavailable instead of being guessed.
          </p>
        </div>
        <button className="primary-action" onClick={() => onNavigate("health")}>
          Open Health Check
        </button>
      </section>

      <InventorySummary />

      <section aria-labelledby="health-categories-heading">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Health categories</p>
            <h3 id="health-categories-heading">
              What PC Manager will evaluate
            </h3>
          </div>
          <span className="status-badge neutral">2 of 5 supported in P3</span>
        </div>

        <div className="category-grid">
          {healthCategories.map((category) => (
            <article className="metric-card" key={category.name}>
              <div className="metric-card-topline">
                <h4>{category.name}</h4>
                <span className="dot" aria-hidden="true" />
              </div>
              <p>{category.text}</p>
              <span className="metric-state">Awaiting data</span>
            </article>
          ))}
        </div>
      </section>

      <section aria-labelledby="quick-actions-heading">
        <div className="section-heading">
          <div>
            <p className="eyebrow">Quick actions</p>
            <h3 id="quick-actions-heading">Choose where to go next</h3>
          </div>
        </div>

        <div className="quick-actions">
          {(
            [
              ["health", "Health Check", "Review explainable system findings."],
              ["clean", "Smart Clean", "Preview safe cleanup candidates."],
              ["startup", "Startup", "Inspect Windows startup sources."],
              ["storage", "Storage", "Understand disk usage."],
            ] as const
          ).map(([page, title, description]) => (
            <button
              className="quick-action"
              key={page}
              onClick={() => onNavigate(page)}
            >
              <span>{title}</span>
              <small>{description}</small>
            </button>
          ))}
        </div>
      </section>

      <section
        className="native-status-card"
        aria-labelledby="native-status-heading"
      >
        <div>
          <p className="eyebrow">Application foundation</p>
          <h3 id="native-status-heading">Native shell status</h3>
        </div>
        <BridgeBadge state={bridgeState} />
        <dl>
          <div>
            <dt>App ID</dt>
            <dd>{appInfo?.appId ?? "Unavailable"}</dd>
          </div>
          <div>
            <dt>Platform</dt>
            <dd>{appInfo?.platform ?? "Unavailable"}</dd>
          </div>
          <div>
            <dt>Device type</dt>
            <dd>{appInfo?.deviceType ?? "Unavailable"}</dd>
          </div>
          <div>
            <dt>Product phase</dt>
            <dd>{appInfo?.phase ?? "Unavailable"}</dd>
          </div>
        </dl>
      </section>
    </div>
  );
}

function PlaceholderPage({
  page,
}: {
  page: Exclude<
    PageId,
    | "overview"
    | "health"
    | "clean"
    | "startup"
    | "apps"
    | "updates"
    | "drivers"
    | "storage"
    | "duplicates"
    | "monitor"
    | "performance"
    | "restore"
    | "settings"
  >;
}) {
  const item = getNavItem(page);

  return (
    <div className="page-stack">
      <section className="placeholder-card">
        <div className="placeholder-icon" aria-hidden="true">
          {item.icon}
        </div>
        <p className="eyebrow">Planned module</p>
        <h2>{item.label}</h2>
        <p className="muted">{item.description}</p>
        <div className="info-callout">
          <strong>No system data is being simulated.</strong>
          <span>
            This module is not implemented in P11. Real actions arrive only in
            later, separately gated phases.
          </span>
        </div>
      </section>
    </div>
  );
}

function themeAccessibilityLabel(
  option: ThemePreference,
  selected: boolean,
): string {
  const name = option[0].toUpperCase() + option.slice(1);
  const state = selected ? "selected" : "not selected";
  return `${name} theme ${state}`;
}

function SettingsPage({
  theme,
  onThemeChange,
  sidebarCollapsed,
  onSidebarChange,
  appearanceError,
  appManagerState,
  managedEnabled,
  onManagedModeChange,
  managedModeError,
}: {
  theme: ThemePreference;
  onThemeChange: (theme: ThemePreference) => void;
  sidebarCollapsed: boolean;
  onSidebarChange: (collapsed: boolean) => void;
  appearanceError: string | null;
  appManagerState: AppManagerRuntimeStatus | null;
  managedEnabled: boolean;
  onManagedModeChange: (enabled: boolean) => void;
  managedModeError: string | null;
}) {
  return (
    <div className="page-stack">
      <section className="settings-card">
        <p className="eyebrow">Appearance</p>
        <h2>Settings</h2>
        <p className="muted">
          PC Manager stores only harmless local interface preferences here. No
          Windows system settings are changed from this screen.
        </p>
        <p className="muted">
          <strong>Offline-first state on this device</strong>
          : Your theme, sidebar layout and central-management choice stay in
          this Windows user profile. No account is required.
        </p>

        <fieldset className="setting-group">
          <legend>Theme</legend>
          <div className="segmented-control">
            {(["system", "dark", "light"] as const).map((option) => (
              <button
                aria-label={themeAccessibilityLabel(option, theme === option)}
                aria-pressed={theme === option}
                className={theme === option ? "selected" : ""}
                key={option}
                onClick={() => onThemeChange(option)}
                type="button"
              >
                {option[0].toUpperCase() + option.slice(1)}
              </button>
            ))}
          </div>
        </fieldset>

        <div className="setting-row">
          <div>
            <strong>Compact sidebar</strong>
            <span>Keep navigation narrow and show labels on hover/title.</span>
          </div>
          <button
            aria-label={`Compact sidebar ${sidebarCollapsed ? "on" : "off"}`}
            aria-pressed={sidebarCollapsed}
            className="toggle-button"
            onClick={() => onSidebarChange(!sidebarCollapsed)}
            type="button"
          >
            {sidebarCollapsed ? "On" : "Off"}
          </button>
        </div>
        {appearanceError && (
          <div className="info-callout health-error" role="alert">
            <strong>Could not save appearance setting</strong>
            <span>{appearanceError}</span>
          </div>
        )}
      </section>

      <section
        className="native-status-card"
        aria-labelledby="app-manager-heading"
      >
        <div>
          <p className="eyebrow">Central management — P8</p>
          <h2 id="app-manager-heading">App Manager</h2>
          <p className="muted">
            Local maintenance works without an account or Internet connection.
            Turn on managed mode only when you want optional outbound App
            Manager coordination. No inbound port or remote shell is added.
          </p>
        </div>
        <div className="setting-row">
          <div>
            <strong>Central management</strong>
            <span>
              {managedEnabled
                ? "On: optional signed gateway sync is enabled."
                : "Off: standalone mode; no automatic gateway sync."}
            </span>
          </div>
          <button
            aria-label="Enable central management"
            aria-pressed={managedEnabled}
            className="toggle-button"
            onClick={() => onManagedModeChange(!managedEnabled)}
            type="button"
          >
            {managedEnabled ? "On" : "Off"}
          </button>
        </div>
        {managedModeError && (
          <div className="info-callout health-error" role="alert">
            <strong>Could not save central management setting</strong>
            <span>{managedModeError}</span>
          </div>
        )}
        <AppManagerBadge
          managedEnabled={managedEnabled}
          state={appManagerState}
        />
        <dl>
          <div>
            <dt>Device</dt>
            <dd>{appManagerState?.device?.deviceCode ?? "Unavailable"}</dd>
          </div>
          <div>
            <dt>Approval</dt>
            <dd>{appManagerState?.device?.status ?? "Unavailable"}</dd>
          </div>
          <div>
            <dt>Entitlement</dt>
            <dd>
              {appManagerState?.device?.entitlementState ?? "Unavailable"}
            </dd>
          </div>
          <div>
            <dt>Release channel</dt>
            <dd>{appManagerState?.device?.releaseChannel ?? "Unavailable"}</dd>
          </div>
        </dl>
        {managedEnabled && !appManagerState?.configured && (
          <div className="info-callout">
            <strong>Gateway not configured.</strong>
            <span>
              Set PC_MANAGER_APP_MANAGER_ORIGIN to the approved HTTPS
              Application Management origin. Local maintenance features continue
              to work without it.
            </span>
          </div>
        )}
        {managedEnabled && appManagerState?.lastError && (
          <div className="info-callout health-error">
            <strong>Last App Manager connection issue</strong>
            <span>{appManagerState.lastError}</span>
          </div>
        )}
      </section>
    </div>
  );
}

export default function App() {
  const [page, setPage] = useState<PageId>("overview");
  const [bridgeState, setBridgeState] = useState<BridgeState>({
    status: "loading",
  });
  const [theme, setTheme] = useState<ThemePreference>(() =>
    readThemePreference(),
  );
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() =>
    readSidebarCollapsed(),
  );
  const [appManagerState, setAppManagerState] =
    useState<AppManagerRuntimeStatus | null>(null);
  const [managedEnabled, setManagedEnabled] = useState(() =>
    readManagedModeEnabled(),
  );
  const [managedModeError, setManagedModeError] = useState<string | null>(null);
  const [appearanceError, setAppearanceError] = useState<string | null>(null);
  const [prefersDark, setPrefersDark] = useState(
    () => window.matchMedia("(prefers-color-scheme: dark)").matches,
  );

  const resolvedTheme = useMemo(
    () => resolveTheme(theme, prefersDark),
    [theme, prefersDark],
  );

  useEffect(() => {
    let active = true;

    getAppInfo()
      .then((info) => {
        if (active) {
          setBridgeState({ status: "ready", info });
        }
      })
      .catch((error: unknown) => {
        if (active) {
          setBridgeState({
            status: "error",
            message:
              error instanceof Error
                ? error.message
                : "Native bridge unavailable.",
          });
        }
      });

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (!managedEnabled) {
      // No status polling or remote sync in standalone mode.
      setAppManagerState(null);
      return;
    }

    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const schedule = (seconds: number) => {
      if (!active) return;
      const boundedSeconds = Math.max(5, Math.min(300, seconds));
      timer = setTimeout(() => {
        void synchronize();
      }, boundedSeconds * 1000);
    };

    const synchronize = async () => {
      try {
        const status = await syncAppManager();
        if (!active) return;
        setAppManagerState(status);
        schedule(status.retryAfterSeconds);
      } catch {
        if (!active) return;
        schedule(60);
      }
    };

    getAppManagerStatus()
      .then((status) => {
        if (active) setAppManagerState(status);
      })
      .catch(() => undefined);

    void synchronize();

    return () => {
      active = false;
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [managedEnabled]);

  useEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme;
  }, [resolvedTheme]);

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = (event: MediaQueryListEvent) =>
      setPrefersDark(event.matches);

    mediaQuery.addEventListener("change", onChange);
    return () => mediaQuery.removeEventListener("change", onChange);
  }, []);

  const updateTheme = (nextTheme: ThemePreference) => {
    if (!writeThemePreference(nextTheme)) {
      setAppearanceError(
        "Local storage is unavailable. Theme was not changed.",
      );
      return;
    }
    setAppearanceError(null);
    setTheme(nextTheme);
  };

  const updateSidebar = (collapsed: boolean) => {
    if (!writeSidebarCollapsed(collapsed)) {
      setAppearanceError(
        "Local storage is unavailable. Sidebar was not changed.",
      );
      return;
    }
    setAppearanceError(null);
    setSidebarCollapsed(collapsed);
  };

  const updateManagedMode = (enabled: boolean) => {
    if (!writeManagedModeEnabled(enabled)) {
      if (!enabled) {
        // Disabling optional network activity is fail-safe even if persistence
        // is denied. The previous saved ON may return after a later restart.
        setManagedEnabled(false);
        setAppManagerState(null);
        setManagedModeError(
          "Central management is OFF for this session, but OFF could not be saved. Check it again after restarting.",
        );
      } else {
        setManagedModeError(
          "Local preference storage is unavailable. Central management was not enabled.",
        );
      }
      return;
    }
    setManagedModeError(null);
    setAppManagerState(null);
    setManagedEnabled(enabled);
  };

  const activeItem = getNavItem(page);

  return (
    <div className={`app-shell ${sidebarCollapsed ? "sidebar-collapsed" : ""}`}>
      <aside className="sidebar" aria-label="Primary navigation">
        <div className="brand-row">
          <div className="brand-mark" aria-hidden="true">
            P
          </div>
          <div className="brand-copy">
            <strong>PC Manager</strong>
            <span>Desktop</span>
          </div>
          <button
            aria-label={
              sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"
            }
            className="icon-button sidebar-toggle"
            onClick={() => updateSidebar(!sidebarCollapsed)}
            title={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
            type="button"
          >
            {sidebarCollapsed ? "›" : "‹"}
          </button>
        </div>

        <nav className="nav-list">
          {NAV_ITEMS.map((item) => (
            <button
              aria-current={page === item.id ? "page" : undefined}
              className={page === item.id ? "nav-item active" : "nav-item"}
              key={item.id}
              onClick={() => setPage(item.id)}
              title={sidebarCollapsed ? item.label : undefined}
              type="button"
            >
              <span className="nav-icon" aria-hidden="true">
                {item.icon}
              </span>
              <span className="nav-label">{item.label}</span>
            </button>
          ))}
        </nav>

        <div className="sidebar-footer">
          <BridgeBadge state={bridgeState} />
          <AppManagerBadge
            managedEnabled={managedEnabled}
            state={appManagerState}
          />
        </div>
      </aside>

      <main className="main-column">
        <header className="topbar">
          <div>
            <p className="breadcrumb">PC Manager / {activeItem.shortLabel}</p>
            <h1>{activeItem.label}</h1>
          </div>
          <div className="topbar-actions">
            <button
              className="icon-button"
              onClick={() =>
                updateTheme(resolvedTheme === "dark" ? "light" : "dark")
              }
              title={
                resolvedTheme === "dark" ? "Use light theme" : "Use dark theme"
              }
              type="button"
            >
              {resolvedTheme === "dark" ? "☀" : "☾"}
            </button>
          </div>
        </header>

        <div className="content-area">
          {page === "overview" ? (
            <OverviewPage onNavigate={setPage} bridgeState={bridgeState} />
          ) : page === "health" ? (
            <HealthCheckPage />
          ) : page === "clean" ? (
            <SmartCleanPage />
          ) : page === "startup" ? (
            <StartupPage />
          ) : page === "apps" ? (
            <AppsPage />
          ) : page === "updates" ? (
            <SoftwareUpdaterPage />
          ) : page === "drivers" ? (
            <DriverCenterPage />
          ) : page === "storage" ? (
            <StoragePage />
          ) : page === "duplicates" ? (
            <DuplicateFinderPage />
          ) : page === "monitor" ? (
            <MonitorPage />
          ) : page === "performance" ? (
            <PerformanceOptimizerPage onNavigate={setPage} />
          ) : page === "restore" ? (
            <RestorePage />
          ) : page === "settings" ? (
            <SettingsPage
              appearanceError={appearanceError}
              onSidebarChange={updateSidebar}
              onThemeChange={updateTheme}
              appManagerState={appManagerState}
              managedEnabled={managedEnabled}
              onManagedModeChange={updateManagedMode}
              managedModeError={managedModeError}
              sidebarCollapsed={sidebarCollapsed}
              theme={theme}
            />
          ) : (
            <PlaceholderPage page={page} />
          )}
        </div>
      </main>
    </div>
  );
}

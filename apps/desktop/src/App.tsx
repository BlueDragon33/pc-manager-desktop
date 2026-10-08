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
  readSidebarCollapsed,
  readThemePreference,
  resolveTheme,
  type ThemePreference,
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
    text: "Not yet assessed by Health Check; no security status is inferred.",
  },
  {
    name: "Updates",
    text: "Health Check does not yet include trusted update evidence; use Software Updater.",
  },
  {
    name: "Privacy",
    text: "Not yet assessed; no privacy score is inferred.",
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

function AppManagerBadge({ state }: { state: AppManagerRuntimeStatus | null }) {
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
          <h2 id="overview-heading">Check your PC with real evidence</h2>
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
              Health Check coverage
            </h3>
          </div>
          <span className="status-badge neutral">2 of 5 evaluated</span>
        </div>

        <div className="category-grid">
          {healthCategories.map((category) => (
            <article className="metric-card" key={category.name}>
              <div className="metric-card-topline">
                <h4>{category.name}</h4>
                <span className="dot" aria-hidden="true" />
              </div>
              <p>{category.text}</p>
              <span className="metric-state">Open Health Check to scan</span>
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
            This module is not implemented yet. Real actions arrive only in
            later, separately gated phases.
          </span>
        </div>
      </section>
    </div>
  );
}

function SettingsPage({
  theme,
  onThemeChange,
  sidebarCollapsed,
  onSidebarChange,
  appManagerState,
}: {
  theme: ThemePreference;
  onThemeChange: (theme: ThemePreference) => void;
  sidebarCollapsed: boolean;
  onSidebarChange: (collapsed: boolean) => void;
  appManagerState: AppManagerRuntimeStatus | null;
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

        <fieldset className="setting-group">
          <legend>Theme</legend>
          <div className="segmented-control">
            {(["system", "dark", "light"] as const).map((option) => (
              <button
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
            aria-pressed={sidebarCollapsed}
            className="toggle-button"
            onClick={() => onSidebarChange(!sidebarCollapsed)}
            type="button"
          >
            {sidebarCollapsed ? "On" : "Off"}
          </button>
        </div>
      </section>

      <section
        className="native-status-card"
        aria-labelledby="app-manager-heading"
      >
        <div>
          <p className="eyebrow">Central management — P8</p>
          <h2 id="app-manager-heading">App Manager</h2>
          <p className="muted">
            PC Manager connects outbound only. It does not open an inbound port,
            and remote actions are restricted to a typed allow-list.
          </p>
        </div>
        <AppManagerBadge state={appManagerState} />
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
        {!appManagerState?.configured && (
          <div className="info-callout">
            <strong>Gateway not configured.</strong>
            <span>
              Set PC_MANAGER_APP_MANAGER_ORIGIN to the approved HTTPS
              Application Management origin. Local maintenance features continue
              to work without it.
            </span>
          </div>
        )}
        {appManagerState?.lastError && (
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
  }, []);

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
    setTheme(nextTheme);
    writeThemePreference(nextTheme);
  };

  const updateSidebar = (collapsed: boolean) => {
    setSidebarCollapsed(collapsed);
    writeSidebarCollapsed(collapsed);
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
          <AppManagerBadge state={appManagerState} />
        </div>
      </aside>

      <main className="main-column">
        <header className="topbar">
          <div>
            <p className="breadcrumb">PC Manager / {activeItem.shortLabel}</p>
            <h1>{activeItem.label}</h1>
          </div>
          <div className="topbar-actions">
            <span className="phase-chip">P12</span>
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
              onSidebarChange={updateSidebar}
              onThemeChange={updateTheme}
              appManagerState={appManagerState}
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

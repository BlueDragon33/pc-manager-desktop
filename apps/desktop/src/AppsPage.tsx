import { useEffect, useMemo, useState } from "react";

import {
  filterInstalledApps,
  launchUninstall,
  listInstalledApps,
  sourceLabel,
  type InstalledAppEntry,
  type UninstallLaunchResult,
} from "./apps";

type LoadState =
  | { status: "loading" }
  | { status: "ready"; apps: InstalledAppEntry[] }
  | { status: "error"; message: string };

type ActionState =
  | { status: "idle" }
  | { status: "confirming"; app: InstalledAppEntry }
  | { status: "running"; app: InstalledAppEntry }
  | { status: "complete"; result: UninstallLaunchResult }
  | { status: "error"; message: string };

function errorMessage(error: unknown, fallback: string): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof (error as { message?: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }
  return error instanceof Error ? error.message : fallback;
}

function formatBytes(bytes: number | null): string {
  if (bytes === null || !Number.isFinite(bytes) || bytes < 0) {
    return "Size unavailable";
  }
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value >= 10 ? value.toFixed(1) : value.toFixed(2)} ${unit}`;
}

export function AppsPage() {
  const [loadState, setLoadState] = useState<LoadState>({ status: "loading" });
  const [actionState, setActionState] = useState<ActionState>({ status: "idle" });
  const [query, setQuery] = useState("");

  const load = () => {
    setLoadState({ status: "loading" });
    listInstalledApps()
      .then((apps) => setLoadState({ status: "ready", apps }))
      .catch((error: unknown) =>
        setLoadState({
          status: "error",
          message: errorMessage(error, "Installed applications could not be loaded."),
        }),
      );
  };

  useEffect(() => {
    load();
  }, []);

  const visibleApps = useMemo(
    () =>
      loadState.status === "ready"
        ? filterInstalledApps(loadState.apps, query)
        : [],
    [loadState, query],
  );

  const executeUninstall = async (app: InstalledAppEntry) => {
    setActionState({ status: "running", app });
    try {
      const result = await launchUninstall(app.id);
      setActionState({ status: "complete", result });
    } catch (error: unknown) {
      setActionState({
        status: "error",
        message: errorMessage(
          error,
          "The application's standard uninstall flow could not be opened.",
        ),
      });
    }
  };

  return (
    <div className="page-stack p6-page">
      <section className="p6-hero">
        <div>
          <p className="eyebrow">Apps — P6A</p>
          <h2>Review installed software and open standard uninstall flows</h2>
          <p className="muted">
            PC Manager rediscovers the selected application before launching its
            registered uninstall flow. It never adds silent uninstall flags.
          </p>
        </div>
        <button className="secondary-action" onClick={load} type="button">
          Refresh
        </button>
      </section>

      <section className="p6-toolbar">
        <label>
          <span>Search installed applications</span>
          <input
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Name, publisher, version, source…"
            type="search"
            value={query}
          />
        </label>
        <div className="p6-note">
          <strong>No forced silent uninstall.</strong>
          <span>
            Vendor/MSI interfaces remain visible so you can review what is being
            removed.
          </span>
        </div>
      </section>

      {loadState.status === "loading" && (
        <section className="scan-progress">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Reading Windows uninstall registrations…</strong>
            <span>Current-user and machine-wide records are being mapped.</span>
          </div>
        </section>
      )}

      {loadState.status === "error" && (
        <section className="info-callout health-error">
          <strong>Apps inventory unavailable</strong>
          <span>{loadState.message}</span>
          <button className="secondary-action" onClick={load} type="button">
            Retry
          </button>
        </section>
      )}

      {actionState.status === "complete" && (
        <section className="info-callout startup-success" aria-live="polite">
          <strong>Standard uninstall flow opened</strong>
          <span>{actionState.result.message}</span>
          <button className="secondary-action" onClick={load} type="button">
            Refresh apps
          </button>
        </section>
      )}

      {actionState.status === "error" && (
        <section className="info-callout health-error" aria-live="polite">
          <strong>Uninstall flow could not be opened</strong>
          <span>{actionState.message}</span>
        </section>
      )}

      {loadState.status === "ready" && (
        <section>
          <div className="section-heading">
            <div>
              <p className="eyebrow">Installed applications</p>
              <h3>{visibleApps.length} visible apps</h3>
            </div>
            <span className="status-badge neutral">
              {loadState.apps.length} total
            </span>
          </div>

          {visibleApps.length === 0 ? (
            <div className="empty-health-state">
              <strong>No applications match this search.</strong>
              <span>Clear the search field or refresh the inventory.</span>
            </div>
          ) : (
            <div className="p6-list">
              {visibleApps.map((app) => (
                <article className="p6-item-card" key={app.id}>
                  <div className="p6-item-main">
                    <div className="p6-item-heading">
                      <div>
                        <span className="startup-source">
                          {sourceLabel(app.source)}
                        </span>
                        <h4>{app.displayName}</h4>
                      </div>
                      <span
                        className={
                          app.canUninstall
                            ? "status-badge good"
                            : "status-badge neutral"
                        }
                      >
                        {app.canUninstall ? "Uninstall available" : "Read-only"}
                      </span>
                    </div>
                    <div className="p6-meta-grid">
                      <span>Version: {app.version ?? "Unavailable"}</span>
                      <span>Publisher: {app.publisher ?? "Unavailable"}</span>
                      <span>{formatBytes(app.estimatedSizeBytes)}</span>
                      <span>{app.detail}</span>
                    </div>
                    {app.installLocation && (
                      <p className="p6-path">{app.installLocation}</p>
                    )}
                  </div>
                  <div className="p6-item-action">
                    <button
                      className="secondary-action"
                      disabled={
                        !app.canUninstall || actionState.status === "running"
                      }
                      onClick={() =>
                        setActionState({ status: "confirming", app })
                      }
                      type="button"
                    >
                      Uninstall…
                    </button>
                  </div>
                </article>
              ))}
            </div>
          )}
        </section>
      )}

      {actionState.status === "confirming" && (
        <div className="startup-confirmation-backdrop" role="presentation">
          <section
            aria-labelledby="apps-confirmation-title"
            aria-modal="true"
            className="cleanup-confirmation startup-confirmation-dialog"
            role="dialog"
          >
            <p className="eyebrow">Confirmation required</p>
            <h3 id="apps-confirmation-title">
              Open the standard uninstall flow for {actionState.app.displayName}?
            </h3>
            <p>
              PC Manager will launch only the uninstall action registered by this
              application. It will not force a silent removal.
            </p>
            <div className="confirmation-actions">
              <button
                autoFocus
                className="secondary-action"
                onClick={() => setActionState({ status: "idle" })}
                type="button"
              >
                Cancel
              </button>
              <button
                className="primary-action"
                onClick={() => void executeUninstall(actionState.app)}
                type="button"
              >
                Open uninstaller
              </button>
            </div>
          </section>
        </div>
      )}

      {actionState.status === "running" && (
        <section className="scan-progress" aria-live="polite">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Revalidating the application…</strong>
            <span>The registered uninstall metadata is being read again.</span>
          </div>
        </section>
      )}
    </div>
  );
}

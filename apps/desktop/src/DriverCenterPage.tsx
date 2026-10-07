import { useMemo, useState } from "react";

import {
  filterDriverUpdates,
  filterInstalledDrivers,
  openDriverUpdateSettings,
  scanDriverCenter,
  type DriverCenterSnapshot,
  type DriverSettingsLaunchResult,
} from "./driverCenter";

type ScanState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "ready"; snapshot: DriverCenterSnapshot }
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

function displayDate(value: string | null): string {
  if (!value) return "Unavailable";
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(date);
}

export function DriverCenterPage() {
  const [scanState, setScanState] = useState<ScanState>({ status: "idle" });
  const [query, setQuery] = useState("");
  const [launchResult, setLaunchResult] =
    useState<DriverSettingsLaunchResult | null>(null);
  const [launchError, setLaunchError] = useState<string | null>(null);

  const runScan = async () => {
    setScanState({ status: "loading" });
    setLaunchResult(null);
    setLaunchError(null);

    try {
      const snapshot = await scanDriverCenter();
      setScanState({ status: "ready", snapshot });
    } catch (error: unknown) {
      setScanState({
        status: "error",
        message: errorMessage(error, "Driver Center could not scan safely."),
      });
    }
  };

  const openSettings = async () => {
    setLaunchResult(null);
    setLaunchError(null);

    try {
      setLaunchResult(await openDriverUpdateSettings());
    } catch (error: unknown) {
      setLaunchError(
        errorMessage(error, "Windows Optional updates could not be opened."),
      );
    }
  };

  const updates = useMemo(
    () =>
      scanState.status === "ready"
        ? filterDriverUpdates(scanState.snapshot.availableUpdates, query)
        : [],
    [query, scanState],
  );

  const installed = useMemo(
    () =>
      scanState.status === "ready"
        ? filterInstalledDrivers(scanState.snapshot.installedDrivers, query)
        : [],
    [query, scanState],
  );

  return (
    <div className="page-stack p6-page">
      <section className="p6-hero">
        <div>
          <p className="eyebrow">Driver Center — P11</p>
          <h2>Use Windows Update evidence, not driver-age guesses</h2>
          <p className="muted">
            PC Manager recommends a driver only when Windows Update reports it
            as applicable to this PC. V1 never downloads driver packages,
            performs blind mass updates, or treats an old date as a defect.
          </p>
        </div>
        <button
          className="primary-action"
          disabled={scanState.status === "loading"}
          onClick={() => void runScan()}
          type="button"
        >
          {scanState.status === "loading"
            ? "Checking drivers…"
            : "Check Driver Center"}
        </button>
      </section>

      <section className="info-callout">
        <strong>Driver changes remain owned by Windows Update.</strong>
        <span>
          Review each optional driver in Windows Settings before installing it.
          This phase has no direct driver installer and no Update All button.
        </span>
      </section>

      {scanState.status === "idle" && (
        <section className="empty-health-state">
          <strong>No driver scan has run in this session.</strong>
          <span>
            Start a read-only inventory and Windows Update applicability check.
          </span>
        </section>
      )}

      {scanState.status === "loading" && (
        <section className="scan-progress" aria-live="polite">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Reading local drivers and Windows Update evidence…</strong>
            <span>No driver package is downloaded or installed.</span>
          </div>
        </section>
      )}

      {scanState.status === "error" && (
        <section className="info-callout health-error" aria-live="polite">
          <strong>Driver Center scan unavailable</strong>
          <span>{scanState.message}</span>
          <button className="secondary-action" onClick={() => void runScan()}>
            Retry
          </button>
        </section>
      )}

      {launchResult && (
        <section className="info-callout startup-success" aria-live="polite">
          <strong>{launchResult.target} opened</strong>
          <span>{launchResult.message}</span>
        </section>
      )}

      {launchError && (
        <section className="info-callout health-error" aria-live="polite">
          <strong>Windows Settings was not opened</strong>
          <span>{launchError}</span>
        </section>
      )}

      {scanState.status === "ready" && (
        <>
          <section className="native-status-card">
            <div>
              <p className="eyebrow">Trusted source</p>
              <h3>{scanState.snapshot.provider}</h3>
              <p className="muted">{scanState.snapshot.sourceDetail}</p>
            </div>
            <span
              className={
                scanState.snapshot.providerAvailable
                  ? "status-badge good"
                  : "status-badge warning"
              }
            >
              {scanState.snapshot.providerAvailable
                ? "Windows Update available"
                : "Windows Update unavailable"}
            </span>
            <dl>
              <div>
                <dt>Applicable updates</dt>
                <dd>{scanState.snapshot.availableUpdates.length}</dd>
              </div>
              <div>
                <dt>Installed drivers</dt>
                <dd>{scanState.snapshot.installedDrivers.length}</dd>
              </div>
              <div>
                <dt>Warnings</dt>
                <dd>{scanState.snapshot.warnings.length}</dd>
              </div>
              <div>
                <dt>Install mode</dt>
                <dd>Windows Settings only</dd>
              </div>
            </dl>
          </section>

          <section className="p6-toolbar">
            <label>
              <span>Filter drivers</span>
              <input
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Device, provider, class, version…"
                type="search"
                value={query}
              />
            </label>
            <button
              className="secondary-action"
              onClick={() => void openSettings()}
              type="button"
            >
              Open Optional updates
            </button>
          </section>

          {scanState.snapshot.warnings.length > 0 && (
            <details className="p6-warning-box">
              <summary>
                {scanState.snapshot.warnings.length} provider warnings
              </summary>
              <ul>
                {scanState.snapshot.warnings.map((warning) => (
                  <li key={warning.code + ":" + warning.message}>
                    {warning.message}
                  </li>
                ))}
              </ul>
            </details>
          )}

          <section>
            <div className="section-heading">
              <div>
                <p className="eyebrow">Applicable driver updates</p>
                <h3>{updates.length} visible candidates</h3>
              </div>
              <span className="status-badge neutral">No automatic install</span>
            </div>

            {updates.length === 0 ? (
              <div className="empty-health-state">
                <strong>
                  No applicable Windows Update drivers in this view.
                </strong>
                <span>
                  PC Manager will not manufacture a recommendation from driver
                  age alone.
                </span>
              </div>
            ) : (
              <div className="p6-list">
                {updates.map((update) => (
                  <article className="p6-item-card" key={update.id}>
                    <div className="p6-item-main">
                      <div className="p6-item-heading">
                        <div>
                          <span className="startup-source">\n                            {update.source}\n                          </span>
                          <h4>{update.title}</h4>
                        </div>
                        <span className="status-badge good">Applicable</span>
                      </div>
                      <div className="p6-meta-grid">
                        <span>
                          Provider: {update.provider ?? "Unavailable"}
                        </span>
                        <span>
                          Manufacturer: {update.manufacturer ?? "Unavailable"}
                        </span>
                        <span>Model: {update.model ?? "Unavailable"}</span>
                        <span>Class: {update.className ?? "Unavailable"}</span>
                        <span>Date: {displayDate(update.driverDate)}</span>
                        <span>
                          Reboot:{" "}
                          {update.rebootRequired
                            ? "May be required"
                            : "Not reported"}
                        </span>
                      </div>
                      <p className="muted">{update.recommendation}</p>
                    </div>
                    <div className="p6-item-action">
                      <button
                        className="primary-action"
                        onClick={() => void openSettings()}
                        type="button"
                      >
                        Review in Windows Update
                      </button>
                    </div>
                  </article>
                ))}
              </div>
            )}
          </section>

          <details className="p6-warning-box">
            <summary>
              Installed driver inventory ({installed.length} visible)
            </summary>
            <div className="p6-list">
              {installed.map((driver) => (
                <article className="p6-item-card" key={driver.id}>
                  <div className="p6-item-main">
                    <div className="p6-item-heading">
                      <div>
                        <span className="startup-source">
                          {driver.className ?? "Driver"}
                        </span>
                        <h4>{driver.deviceName}</h4>
                      </div>
                      <span
                        className={
                          driver.isSigned === false
                            ? "status-badge warning"
                            : "status-badge neutral"
                        }
                      >
                        {driver.isSigned === false ? "Unsigned" : "Inventory"}
                      </span>
                    </div>
                    <div className="p6-meta-grid">
                      <span>Version: {driver.version ?? "Unavailable"}</span>
                      <span>Provider: {driver.provider ?? "Unavailable"}</span>
                      <span>
                        Manufacturer: {driver.manufacturer ?? "Unavailable"}
                      </span>
                      <span>Date: {displayDate(driver.driverDate)}</span>
                      <span>INF: {driver.infName ?? "Unavailable"}</span>
                      <span>Signer: {driver.signer ?? "Unavailable"}</span>
                    </div>
                  </div>
                </article>
              ))}
            </div>
          </details>
        </>
      )}
    </div>
  );
}

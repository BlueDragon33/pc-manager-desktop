import { useMemo, useState } from "react";

import {
  checkSoftwareUpdates,
  filterUpdateCandidates,
  launchSoftwareUpdate,
  type SoftwareUpdateCandidate,
  type SoftwareUpdateLaunchResult,
  type SoftwareUpdateScan,
} from "./softwareUpdater";

type ScanState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "ready"; scan: SoftwareUpdateScan }
  | { status: "error"; message: string };

type ActionState =
  | { status: "idle" }
  | { status: "confirming"; candidate: SoftwareUpdateCandidate }
  | { status: "running"; candidate: SoftwareUpdateCandidate }
  | { status: "complete"; result: SoftwareUpdateLaunchResult }
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

export function SoftwareUpdaterPage() {
  const [scanState, setScanState] = useState<ScanState>({ status: "idle" });
  const [actionState, setActionState] = useState<ActionState>({
    status: "idle",
  });
  const [query, setQuery] = useState("");

  const runScan = async () => {
    setScanState({ status: "loading" });
    setActionState({ status: "idle" });
    try {
      const scan = await checkSoftwareUpdates();
      setScanState({ status: "ready", scan });
    } catch (error: unknown) {
      setScanState({
        status: "error",
        message: errorMessage(
          error,
          "Software updates could not be checked safely.",
        ),
      });
    }
  };

  const visibleCandidates = useMemo(
    () =>
      scanState.status === "ready"
        ? filterUpdateCandidates(scanState.scan.candidates, query)
        : [],
    [query, scanState],
  );

  const launch = async (candidate: SoftwareUpdateCandidate) => {
    if (scanState.status !== "ready") return;

    setActionState({ status: "running", candidate });
    try {
      const result = await launchSoftwareUpdate(
        scanState.scan.scanId,
        candidate.id,
      );
      setActionState({ status: "complete", result });
    } catch (error: unknown) {
      setActionState({
        status: "error",
        message: errorMessage(
          error,
          "The selected update could not be launched safely.",
        ),
      });
    }
  };

  return (
    <div className="page-stack p6-page">
      <section className="p6-hero">
        <div>
          <p className="eyebrow">Software Updater — P10</p>
          <h2>Check trusted WinGet updates without arbitrary downloads</h2>
          <p className="muted">
            PC Manager uses only the verified official WinGet source in V1. It
            does not accept download URLs, custom installer arguments, force
            flags, silent flags, or hash-bypass flags from the UI or App
            Manager.
          </p>
        </div>
        <button
          className="primary-action"
          disabled={
            scanState.status === "loading" || actionState.status === "running"
          }
          onClick={() => void runScan()}
          type="button"
        >
          {scanState.status === "loading" ? "Checking…" : "Check for updates"}
        </button>
      </section>

      {scanState.status === "idle" && (
        <section className="info-callout">
          <strong>No update scan has run in this session.</strong>
          <span>
            Start a read-only scan. PC Manager will first verify the configured
            WinGet source identity and trust metadata.
          </span>
        </section>
      )}

      {scanState.status === "loading" && (
        <section className="scan-progress" aria-live="polite">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Verifying WinGet and checking available updates…</strong>
            <span>
              No installer is launched during this scan and no arbitrary URL is
              downloaded.
            </span>
          </div>
        </section>
      )}

      {scanState.status === "error" && (
        <section className="info-callout health-error" aria-live="polite">
          <strong>Trusted update scan unavailable</strong>
          <span>{scanState.message}</span>
          <button className="secondary-action" onClick={() => void runScan()}>
            Retry
          </button>
        </section>
      )}

      {actionState.status === "complete" && (
        <section className="info-callout startup-success" aria-live="polite">
          <strong>Verified WinGet update flow launched</strong>
          <span>{actionState.result.message}</span>
          <button className="secondary-action" onClick={() => void runScan()}>
            Scan again
          </button>
        </section>
      )}

      {actionState.status === "error" && (
        <section className="info-callout health-error" aria-live="polite">
          <strong>Update was not launched</strong>
          <span>{actionState.message}</span>
          <button className="secondary-action" onClick={() => void runScan()}>
            Re-scan
          </button>
        </section>
      )}

      {scanState.status === "ready" && (
        <>
          <section className="native-status-card">
            <div>
              <p className="eyebrow">Provider evidence</p>
              <h3>{scanState.scan.provider}</h3>
              <p className="muted">{scanState.scan.sourceDetail}</p>
            </div>
            <span
              className={
                scanState.scan.sourceVerified
                  ? "status-badge good"
                  : "status-badge warning"
              }
            >
              {scanState.scan.sourceVerified
                ? "Official source verified"
                : "Source not verified"}
            </span>
            <dl>
              <div>
                <dt>Provider</dt>
                <dd>
                  {scanState.scan.providerAvailable
                    ? "Available"
                    : "Unavailable"}
                </dd>
              </div>
              <div>
                <dt>Updates</dt>
                <dd>{scanState.scan.candidates.length}</dd>
              </div>
              <div>
                <dt>Warnings</dt>
                <dd>{scanState.scan.warnings.length}</dd>
              </div>
              <div>
                <dt>Mode</dt>
                <dd>One app at a time</dd>
              </div>
            </dl>
          </section>

          <section className="p6-toolbar">
            <label>
              <span>Filter available updates</span>
              <input
                onChange={(event) => setQuery(event.target.value)}
                placeholder="App name, package ID, version…"
                type="search"
                value={query}
              />
            </label>
            <div className="p6-note">
              <strong>
                Publisher verification is intentionally conservative.
              </strong>
              <span>
                V1 verifies WinGet source/package identity. Publisher metadata
                is shown only when independently available; otherwise it remains
                explicitly unavailable.
              </span>
            </div>
          </section>

          {scanState.scan.warnings.length > 0 && (
            <details className="p6-warning-box">
              <summary>
                {scanState.scan.warnings.length} provider warnings
              </summary>
              <ul>
                {scanState.scan.warnings.map((warning) => (
                  <li key={warning}>{warning}</li>
                ))}
              </ul>
            </details>
          )}

          <section>
            <div className="section-heading">
              <div>
                <p className="eyebrow">Available updates</p>
                <h3>{visibleCandidates.length} visible candidates</h3>
              </div>
              <span className="status-badge neutral">
                No automatic Update All
              </span>
            </div>

            {visibleCandidates.length === 0 ? (
              <div className="empty-health-state">
                <strong>No trusted updates match this view.</strong>
                <span>
                  The machine may already be current, or the filter may exclude
                  available candidates.
                </span>
              </div>
            ) : (
              <div className="p6-list">
                {visibleCandidates.map((candidate) => (
                  <article className="p6-item-card" key={candidate.id}>
                    <div className="p6-item-main">
                      <div className="p6-item-heading">
                        <div>
                          <span className="startup-source">
                            {candidate.source}
                          </span>
                          <h4>{candidate.name}</h4>
                        </div>
                        <span className="status-badge good">
                          Trusted source
                        </span>
                      </div>
                      <div className="p6-meta-grid">
                        <span>Installed: {candidate.installedVersion}</span>
                        <span>Available: {candidate.availableVersion}</span>
                        <span>Package ID: {candidate.packageId}</span>
                        <span>
                          Publisher: {candidate.publisher ?? "Unavailable"}
                        </span>
                      </div>
                      <p className="p6-path">
                        {candidate.publisherVerification}
                      </p>
                      <p className="muted">{candidate.detail}</p>
                    </div>
                    <div className="p6-item-action">
                      <button
                        className="primary-action"
                        disabled={actionState.status === "running"}
                        onClick={() =>
                          setActionState({ status: "confirming", candidate })
                        }
                        type="button"
                      >
                        Update…
                      </button>
                    </div>
                  </article>
                ))}
              </div>
            )}
          </section>
        </>
      )}

      {actionState.status === "confirming" && scanState.status === "ready" && (
        <div className="startup-confirmation-backdrop" role="presentation">
          <section
            aria-labelledby="software-update-confirmation-title"
            aria-modal="true"
            className="cleanup-confirmation startup-confirmation-dialog"
            role="dialog"
          >
            <p className="eyebrow">Confirmation required</p>
            <h3 id="software-update-confirmation-title">
              Update {actionState.candidate.name}?
            </h3>
            <p>
              PC Manager will first re-scan the trusted WinGet source and
              require the same exact package ID and version transition. If
              anything changed, the launch is refused.
            </p>
            <div className="p6-meta-grid">
              <span>{actionState.candidate.installedVersion}</span>
              <span>→</span>
              <span>{actionState.candidate.availableVersion}</span>
              <span>{actionState.candidate.packageId}</span>
            </div>
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
                onClick={() => void launch(actionState.candidate)}
                type="button"
              >
                Open trusted updater
              </button>
            </div>
          </section>
        </div>
      )}

      {actionState.status === "running" && (
        <section className="scan-progress" aria-live="polite">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Revalidating this exact update…</strong>
            <span>
              PC Manager will refuse a stale candidate instead of launching a
              changed package.
            </span>
          </div>
        </section>
      )}
    </div>
  );
}

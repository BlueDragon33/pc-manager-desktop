import { useRef, useState } from "react";

import {
  formatCleanupCategory,
  isCurrentScan,
  scanCleanupCandidates,
  type CleanupProviderSummary,
  type CleanupScanSummary,
} from "./smartClean";
import { formatBytes } from "./systemInventory";

type ScanState =
  | { status: "idle" }
  | { status: "scanning" }
  | { status: "cancelled" }
  | { status: "error"; message: string }
  | { status: "complete"; summary: CleanupScanSummary };

function providerStatus(provider: CleanupProviderSummary): string {
  if (!provider.available) {
    return "Unavailable";
  }
  if (provider.fileCount === 0) {
    return "Nothing found";
  }
  return "Candidates found";
}

export function SmartCleanPage() {
  const [state, setState] = useState<ScanState>({ status: "idle" });
  const [includeRecycleBin, setIncludeRecycleBin] = useState(false);
  const generationRef = useRef(0);

  const startScan = () => {
    const generation = generationRef.current + 1;
    generationRef.current = generation;
    setState({ status: "scanning" });

    scanCleanupCandidates({ includeRecycleBin })
      .then((summary) => {
        if (!isCurrentScan(generation, generationRef.current)) {
          return;
        }
        setState({ status: "complete", summary });
      })
      .catch((error: unknown) => {
        if (!isCurrentScan(generation, generationRef.current)) {
          return;
        }

        const message =
          typeof error === "object" &&
          error !== null &&
          "message" in error &&
          typeof (error as { message?: unknown }).message === "string"
            ? (error as { message: string }).message
            : error instanceof Error
              ? error.message
              : "Smart Clean preview could not complete.";

        setState({ status: "error", message });
      });
  };

  const cancelScan = () => {
    if (state.status !== "scanning") {
      return;
    }

    generationRef.current += 1;
    setState({ status: "cancelled" });
  };

  const summary = state.status === "complete" ? state.summary : null;

  return (
    <div className="page-stack cleaner-page">
      <section className="cleaner-hero">
        <div>
          <p className="eyebrow">Smart Clean — P4A</p>
          <h2>Real cleanup preview, deletion still disabled</h2>
          <p className="muted">
            PC Manager scans only built-in temp and cache locations. The frontend
            cannot submit arbitrary folders, and P4A contains no delete command.
          </p>
        </div>
        <div className="cleaner-actions">
          {state.status === "scanning" ? (
            <button className="secondary-action" onClick={cancelScan}>
              Cancel scan
            </button>
          ) : (
            <button className="primary-action" onClick={startScan}>
              {summary ? "Scan again" : "Scan cleanup candidates"}
            </button>
          )}
        </div>
      </section>

      <section className="cleaner-safety-card">
        <div>
          <span className="status-badge good">Preview only</span>
          <strong>No files can be deleted in this phase.</strong>
          <p>
            The native engine creates a cleanup plan and stores it inside the
            desktop process. The UI receives only aggregate totals and a plan ID.
          </p>
        </div>
        <label className="cleaner-option">
          <input
            checked={includeRecycleBin}
            disabled={state.status === "scanning"}
            onChange={(event) => setIncludeRecycleBin(event.target.checked)}
            type="checkbox"
          />
          <span>
            <strong>Include Recycle Bin check</strong>
            <small>
              Opt-in only. P4A currently reports this provider as unavailable
              until its native size/count implementation is verified.
            </small>
          </span>
        </label>
      </section>

      {state.status === "scanning" && (
        <section className="scan-progress" aria-live="polite">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Scanning explicit temp and cache roots…</strong>
            <span>
              Junctions, symlinks, and Windows reparse points are skipped. The
              scanner never enters Documents, Desktop, Downloads, or arbitrary
              user-selected folders.
            </span>
          </div>
        </section>
      )}

      {state.status === "cancelled" && (
        <section className="info-callout" aria-live="polite">
          <strong>Preview scan cancelled</strong>
          <span>
            Any late native result is discarded. Nothing was modified or deleted.
          </span>
        </section>
      )}

      {state.status === "error" && (
        <section className="info-callout health-error" aria-live="polite">
          <strong>Smart Clean preview could not complete</strong>
          <span>{state.message}</span>
          <button className="secondary-action" onClick={startScan}>
            Retry
          </button>
        </section>
      )}

      {summary ? (
        <>
          <section className="cleaner-total-card">
            <div>
              <p className="eyebrow">Preview total</p>
              <strong className="cleaner-total-bytes">
                {formatBytes(summary.totalBytes)}
              </strong>
              <span>{summary.totalFiles.toLocaleString()} candidate files</span>
            </div>
            <dl>
              <div>
                <dt>Plan ID</dt>
                <dd>{summary.planId}</dd>
              </div>
              <div>
                <dt>Collected</dt>
                <dd>
                  {new Date(summary.collectedAtEpochMs).toLocaleString()}
                </dd>
              </div>
              <div>
                <dt>Execution</dt>
                <dd>
                  {summary.executionAvailable ? "Available" : "Disabled in P4A"}
                </dd>
              </div>
            </dl>
          </section>

          <section aria-labelledby="cleaner-provider-heading">
            <div className="section-heading">
              <div>
                <p className="eyebrow">Providers</p>
                <h3 id="cleaner-provider-heading">
                  Exactly where the preview came from
                </h3>
              </div>
            </div>
            <div className="cleaner-provider-grid">
              {summary.providers.map((provider) => (
                <article
                  className={
                    provider.available
                      ? "cleaner-provider-card"
                      : "cleaner-provider-card provider-unavailable"
                  }
                  key={provider.providerId}
                >
                  <div className="cleaner-provider-heading">
                    <div>
                      <span>{formatCleanupCategory(provider.category)}</span>
                      <h4>{provider.displayName}</h4>
                    </div>
                    <span className="status-badge neutral">
                      {providerStatus(provider)}
                    </span>
                  </div>
                  <div className="cleaner-provider-metrics">
                    <strong>{formatBytes(provider.bytes)}</strong>
                    <span>{provider.fileCount.toLocaleString()} files</span>
                  </div>
                  <p>{provider.description}</p>
                  <small>
                    {provider.reversible
                      ? "Rollback supported"
                      : "Deletion would not be automatically restorable"}
                  </small>
                  {provider.warnings.length > 0 && (
                    <ul>
                      {provider.warnings.map((warning) => (
                        <li key={warning}>{warning}</li>
                      ))}
                    </ul>
                  )}
                </article>
              ))}
            </div>
          </section>

          {summary.warnings.length > 0 && (
            <section className="inventory-warning-list">
              <p className="eyebrow">Scan warnings</p>
              <h3>Some provider items could not be inspected</h3>
              <ul>
                {summary.warnings.map((warning, index) => (
                  <li key={`${warning.providerId}-${index}`}>
                    <strong>{warning.providerId}</strong>
                    <span>{warning.message}</span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          <section className="cleaner-locked-action">
            <div>
              <strong>Clean action intentionally locked</strong>
              <span>
                First verify these preview totals on a real Windows machine. P4B
                will then add plan revalidation, explicit confirmation, execution
                results, and operation history.
              </span>
            </div>
            <button disabled type="button">
              Clean selected items
            </button>
          </section>
        </>
      ) : (
        state.status !== "scanning" && (
          <section className="empty-health-state">
            <strong>No Smart Clean preview yet.</strong>
            <span>
              Start a scan to measure real stale temp files and explicit browser
              or application caches. This phase cannot delete anything.
            </span>
          </section>
        )
      )}
    </div>
  );
}

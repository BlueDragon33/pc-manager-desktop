import { useRef, useState } from "react";

import {
  cleanupResultSummary,
  executeCleanupPlan,
  formatCleanupCategory,
  isCurrentScan,
  scanCleanupCandidates,
  type CleanupOperationRecord,
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

type ExecutionState =
  | { status: "idle" }
  | { status: "confirming" }
  | { status: "executing" }
  | { status: "error"; message: string }
  | { status: "complete"; operation: CleanupOperationRecord };

function providerStatus(provider: CleanupProviderSummary): string {
  if (!provider.available) {
    return "Unavailable";
  }
  if (provider.fileCount === 0) {
    return "Nothing found";
  }
  return "Candidates found";
}

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

export function SmartCleanPage({
  onOpenRestore,
}: {
  onOpenRestore?: () => void;
}) {
  const [state, setState] = useState<ScanState>({ status: "idle" });
  const [execution, setExecution] = useState<ExecutionState>({
    status: "idle",
  });
  const [includeRecycleBin, setIncludeRecycleBin] = useState(false);
  const generationRef = useRef(0);

  const startScan = () => {
    const generation = generationRef.current + 1;
    generationRef.current = generation;
    setExecution({ status: "idle" });
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

        setState({
          status: "error",
          message: errorMessage(
            error,
            "Smart Clean preview could not complete.",
          ),
        });
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

  const confirmExecution = () => {
    if (!summary || !summary.executionAvailable || summary.totalFiles === 0) {
      return;
    }
    setExecution({ status: "confirming" });
  };

  const executePlan = () => {
    if (!summary || execution.status !== "confirming") {
      return;
    }

    setExecution({ status: "executing" });
    executeCleanupPlan(summary.planId)
      .then((operation) => {
        setExecution({ status: "complete", operation });
      })
      .catch((error: unknown) => {
        setExecution({
          status: "error",
          message: errorMessage(
            error,
            "The cleanup plan could not be executed safely.",
          ),
        });
      });
  };

  return (
    <div className="page-stack cleaner-page">
      <section className="cleaner-hero">
        <div>
          <p className="eyebrow">Smart Clean — P4B</p>
          <h2>Preview first, then execute the exact native plan</h2>
          <p className="muted">
            PC Manager scans only built-in temp and cache locations. Before
            deletion, every planned file is re-resolved and checked again
            against its original allow-listed provider root.
          </p>
        </div>
        <div className="cleaner-actions">
          {state.status === "scanning" ? (
            <button className="secondary-action" onClick={cancelScan}>
              Cancel scan
            </button>
          ) : (
            <button
              className="primary-action"
              disabled={execution.status === "executing"}
              onClick={startScan}
            >
              {summary ? "Scan again" : "Scan cleanup candidates"}
            </button>
          )}
        </div>
      </section>

      <section className="cleaner-safety-card">
        <div>
          <span className="status-badge good">Plan-only execution</span>
          <strong>The UI cannot submit arbitrary paths for deletion.</strong>
          <p>
            Execution accepts only the native plan ID. Changed files, paths
            outside their provider root, reparse points, and stale plans are
            rejected item by item and recorded in the operation audit.
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
              Still opt-in and unavailable in P4B until its native provider has
              been separately verified. Smart Clean will not empty it.
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
            Any late native result is discarded. Nothing was modified or
            deleted.
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
                <dd>{new Date(summary.collectedAtEpochMs).toLocaleString()}</dd>
              </div>
              <div>
                <dt>Execution</dt>
                <dd>
                  {summary.executionAvailable
                    ? "Available after confirmation"
                    : "Nothing to clean"}
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
                      : "Deletion is not automatically restorable"}
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

          {execution.status === "complete" ? (
            <section className="cleanup-result-card" aria-live="polite">
              <div>
                <p className="eyebrow">Cleanup completed</p>
                <h3>{cleanupResultSummary(execution.operation)}</h3>
                <p className="muted">
                  {formatBytes(execution.operation.deletedBytes)} was removed.
                  This cache/temp cleanup is marked Not restorable in Restore
                  Center.
                </p>
              </div>
              <dl>
                <div>
                  <dt>Deleted</dt>
                  <dd>
                    {execution.operation.deletedFiles.toLocaleString()} files
                  </dd>
                </div>
                <div>
                  <dt>Failed / skipped</dt>
                  <dd>
                    {execution.operation.failedFiles.toLocaleString()} files
                  </dd>
                </div>
                <div>
                  <dt>Operation ID</dt>
                  <dd>{execution.operation.operationId}</dd>
                </div>
              </dl>
              {onOpenRestore && (
                <button className="secondary-action" onClick={onOpenRestore}>
                  Open Restore Center
                </button>
              )}
            </section>
          ) : (
            <section className="cleaner-locked-action cleaner-execution-action">
              <div>
                <strong>Ready to execute this exact preview plan</strong>
                <span>
                  Cache and temporary-file deletion is usually not reversible.
                  Files changed since the scan will be skipped rather than
                  deleted.
                </span>
              </div>
              <button
                className="danger-action"
                disabled={
                  !summary.executionAvailable ||
                  execution.status === "executing"
                }
                onClick={confirmExecution}
                type="button"
              >
                {execution.status === "executing"
                  ? "Cleaning…"
                  : "Clean previewed files"}
              </button>
            </section>
          )}

          {execution.status === "error" && (
            <section className="info-callout health-error" aria-live="polite">
              <strong>Cleanup did not complete normally</strong>
              <span>{execution.message}</span>
              <span>
                The native plan is single-use. Run a fresh preview before trying
                again so no stale paths can be replayed.
              </span>
            </section>
          )}
        </>
      ) : (
        state.status !== "scanning" && (
          <section className="empty-health-state">
            <strong>No Smart Clean preview yet.</strong>
            <span>
              Start a scan to measure real stale temp files and explicit browser
              or application caches before deciding whether to delete anything.
            </span>
          </section>
        )
      )}

      {execution.status === "confirming" && summary && (
        <div className="confirmation-backdrop" role="presentation">
          <section
            aria-labelledby="cleanup-confirm-title"
            aria-modal="true"
            className="confirmation-dialog"
            role="dialog"
          >
            <p className="eyebrow">Irreversible cleanup</p>
            <h3 id="cleanup-confirm-title">
              Delete the previewed temp/cache files?
            </h3>
            <p>
              PC Manager will revalidate all{" "}
              {summary.totalFiles.toLocaleString()} planned files immediately
              before deletion. Up to {formatBytes(summary.totalBytes)} may be
              removed. Ordinary cache deletion cannot be restored by PC Manager.
            </p>
            <div className="confirmation-actions">
              <button
                className="secondary-action"
                onClick={() => setExecution({ status: "idle" })}
                type="button"
              >
                Cancel
              </button>
              <button
                className="danger-action"
                onClick={executePlan}
                type="button"
              >
                Delete previewed temp/cache files
              </button>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}

import { useEffect, useState } from "react";

import {
  listCleanupOperations,
  type CleanupOperationRecord,
} from "./smartClean";
import { formatBytes } from "./systemInventory";

type HistoryState =
  | { status: "loading" }
  | { status: "ready"; operations: CleanupOperationRecord[] }
  | { status: "error"; message: string };

function loadErrorMessage(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof (error as { message?: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }

  return error instanceof Error
    ? error.message
    : "Cleanup operation history could not be loaded.";
}

export function RestoreCenterPage() {
  const [state, setState] = useState<HistoryState>({ status: "loading" });

  const refresh = () => {
    setState({ status: "loading" });
    listCleanupOperations()
      .then((operations) => {
        setState({ status: "ready", operations });
      })
      .catch((error: unknown) => {
        setState({ status: "error", message: loadErrorMessage(error) });
      });
  };

  useEffect(() => {
    refresh();
  }, []);

  return (
    <div className="page-stack restore-page">
      <section className="restore-hero">
        <div>
          <p className="eyebrow">Restore Center</p>
          <h2>Operation history with honest rollback capability</h2>
          <p className="muted">
            P4B records every Smart Clean execution locally. Cache and temporary
            file deletion is normally not restorable, so those operations are
            shown as Not restorable instead of offering a fake undo button.
          </p>
        </div>
        <button className="secondary-action" onClick={refresh} type="button">
          Refresh history
        </button>
      </section>

      {state.status === "loading" && (
        <section className="scan-progress" aria-live="polite">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Loading local operation history…</strong>
            <span>No cloud connection is required.</span>
          </div>
        </section>
      )}

      {state.status === "error" && (
        <section className="info-callout health-error" aria-live="polite">
          <strong>Restore Center is unavailable</strong>
          <span>{state.message}</span>
          <button className="secondary-action" onClick={refresh}>
            Retry
          </button>
        </section>
      )}

      {state.status === "ready" && state.operations.length === 0 && (
        <section className="empty-health-state">
          <strong>No maintenance operations have been recorded yet.</strong>
          <span>
            Smart Clean executions will appear here after they finish and the
            native audit record is committed.
          </span>
        </section>
      )}

      {state.status === "ready" && state.operations.length > 0 && (
        <section aria-labelledby="restore-history-heading">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Local audit history</p>
              <h3 id="restore-history-heading">
                {state.operations.length} recorded operation(s)
              </h3>
            </div>
          </div>

          <div className="restore-operation-list">
            {state.operations.map((operation) => (
              <article
                className="restore-operation-card"
                key={operation.operationId}
              >
                <div className="restore-operation-heading">
                  <div>
                    <span className="status-badge neutral">Smart Clean</span>
                    <h4>
                      {new Date(operation.completedAtEpochMs).toLocaleString()}
                    </h4>
                  </div>
                  <span className="status-badge warning">Not restorable</span>
                </div>

                <div className="restore-operation-metrics">
                  <div>
                    <span>Deleted</span>
                    <strong>{formatBytes(operation.deletedBytes)}</strong>
                    <small>
                      {operation.deletedFiles.toLocaleString()} file(s)
                    </small>
                  </div>
                  <div>
                    <span>Failed / skipped</span>
                    <strong>{operation.failedFiles.toLocaleString()}</strong>
                    <small>revalidation or deletion failures</small>
                  </div>
                  <div>
                    <span>Requested</span>
                    <strong>{formatBytes(operation.requestedBytes)}</strong>
                    <small>
                      {operation.requestedFiles.toLocaleString()} file(s)
                    </small>
                  </div>
                </div>

                <dl className="restore-operation-ids">
                  <div>
                    <dt>Operation ID</dt>
                    <dd>{operation.operationId}</dd>
                  </div>
                  <div>
                    <dt>Plan ID</dt>
                    <dd>{operation.planId}</dd>
                  </div>
                </dl>

                {operation.issues.length > 0 && (
                  <div className="restore-issues">
                    <strong>Execution notes</strong>
                    <ul>
                      {operation.issues.map((issue) => (
                        <li key={issue.code + ":" + issue.message}>
                          <span>{issue.message}</span>
                          <strong>× {issue.count.toLocaleString()}</strong>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </article>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

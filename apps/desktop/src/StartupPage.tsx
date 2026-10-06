import { useEffect, useMemo, useState } from "react";

import {
  filterStartupEntries,
  listStartupEntries,
  setStartupEntryEnabled,
  sourceTypeLabel,
  type StartupEntry,
  type StartupOperationRecord,
} from "./startup";

type LoadState =
  | { status: "loading" }
  | { status: "ready"; entries: StartupEntry[] }
  | { status: "error"; message: string };

type ChangeState =
  | { status: "idle" }
  | { status: "confirming"; entry: StartupEntry; nextEnabled: boolean }
  | { status: "running"; entry: StartupEntry; nextEnabled: boolean }
  | { status: "complete"; record: StartupOperationRecord }
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

export function StartupPage() {
  const [loadState, setLoadState] = useState<LoadState>({ status: "loading" });
  const [changeState, setChangeState] = useState<ChangeState>({ status: "idle" });
  const [query, setQuery] = useState("");

  const load = () => {
    setLoadState({ status: "loading" });
    listStartupEntries()
      .then((entries) => {
        setLoadState({ status: "ready", entries });
      })
      .catch((error: unknown) => {
        setLoadState({
          status: "error",
          message: errorMessage(error, "Startup inventory could not be loaded."),
        });
      });
  };

  useEffect(() => {
    load();
  }, []);

  const visibleEntries = useMemo(() => {
    if (loadState.status !== "ready") {
      return [];
    }
    return filterStartupEntries(loadState.entries, query);
  }, [loadState, query]);

  const executeChange = async (entry: StartupEntry, nextEnabled: boolean) => {
    setChangeState({ status: "running", entry, nextEnabled });
    try {
      const record = await setStartupEntryEnabled(entry.id, nextEnabled);
      setChangeState({ status: "complete", record });
      load();
    } catch (error: unknown) {
      setChangeState({
        status: "error",
        message: errorMessage(error, "The startup state could not be changed."),
      });
    }
  };

  return (
    <div className="page-stack startup-page">
      <section className="startup-hero">
        <div>
          <p className="eyebrow">Startup Manager — P5</p>
          <h2>Control supported Windows startup entries without uninstalling apps</h2>
          <p className="muted">
            PC Manager rediscovers every entry before changing it. System-level
            entries that require unsupported elevation remain read-only.
          </p>
        </div>
        <button className="secondary-action" onClick={load}>
          Refresh
        </button>
      </section>

      <section className="startup-toolbar">
        <label>
          <span>Search startup entries</span>
          <input
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Name, source, command, publisher…"
            type="search"
            value={query}
          />
        </label>
        <div className="startup-note">
          <strong>Impact stays Unknown unless measured.</strong>
          <span>PC Manager does not invent low/medium/high impact scores.</span>
        </div>
      </section>

      {loadState.status === "loading" && (
        <section className="scan-progress">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Reading supported Windows startup sources…</strong>
            <span>Registry Run keys, Startup folders, and startup/logon tasks.</span>
          </div>
        </section>
      )}

      {loadState.status === "error" && (
        <section className="info-callout health-error">
          <strong>Startup inventory unavailable</strong>
          <span>{loadState.message}</span>
          <button className="secondary-action" onClick={load}>
            Retry
          </button>
        </section>
      )}

      {changeState.status === "complete" && (
        <section className="info-callout startup-success" aria-live="polite">
          <strong>Startup state changed</strong>
          <span>{changeState.record.message}</span>
          <span>
            Rollback: {changeState.record.rollbackAvailable ? "available" : "not available"}
          </span>
        </section>
      )}

      {changeState.status === "error" && (
        <section className="info-callout health-error" aria-live="polite">
          <strong>Startup change failed</strong>
          <span>{changeState.message}</span>
        </section>
      )}

      {loadState.status === "ready" && (
        <section aria-labelledby="startup-list-heading">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Startup inventory</p>
              <h3 id="startup-list-heading">
                {visibleEntries.length} visible entr{visibleEntries.length === 1 ? "y" : "ies"}
              </h3>
            </div>
            <span className="status-badge neutral">
              {loadState.entries.length} total
            </span>
          </div>

          {visibleEntries.length === 0 ? (
            <div className="empty-health-state">
              <strong>No startup entries match this filter.</strong>
              <span>Clear the search box or refresh the inventory.</span>
            </div>
          ) : (
            <div className="startup-list">
              {visibleEntries.map((entry) => (
                <article className="startup-card" key={entry.id}>
                  <div className="startup-card-main">
                    <div className="startup-card-heading">
                      <div>
                        <span className="startup-source">
                          {sourceTypeLabel(entry.sourceType)}
                        </span>
                        <h4>{entry.name}</h4>
                      </div>
                      <span
                        className={
                          entry.enabled ? "status-badge good" : "status-badge neutral"
                        }
                      >
                        {entry.enabled ? "Enabled" : "Disabled"}
                      </span>
                    </div>
                    <p className="startup-command">{entry.command || "Command unavailable"}</p>
                    <div className="startup-meta">
                      <span>Impact: {entry.impact}</span>
                      <span>{entry.detail}</span>
                      {entry.publisher && <span>{entry.publisher}</span>}
                    </div>
                  </div>

                  <div className="startup-card-action">
                    {entry.canChange ? (
                      <button
                        className={entry.enabled ? "secondary-action" : "primary-action"}
                        disabled={changeState.status === "running"}
                        onClick={() =>
                          setChangeState({
                            status: "confirming",
                            entry,
                            nextEnabled: !entry.enabled,
                          })
                        }
                        type="button"
                      >
                        {entry.enabled ? "Disable" : "Enable"}
                      </button>
                    ) : (
                      <div className="startup-readonly">
                        <span className="status-badge neutral">Read-only</span>
                        <small>
                          {entry.requiresElevation
                            ? "Requires elevation; privileged service not used in P5."
                            : "This source is not safely mutable in P5."}
                        </small>
                      </div>
                    )}
                  </div>
                </article>
              ))}
            </div>
          )}
        </section>
      )}

      {changeState.status === "confirming" && (
        <section className="cleanup-confirmation" role="dialog" aria-modal="true">
          <p className="eyebrow">Confirmation required</p>
          <h3>
            {changeState.nextEnabled ? "Enable" : "Disable"}{" "}
            {changeState.entry.name} at Windows startup?
          </h3>
          <p>
            This changes startup behavior only. It does not uninstall the
            application or delete its program files.
          </p>
          <div className="confirmation-actions">
            <button
              className="secondary-action"
              onClick={() => setChangeState({ status: "idle" })}
              type="button"
            >
              Cancel
            </button>
            <button
              className="primary-action"
              onClick={() =>
                void executeChange(
                  changeState.entry,
                  changeState.nextEnabled,
                )
              }
              type="button"
            >
              Confirm change
            </button>
          </div>
        </section>
      )}

      {changeState.status === "running" && (
        <section className="scan-progress" aria-live="polite">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Revalidating startup entry…</strong>
            <span>PC Manager changes only the native entry ID you confirmed.</span>
          </div>
        </section>
      )}
    </div>
  );
}

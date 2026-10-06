import { useEffect, useState } from "react";

import {
  appendMonitorHistory,
  formatBytes,
  formatPercent,
  formatRate,
  getMonitorSnapshot,
  MONITOR_INTERVALS_MS,
  normalizeMonitorInterval,
  shouldScheduleMonitor,
  type MonitorHistoryPoint,
  type MonitorSnapshot,
} from "./monitor";

type MonitorState =
  | { status: "loading" }
  | { status: "ready"; snapshot: MonitorSnapshot }
  | { status: "error"; message: string };

function errorMessage(error: unknown): string {
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
    : "System Monitor sampling failed.";
}

function Availability({
  available,
  reason,
}: {
  available: boolean;
  reason?: string | null;
}) {
  return available ? (
    <span className="status-badge good">Available</span>
  ) : (
    <span className="monitor-unavailable" title={reason ?? undefined}>
      Unavailable
    </span>
  );
}

function History({
  history,
  field,
  label,
}: {
  history: MonitorHistoryPoint[];
  field: "cpu" | "memory";
  label: string;
}) {
  return (
    <div className="monitor-history" aria-label={label}>
      {history.length === 0 ? (
        <span className="muted">Waiting for samples…</span>
      ) : (
        history.map((point) => {
          const value = point[field];
          return (
            <span
              aria-label={
                value === null
                  ? "Unavailable"
                  : `${new Date(point.at).toLocaleTimeString()}: ${value.toFixed(1)}%`
              }
              className={value === null ? "history-bar unavailable" : "history-bar"}
              key={point.at}
              style={{ height: `${Math.max(4, Math.min(100, value ?? 4))}%` }}
              title={value === null ? "Unavailable" : `${value.toFixed(1)}%`}
            />
          );
        })
      )}
    </div>
  );
}

export function MonitorPage() {
  const [state, setState] = useState<MonitorState>({ status: "loading" });
  const [paused, setPaused] = useState(false);
  const [intervalMs, setIntervalMs] = useState(5000);
  const [history, setHistory] = useState<MonitorHistoryPoint[]>([]);

  useEffect(() => {
    if (!shouldScheduleMonitor(paused)) return;

    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const sample = async () => {
      try {
        const snapshot = await getMonitorSnapshot();
        if (cancelled) return;
        setState({ status: "ready", snapshot });
        setHistory((current) => appendMonitorHistory(current, snapshot));
      } catch (error: unknown) {
        if (cancelled) return;
        setState({ status: "error", message: errorMessage(error) });
      }

      if (!cancelled) {
        timer = setTimeout(sample, normalizeMonitorInterval(intervalMs));
      }
    };

    void sample();

    return () => {
      cancelled = true;
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [intervalMs, paused]);

  const snapshot = state.status === "ready" ? state.snapshot : null;

  return (
    <div className="page-stack monitor-page">
      <section className="monitor-hero">
        <div>
          <p className="eyebrow">System Monitor — P7</p>
          <h2>Live Windows telemetry without permanent background polling</h2>
          <p className="muted">
            Sampling runs only while this page is open and not paused. Unsupported
            hardware sources stay explicitly unavailable instead of being guessed.
          </p>
        </div>
        <div className="monitor-controls">
          <label>
            <span>Sampling interval</span>
            <select
              disabled={paused}
              onChange={(event) =>
                setIntervalMs(normalizeMonitorInterval(Number(event.target.value)))
              }
              value={intervalMs}
            >
              {MONITOR_INTERVALS_MS.map((value) => (
                <option key={value} value={value}>
                  {value / 1000}s
                </option>
              ))}
            </select>
          </label>
          <button
            className={paused ? "primary-action" : "secondary-action"}
            onClick={() => setPaused((current) => !current)}
            type="button"
          >
            {paused ? "Resume" : "Pause"}
          </button>
        </div>
      </section>

      {paused && (
        <section className="info-callout">
          <strong>Monitoring paused</strong>
          <span>No new native monitor samples are requested while paused.</span>
        </section>
      )}

      {state.status === "loading" && (
        <section className="scan-progress">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Reading Windows performance telemetry…</strong>
            <span>CPU, memory, disk, network, processes and capabilities.</span>
          </div>
        </section>
      )}

      {state.status === "error" && (
        <section className="info-callout health-error">
          <strong>System Monitor unavailable</strong>
          <span>{state.message}</span>
          <button
            className="secondary-action"
            onClick={() => {
              setPaused(true);
              setTimeout(() => setPaused(false), 0);
            }}
            type="button"
          >
            Retry
          </button>
        </section>
      )}

      {snapshot && (
        <>
          <section className="monitor-summary-grid">
            <article className="monitor-card">
              <div className="monitor-card-heading">
                <span>CPU</span>
                <Availability
                  available={snapshot.cpu.available}
                  reason={snapshot.cpu.reason}
                />
              </div>
              <strong>{formatPercent(snapshot.cpu.value)}</strong>
              <History history={history} field="cpu" label="Recent CPU history" />
            </article>

            <article className="monitor-card">
              <div className="monitor-card-heading">
                <span>Memory</span>
                <Availability
                  available={snapshot.memory.available}
                  reason={snapshot.memory.reason}
                />
              </div>
              <strong>{formatPercent(snapshot.memory.usedPercent)}</strong>
              <small>
                {formatBytes(snapshot.memory.usedBytes)} /{" "}
                {formatBytes(snapshot.memory.totalBytes)}
              </small>
              <History
                history={history}
                field="memory"
                label="Recent memory history"
              />
            </article>

            <article className="monitor-card">
              <div className="monitor-card-heading">
                <span>Disk</span>
                <Availability
                  available={snapshot.disk.available}
                  reason={snapshot.disk.reason}
                />
              </div>
              <dl className="monitor-pairs">
                <div>
                  <dt>Read</dt>
                  <dd>{formatRate(snapshot.disk.readBytesPerSec)}</dd>
                </div>
                <div>
                  <dt>Write</dt>
                  <dd>{formatRate(snapshot.disk.writeBytesPerSec)}</dd>
                </div>
              </dl>
            </article>

            <article className="monitor-card">
              <div className="monitor-card-heading">
                <span>Network</span>
                <Availability
                  available={snapshot.network.available}
                  reason={snapshot.network.reason}
                />
              </div>
              <dl className="monitor-pairs">
                <div>
                  <dt>Receive</dt>
                  <dd>{formatRate(snapshot.network.receiveBytesPerSec)}</dd>
                </div>
                <div>
                  <dt>Send</dt>
                  <dd>{formatRate(snapshot.network.sendBytesPerSec)}</dd>
                </div>
              </dl>
            </article>

            <article className="monitor-card">
              <div className="monitor-card-heading">
                <span>GPU</span>
                <Availability
                  available={snapshot.gpu.available}
                  reason={snapshot.gpu.reason}
                />
              </div>
              <strong>{formatPercent(snapshot.gpu.utilizationPercent)}</strong>
              <small>{snapshot.gpu.metricLabel}</small>
              {!snapshot.gpu.available && (
                <p className="monitor-reason">{snapshot.gpu.reason}</p>
              )}
            </article>

            <article className="monitor-card">
              <div className="monitor-card-heading">
                <span>Sensors</span>
                <Availability
                  available={snapshot.sensors.some((sensor) => sensor.available)}
                  reason="No supported generic temperature source."
                />
              </div>
              {snapshot.sensors.map((sensor) => (
                <div className="monitor-sensor" key={sensor.name}>
                  <strong>{sensor.name}</strong>
                  <span>
                    {sensor.available && sensor.valueCelsius !== null
                      ? `${sensor.valueCelsius.toFixed(1)} °C`
                      : "Unavailable"}
                  </span>
                  {!sensor.available && <small>{sensor.reason}</small>}
                </div>
              ))}
            </article>
          </section>

          <section className="monitor-process-card">
            <div className="section-heading">
              <div>
                <p className="eyebrow">Top consumers</p>
                <h3>Processes by current CPU activity</h3>
              </div>
              <div className="monitor-sample-meta">
                <span>
                  Sample: <strong>{snapshot.sampleDurationMs} ms</strong>
                </span>
                <span>
                  Updated:{" "}
                  <strong>
                    {new Date(snapshot.collectedAtEpochMs).toLocaleTimeString()}
                  </strong>
                </span>
              </div>
            </div>

            {snapshot.topProcesses.length === 0 ? (
              <div className="empty-health-state">
                <strong>Process telemetry unavailable.</strong>
                <span>Windows did not return supported process counters.</span>
              </div>
            ) : (
              <div className="monitor-process-table" role="table">
                <div className="monitor-process-row header" role="row">
                  <span>Process</span>
                  <span>PID</span>
                  <span>CPU</span>
                  <span>Memory</span>
                </div>
                {snapshot.topProcesses.map((process) => (
                  <div
                    className="monitor-process-row"
                    key={`${process.pid}-${process.name}`}
                    role="row"
                  >
                    <strong title={process.name}>{process.name}</strong>
                    <span>{process.pid}</span>
                    <span>{formatPercent(process.cpuPercent)}</span>
                    <span>{formatBytes(process.memoryBytes)}</span>
                  </div>
                ))}
              </div>
            )}
          </section>
        </>
      )}
    </div>
  );
}

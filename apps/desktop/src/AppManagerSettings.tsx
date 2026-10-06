import { useEffect, useState } from "react";

import {
  appManagerApprovalLabel,
  appManagerConnectionLabel,
  getAppManagerState,
  setAppManagerEndpoint,
  syncAppManagerNow,
  type AppManagerState,
} from "./appManager";

type ViewState =
  | { status: "loading" }
  | { status: "ready"; value: AppManagerState }
  | { status: "error"; message: string };

function messageFrom(error: unknown) {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof (error as { message?: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }
  return error instanceof Error ? error.message : "App Manager request failed.";
}

export function AppManagerSettings() {
  const [view, setView] = useState<ViewState>({ status: "loading" });
  const [endpoint, setEndpoint] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let active = true;

    const refresh = () => {
      getAppManagerState()
        .then((value) => {
          if (!active) return;
          setView({ status: "ready", value });
          setEndpoint((current) => current || value.endpoint || "");
        })
        .catch((error: unknown) => {
          if (active) setView({ status: "error", message: messageFrom(error) });
        });
    };

    refresh();
    const timer = window.setInterval(refresh, 15_000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, []);

  const update = async (action: () => Promise<AppManagerState>) => {
    setBusy(true);
    try {
      const value = await action();
      setView({ status: "ready", value });
      setEndpoint(value.endpoint ?? "");
    } catch (error: unknown) {
      setView({ status: "error", message: messageFrom(error) });
    } finally {
      setBusy(false);
    }
  };

  const state = view.status === "ready" ? view.value : null;

  return (
    <section className="settings-card app-manager-settings">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Application Management — P8</p>
          <h2>Desktop Agent Gateway</h2>
        </div>
        {state && (
          <span
            className={
              state.online
                ? "status-badge good"
                : state.configured
                  ? "status-badge warning"
                  : "status-badge neutral"
            }
          >
            {appManagerConnectionLabel(state)}
          </span>
        )}
      </div>

      <p className="muted">
        PC Manager opens an outbound HTTPS connection only. No inbound Windows
        port is created. The private P-256 device key stays in the Windows CNG
        user key store and is never sent to App Manager.
      </p>

      <div className="app-manager-endpoint">
        <label htmlFor="app-manager-endpoint">
          <span>App Manager origin</span>
          <input
            id="app-manager-endpoint"
            onChange={(event) => setEndpoint(event.target.value)}
            placeholder="https://your-application-management-origin"
            spellCheck={false}
            type="url"
            value={endpoint}
          />
        </label>
        <div className="app-manager-actions">
          <button
            className="secondary-action"
            disabled={busy}
            onClick={() =>
              void update(() => setAppManagerEndpoint(endpoint.trim() || null))
            }
            type="button"
          >
            Save & connect
          </button>
          <button
            className="secondary-action"
            disabled={busy || !state?.configured}
            onClick={() => void update(syncAppManagerNow)}
            type="button"
          >
            Sync now
          </button>
          <button
            className="secondary-action"
            disabled={busy || (!endpoint && !state?.configured)}
            onClick={() => void update(() => setAppManagerEndpoint(null))}
            type="button"
          >
            Disconnect
          </button>
        </div>
      </div>

      {view.status === "loading" && (
        <div className="info-callout">
          <strong>Reading App Manager state…</strong>
          <span>Only local agent state is read here.</span>
        </div>
      )}

      {view.status === "error" && (
        <div className="info-callout health-error">
          <strong>App Manager state unavailable</strong>
          <span>{view.message}</span>
        </div>
      )}

      {state && (
        <>
          <div className="app-manager-state-grid">
            <div>
              <span>Device</span>
              <strong>{state.deviceCode ?? "Not registered"}</strong>
            </div>
            <div>
              <span>Approval</span>
              <strong>{appManagerApprovalLabel(state.approvalState)}</strong>
            </div>
            <div>
              <span>License</span>
              <strong>{state.entitlementState}</strong>
            </div>
            <div>
              <span>Release</span>
              <strong>{state.releaseChannel}</strong>
            </div>
            <div>
              <span>Updates</span>
              <strong>{state.updatePolicy}</strong>
            </div>
            <div>
              <span>App version</span>
              <strong>{state.appVersion}</strong>
            </div>
          </div>
          <div className="app-manager-message">
            <strong>{state.message}</strong>
            <span>
              {state.lastSuccessEpochMs
                ? `Last successful contact: ${new Date(
                    state.lastSuccessEpochMs,
                  ).toLocaleString()}`
                : "No successful heartbeat yet."}
              {" · "}
              Retry/heartbeat: {state.retryAfterSeconds}s
            </span>
          </div>
        </>
      )}
    </section>
  );
}

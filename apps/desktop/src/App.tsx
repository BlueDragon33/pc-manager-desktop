import { useEffect, useState } from "react";

import { type AppInfo, getAppInfo } from "./appInfo";
import "./styles.css";

type LoadState =
  | { status: "loading" }
  | { status: "ready"; info: AppInfo }
  | { status: "error"; message: string };

export default function App() {
  const [state, setState] = useState<LoadState>({ status: "loading" });

  useEffect(() => {
    let active = true;

    getAppInfo()
      .then((info) => {
        if (active) {
          setState({ status: "ready", info });
        }
      })
      .catch((error: unknown) => {
        if (active) {
          const message =
            error instanceof Error ? error.message : "Native bridge unavailable.";
          setState({ status: "error", message });
        }
      });

    return () => {
      active = false;
    };
  }, []);

  return (
    <main className="shell">
      <section className="foundation-card" aria-labelledby="app-title">
        <p className="eyebrow">Windows native utility</p>
        <h1 id="app-title">PC Manager Desktop</h1>
        <p className="subtitle">
          Safe, transparent system management. This build only proves the P0
          application foundation.
        </p>

        <div className="status-panel">
          <h2>Foundation status</h2>
          <ul>
            <li>
              <span aria-hidden="true">✓</span> React UI
            </li>
            <li>
              <span aria-hidden="true">✓</span> Tauri shell
            </li>
            <li>
              <span aria-hidden="true">✓</span> Rust workspace
            </li>
          </ul>
        </div>

        <div className="bridge-panel" aria-live="polite">
          {state.status === "loading" && <p>Checking native bridge…</p>}
          {state.status === "error" && (
            <p className="bridge-error">
              Native bridge check failed: {state.message}
            </p>
          )}
          {state.status === "ready" && (
            <>
              <p className="bridge-ok">Native bridge connected</p>
              <dl>
                <div>
                  <dt>App ID</dt>
                  <dd>{state.info.appId}</dd>
                </div>
                <div>
                  <dt>Platform</dt>
                  <dd>{state.info.platform}</dd>
                </div>
                <div>
                  <dt>Device type</dt>
                  <dd>{state.info.deviceType}</dd>
                </div>
              </dl>
            </>
          )}
        </div>

        <p className="phase">Phase: P0 — Repository Foundation</p>
      </section>
    </main>
  );
}

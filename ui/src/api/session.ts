import { LocalApi } from "./client";

const storageKey = "wafflery.session.v1";
type Connection = { api: LocalApi; hostSession: string };
let connecting: Promise<Connection | null> | undefined;

function isSession(value: unknown): value is { token: string; host_session: string } {
  if (!value || typeof value !== "object") return false;
  return "token" in value && typeof value.token === "string" && /^[a-f0-9]{64}$/.test(value.token)
    && "host_session" in value && typeof value.host_session === "string" && /^[a-f0-9]{64}$/.test(value.host_session);
}

async function initialize(given?: string): Promise<Connection | null> {
  const fragment = new URLSearchParams(window.location.hash.slice(1));
  const bootstrap = fragment.get("bootstrap") ?? given;
  // Remove the fragment before starting any network request, including failed exchanges.
  if (window.location.hash) window.history.replaceState(window.history.state, "", window.location.pathname + window.location.search);
  try {
    let session: unknown;
    if (bootstrap) {
      window.sessionStorage.removeItem(storageKey);
      const response = await fetch("/api/session", {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ bootstrap }), credentials: "omit", mode: "same-origin", redirect: "error",
      });
      if (!response.ok) throw new Error("This opening link expired or was already used. Run wafflery to open a fresh session.");
      session = await response.json();
    } else {
      const saved = window.sessionStorage.getItem(storageKey);
      if (!saved) return null;
      session = JSON.parse(saved) as unknown;
    }
    if (!isSession(session)) throw new Error("The workspace session is unavailable. Run wafflery to reopen it.");
    const api = new LocalApi(window.location.origin, session.token);
    await api.request("/api/state");
    window.sessionStorage.setItem(storageKey, JSON.stringify(session));
    return { api, hostSession: session.host_session };
  } catch (error) {
    window.sessionStorage.removeItem(storageKey);
    throw error;
  }
}

/** Development only: Vite's dev server asks the running host for a fresh bootstrap (ui/dev-host.ts). */
async function devBootstrap(): Promise<string | null> {
  const response = await fetch("/__wafflery/bootstrap", { credentials: "omit", cache: "no-store" }).catch(() => null);
  if (!response?.ok) return null;
  const body: unknown = await response.json().catch(() => null);
  return body && typeof body === "object" && "bootstrap" in body && typeof body.bootstrap === "string" ? body.bootstrap : null;
}

/** Development only: join the host without a launcher link, and rejoin it after it restarts. */
async function initializeDev(): Promise<Connection | null> {
  let connection: Connection | null;
  try {
    connection = await initialize();
  } catch (error) {
    const bootstrap = await devBootstrap();
    if (!bootstrap) throw error;
    connection = await initialize(bootstrap);
  }
  if (!connection) {
    const bootstrap = await devBootstrap();
    connection = bootstrap ? await initialize(bootstrap) : null;
  }
  // A restarted host forgets its sessions; reloading rejoins it with a fresh bootstrap.
  if (connection) connection.api.onSessionEnded = () => window.location.reload();
  return connection;
}

// React StrictMode may mount twice. One document performs only one bootstrap exchange.
export function connect(): Promise<Connection | null> {
  connecting ??= import.meta.env.DEV ? initializeDev() : initialize();
  return connecting;
}

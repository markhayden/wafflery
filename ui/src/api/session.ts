import { LocalApi } from "./client";

const storageKey = "wafflehouse.session.v1";
type Connection = { api: LocalApi; hostSession: string };
let connecting: Promise<Connection | null> | undefined;

function isSession(value: unknown): value is { token: string; host_session: string } {
  if (!value || typeof value !== "object") return false;
  return "token" in value && typeof value.token === "string" && /^[a-f0-9]{64}$/.test(value.token)
    && "host_session" in value && typeof value.host_session === "string" && /^[a-f0-9]{64}$/.test(value.host_session);
}

async function initialize(): Promise<Connection | null> {
  const fragment = new URLSearchParams(window.location.hash.slice(1));
  const bootstrap = fragment.get("bootstrap");
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
      if (!response.ok) throw new Error("This opening link expired or was already used. Run wafflehouse to open a fresh session.");
      session = await response.json();
    } else {
      const saved = window.sessionStorage.getItem(storageKey);
      if (!saved) return null;
      session = JSON.parse(saved) as unknown;
    }
    if (!isSession(session)) throw new Error("The workspace session is unavailable. Run wafflehouse to reopen it.");
    const api = new LocalApi(window.location.origin, session.token);
    await api.request("/api/state");
    window.sessionStorage.setItem(storageKey, JSON.stringify(session));
    return { api, hostSession: session.host_session };
  } catch (error) {
    window.sessionStorage.removeItem(storageKey);
    throw error;
  }
}

// React StrictMode may mount twice. One document performs only one bootstrap exchange.
export function connect(): Promise<Connection | null> {
  connecting ??= initialize();
  return connecting;
}

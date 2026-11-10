import { useEffect, useState } from "react";
import { Button } from "../../components/Button";
import type { Installable, Json } from "../actions/model";
import styles from "./Installables.module.css";

const stateLabel: Record<Installable["state"], string> = {
  installed: "Ready", not_installed: "Not installed", installing: "Installing", unavailable: "Unavailable",
};
/** "3.4 GB", "120 MB": decimal units, as download sizes are quoted. */
export function bytes(value: number): string {
  if (value >= 1e9) return `${(value / 1e9).toFixed(1)} GB`;
  if (value >= 1e6) return `${Math.round(value / 1e6)} MB`;
  return `${Math.max(1, Math.round(value / 1e3))} kB`;
}
/** A tool's own extension may carry a lab line (`x-<tool>.lab`): its best use and remove score. */
function labLine(item: Installable): string | undefined {
  for (const [key, value] of Object.entries(item)) {
    if (!key.startsWith("x-") || !value || typeof value !== "object" || Array.isArray(value)) continue;
    const lab = (value as Record<string, Json>).lab;
    if (!lab || typeof lab !== "object" || Array.isArray(lab)) continue;
    const parts: string[] = [];
    if (typeof lab.best_for === "string") parts.push(`Best for ${lab.best_for}`);
    if (typeof lab.remove === "number") parts.push(`lab remove score ${lab.remove.toFixed(3)}`);
    if (parts.length) return parts.join(" · ");
  }
  return undefined;
}
function outcomeLine(item: Installable): string | undefined {
  const outcome = item.last_outcome;
  if (!outcome || outcome.state === "succeeded") return undefined;
  const what = outcome.action === "install" ? "The last install" : "The last removal";
  return outcome.state === "cancelled"
    ? `${what} was cancelled.`
    : `${what} failed${outcome.error ? `: ${outcome.error.message}` : "."}`;
}

type Props = {
  installables: Installable[];
  /** Starts an install; the tool owns it from then on. */
  onInstall: (id: string) => Promise<void>;
  /** Removes an installed one, or cancels one that is installing. */
  onUninstall: (id: string) => Promise<void>;
  /** Rereads the description; called while anything is installing. */
  onRefresh: () => Promise<void>;
  /** How often to reread while installing, in milliseconds. */
  pollMs?: number;
};
/** The tool's installables (table-manners 0.6 draft, 3.5): what each is, its state, and Get or
 * Remove. An install keeps going if this page closes; the list catches up when it is reopened. */
export function Installables({ installables, onInstall, onUninstall, onRefresh, pollMs = 1500 }: Props) {
  const [confirming, setConfirming] = useState<string>();
  const [pending, setPending] = useState<string>();
  const [error, setError] = useState<string>();
  const installing = installables.some(item => item.state === "installing");
  useEffect(() => {
    if (!installing) return;
    const timer = setInterval(() => { void onRefresh().catch(() => undefined); }, pollMs);
    return () => clearInterval(timer);
  }, [installing, onRefresh, pollMs]);
  const run = async (id: string, work: (id: string) => Promise<void>) => {
    setPending(id); setError(undefined); setConfirming(undefined);
    try { await work(id); await onRefresh(); }
    catch (failure) { setError(failure instanceof Error ? failure.message : "The tool could not do that."); }
    finally { setPending(undefined); }
  };
  return <section aria-label="Installables" className={styles.installables}>
    <h3>Models</h3>
    <p>Models are a setup step: nothing downloads until you choose Get. Each runs on this machine once installed.</p>
    {error && <p role="alert">{error}</p>}
    <ul>{installables.map(item => {
      const lab = labLine(item);
      const outcome = outcomeLine(item);
      const busy = pending === item.id;
      return <li key={item.id} className={styles.row} aria-label={item.title}>
        <div className={styles.head}>
          <strong>{item.title}</strong>
          <span className={styles.state} data-state={item.state}>{stateLabel[item.state]}</span>
        </div>
        {lab && <p>{lab}</p>}
        {item.notes && <p>{item.notes}</p>}
        <p className={styles.facts}>{bytes(item.download_bytes)} download · {item.licence}</p>
        {item.state === "installing" && <div className={styles.track} role="progressbar" aria-label={`${item.title} install progress`}
          aria-valuemin={0} aria-valuemax={100} aria-valuenow={item.progress === undefined ? undefined : Math.round(item.progress * 100)}>
          <span className={styles.bar} style={{ width: `${Math.round((item.progress ?? 0) * 100)}%` }} />
        </div>}
        {outcome && <p role="status">{outcome}</p>}
        {confirming === item.id ? <div className={styles.confirm} role="group" aria-label={`Confirm getting ${item.title}`}>
          <p>Download {bytes(item.download_bytes)} ({bytes(item.installed_bytes)} on disk) under the {item.licence} licence?</p>
          <div className={styles.actions}>
            <Button variant="primary" onClick={() => void run(item.id, onInstall)}>Get {item.title}</Button>
            <Button onClick={() => setConfirming(undefined)}>Not now</Button>
          </div>
        </div> : <div className={styles.actions}>
          {item.state === "not_installed" && <Button busy={busy} onClick={() => setConfirming(item.id)}>Get</Button>}
          {item.state === "installing" && <Button busy={busy} onClick={() => void run(item.id, onUninstall)}>Cancel</Button>}
          {item.state === "installed" && <Button variant="destructive" busy={busy} onClick={() => void run(item.id, onUninstall)}>Remove</Button>}
        </div>}
      </li>;
    })}</ul>
  </section>;
}

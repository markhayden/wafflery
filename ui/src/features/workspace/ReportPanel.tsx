import { useEffect, useState } from "react";
import { Button } from "../../components/Button";
import { CodeBlock } from "../../components/CodeBlock";
import { ActionRow } from "../../components/ActionRow";
import { Picker } from "../../components/Picker";
import type { ReportSnapshot } from "./report-owner";
import styles from "./ReportPanel.module.css";
export type ReportOption = { name: string; kind: "server" | "document" };
type Props = {
  attempt: string; choices: ReportOption[]; selection: string; snapshot: ReportSnapshot;
  onSelect: (name: string) => void; onOpen: () => void; onClose: () => void;
  onRecover: () => void; onForce: () => void;
};
export function ReportPanel({ attempt, choices, selection, snapshot, onSelect, onOpen, onClose, onRecover, onForce }: Props) {
  const { view, pending, uncertain, error } = snapshot;
  const [confirming, setConfirming] = useState(false);
  const [interacted, setInteracted] = useState(false);
  const [failedUrl, setFailedUrl] = useState<string | null>(null);
  const selected = view?.attempt === attempt && view.name === selection;
  const active = !!view && view.phase !== "exited";
  const relevant = selected || active || uncertain;
  const url = selected ? snapshot.url : null;
  useEffect(() => { setConfirming(false); }, [view?.id, view?.phase]);
  const message = pending ? "Updating report…" : uncertain ? "Report ownership is unconfirmed."
    : view?.phase === "needs_force" ? "Report did not stop."
    : selected && view?.error ? "Report unavailable"
    : selected && view?.idle_expired ? "Report closed after inactivity."
    : active && !selected ? "A previous report is still open."
    : selected && view?.phase === "starting" ? "Starting report…"
    : selected && view?.phase === "stopping" ? "Closing report…"
    : selected && view?.phase === "ready" ? (url ? "Report ready" : "Preparing report…")
    : "Choose a report to inspect this saved run.";
  return <section aria-label="Tool report" className={styles.panel}>
    {choices.length ? <Picker label="Report" value={selection} choices={choices.map(choice => ({
      id:choice.name, title:choice.name, description:choice.kind === "server" ? "Live viewer" : "Document",
    }))} onChange={onSelect} disabled={pending || uncertain} /> : <p>No report views are available for this saved run.</p>}
    <ActionRow>
      <Button onClick={onOpen} disabled={!selection || !choices.length || pending || uncertain || active}>Open report</Button>
      {active && <Button onClick={onClose} disabled={pending || uncertain}>Close report</Button>}
      {((view && relevant) || uncertain) && <Button onClick={onRecover} disabled={pending}>Check report status</Button>}
    </ActionRow>
    <p role="status" className={styles.status}>{message}</p>
    {(error || (relevant && view?.error)) && <div role="alert" className={styles.notice}>
      <p>{error ?? view?.error}</p><p>Saved inputs and results remain available in the other tabs.</p>
    </div>}
    {uncertain && <p>Check the report status before opening another. An unconfirmed open request may require restarting Wafflery.</p>}
    {active && !selected && <p>Previous report: {view?.name}</p>}
    {view?.phase === "needs_force" && !uncertain && <div className={styles.notice}>
      {confirming ? <>
        <p>Force stop this report and its owned helper processes? The saved run will remain available.</p>
        <ActionRow>
          <Button autoFocus onClick={() => setConfirming(false)}>Keep report</Button>
          <Button variant="destructive" disabled={pending} onClick={() => { setConfirming(false); onForce(); }}>Confirm force stop</Button>
        </ActionRow>
      </> : <Button autoFocus={interacted} disabled={pending} onClick={() => { setInteracted(true); setConfirming(true); }}>Force stop report</Button>}
    </div>}
    {url && failedUrl !== url ? <iframe key={url} title="Report viewer" src={url} referrerPolicy="no-referrer"
      className={styles.frame} onError={() => setFailedUrl(url)} />
      : <div className={styles.empty} aria-busy={pending || (selected && view?.phase === "starting") || undefined}>
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.25" aria-hidden="true">
          <path d="M5 3h10l4 4v14H5zM15 3v5h4M8 12h8M8 16h6" />
        </svg>
        <p>{url && failedUrl === url ? "The report frame could not load. Close it and open the report again." : message}</p>
      </div>}
    {view && relevant && <details className={styles.diagnostics}>
      <summary>Report diagnostics</summary>
      <p>{view.name} · {view.phase}{view.exit_code !== null ? ` · Exit ${view.exit_code}` : ""}</p>
      <CodeBlock label="Report diagnostics output" copyLabel="Copy diagnostics" value={view.diagnostics || "No diagnostic output."}/>
      {view.diagnostics_truncated && <p>Diagnostic output was truncated.</p>}
    </details>}
    <p className={styles.note}>Reports are opened from the saved run. Closing inspection releases the viewer; inactive reports close automatically.</p>
  </section>;
}

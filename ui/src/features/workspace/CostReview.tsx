import type {BindingIssue} from "../../api/client";
import { useEffect, useId, useRef } from "react";
import { Button } from "../../components/Button";
import { positiveCap, type Estimate } from "../../api/estimates";
import { usd, type ToolSession } from "../../api/tool-session";
import styles from "./CostReview.module.css";

export type CostState = { kind: "idle" | "stale" }
  | { kind: "pending"; stale?: boolean }
  | { kind: "error"; message: string; bindings?: BindingIssue[] }
  | { kind: "quoted"; estimate: Estimate };
type Props = {
  cap: string;
  onCapChange: (value: string) => void;
  state: CostState;
  onEstimate: () => void;
  onSetup: (opener: HTMLButtonElement) => void;
  session?: ToolSession;
  disabled?: boolean;
};
export function CostReview({ cap, onCapChange, state, onEstimate, onSetup, session, disabled = false }: Props) {
  const id = useId(), input = useRef<HTMLInputElement>(null);
  const valid = positiveCap(cap) !== undefined;
  const invalid = cap.length > 0 && !valid;
  useEffect(() => { input.current?.setCustomValidity(valid ? "" : "Enter a finite amount greater than zero."); }, [valid]);
  const observed = state.kind === "quoted" ? state.estimate : undefined;
  const changedSession = !!observed && !!session && observed.session.id !== session.id;
  const quote = changedSession ? undefined : observed;
  const ledger = session ?? quote?.session;
  const pending = state.kind === "pending";
  return <section className={styles.cost} aria-labelledby={`${id}-heading`} aria-busy={pending}>
    <h3 id={`${id}-heading`}>Cost</h3>
    <label className={styles.cap} htmlFor={`${id}-cap`}>
      <span>Spending cap (USD)</span>
      <input ref={input} id={`${id}-cap`} type="text" inputMode="decimal" required
        value={cap} disabled={disabled} aria-invalid={invalid} aria-describedby={`${id}-help`}
        onChange={event => onCapChange(event.target.value)} />
    </label>
    <p id={`${id}-help`}>{invalid ? "Enter a finite amount greater than zero."
      : "The most you are willing to spend on this run. Estimating does not run the action."}</p>
    <div className={styles.actions}>
      <Button disabled={disabled || !valid} busy={pending} onClick={event => {
        if (valid && event.currentTarget.form?.reportValidity() !== false) onEstimate();
      }}>Estimate cost</Button>
    </div>
    {state.kind === "pending" && <p role="status">{state.stale ? "Waiting for the previous estimate to finish…" : "Estimating these inputs…"}</p>}
    {(state.kind === "stale" || changedSession) && <p role="status">Estimate again for the current inputs and session.</p>}
    {state.kind === "error" && <p role="alert">{state.message}</p>}
    {quote && <div className={styles.estimate}>
      <p role="status" className={styles.decision}>{quote.allowed ? "Estimate within cap." :
        quote.refused === "readiness" ? (quote.readiness.state === "unavailable"
          ? "This action is unavailable in the current tool environment." : "Setup is needed before these inputs can run.") :
        quote.refused === "cap" ? "Estimated cost exceeds this cap." : "The tool’s remaining session budget is too low."}</p>
      <dl className={styles.amounts}>
        <div><dt>Worst case</dt><dd>{quote.worst_case_usd === null ? "Cost unavailable" : usd(quote.worst_case_usd)}</dd></div>
        <div><dt>Estimated cap</dt><dd>{quote.cap_usd === null ? "Unavailable" : usd(quote.cap_usd)}</dd></div>
        {quote.effective.model !== null && <div><dt>Model</dt><dd>{quote.effective.model}</dd></div>}
      </dl>
      {quote.refused === "readiness" && <div className={styles.requirements}>
        {quote.readiness.requirements.length > 0 && <ul>{quote.readiness.requirements.map((item, index) =>
          <li key={`${item.kind}:${item.name}:${index}`}><p><code>{item.name}</code> · {item.state}</p>{item.setup && <p>{item.setup}</p>}</li>)}</ul>}
        <Button onClick={event => onSetup(event.currentTarget)}>Tool setup</Button>
      </div>}
      <details><summary>Estimate details</summary><dl className={styles.details}>
        <dt>Configuration</dt><dd><code>{quote.effective.config_identity ?? "Unavailable"}</code></dd>
        <dt>Session</dt><dd><code>{quote.session.id}</code></dd>
        <dt>Expires</dt><dd>{new Date(quote.expires).toLocaleString()}</dd>
      </dl></details>
    </div>}
    {ledger && <div className={styles.ledger}>
      <dl className={styles.amounts}><div><dt>Session remaining</dt><dd>{ledger.remaining_usd === null ? "No session limit" : usd(ledger.remaining_usd)}</dd></div></dl>
      {ledger.unresolved_usd > 0 && <p>Reported spend includes {usd(ledger.unresolved_usd)} whose final charge is unknown.</p>}
      {quote?.allowed && quote.worst_case_usd !== null && ledger.remaining_usd !== null && quote.worst_case_usd > ledger.remaining_usd + 1e-6 &&
        <p role="status">The current session balance may no longer cover this estimate.</p>}
      <p>The tool checks the current budget again when you run. An estimate does not reserve funds.</p>
    </div>}
  </section>;
}

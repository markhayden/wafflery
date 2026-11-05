import type {BindingIssue} from "../../api/client";
import { useEffect, useId, useRef } from "react";
import { Button } from "../../components/Button";
import { FieldTag } from "../../components/FieldTag";
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
  // Optional: empty means the tool's own limits apply.
  const invalid = cap.trim().length > 0 && positiveCap(cap) === undefined;
  useEffect(() => { input.current?.setCustomValidity(invalid ? "Enter an amount above zero, or leave it empty." : ""); }, [invalid]);
  const observed = state.kind === "quoted" ? state.estimate : undefined;
  const changedSession = !!observed && !!session && observed.session.id !== session.id;
  const quote = changedSession ? undefined : observed;
  const ledger = session ?? quote?.session;
  const pending = state.kind === "pending";
  return <section className={styles.cost} aria-label="Cost options" aria-busy={pending}>
    <div className={styles.cap}>
      <label className={styles.label} htmlFor={`${id}-cap`}>Spending limit (USD) <FieldTag required={false} /></label>
      <span className={styles.capRow}>
        <input ref={input} id={`${id}-cap`} type="text" inputMode="decimal" placeholder="The tool's own limit"
          value={cap} disabled={disabled} aria-invalid={invalid} aria-describedby={`${id}-help`}
          onChange={event => onCapChange(event.target.value)} />
        <Button disabled={disabled || invalid} busy={pending} onClick={event => {
          event.preventDefault();
          if (!invalid && event.currentTarget.form?.reportValidity() !== false) onEstimate();
        }}>Check cost</Button>
      </span>
    </div>
    <p id={`${id}-help`}>{invalid ? "Enter an amount above zero, or leave it empty."
      : "The most this run may spend. Check cost shows what it may cost; nothing runs."}</p>
    {state.kind === "pending" && <p role="status">{state.stale ? "Waiting for the previous check to finish…" : "Checking what this run may cost…"}</p>}
    {(state.kind === "stale" || changedSession) && <p role="status">The inputs changed since the last check. Check again, or just run.</p>}
    {state.kind === "error" && <p role="alert">{state.message}</p>}
    {quote && <div className={styles.estimate}>
      <p role="status" className={styles.decision}>{quote.allowed ? "Within the limit. Running uses this price." :
        quote.refused === "readiness" ? (quote.readiness.state === "unavailable"
          ? "This action is unavailable in the current tool environment." : "Setup is needed before these inputs can run.") :
        quote.refused === "cap" ? "This run may cost more than the limit." : "The tool’s remaining session budget is too low."}</p>
      <dl className={styles.amounts}>
        <div><dt>Worst case</dt><dd>{quote.worst_case_usd === null ? "Cost unavailable" : usd(quote.worst_case_usd)}</dd></div>
        <div><dt>Limit</dt><dd>{quote.cap_usd === null ? "None" : usd(quote.cap_usd)}</dd></div>
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
    {/* Only a tool with a session budget (its config's total across runs) has anything to say. */}
    {ledger && (ledger.budget_usd !== null || ledger.unresolved_usd > 0) && <div className={styles.ledger}>
      {ledger.budget_usd !== null && ledger.remaining_usd !== null && <p>Left to spend this session:{" "}
        <span className={styles.value}>{usd(ledger.remaining_usd)} of {usd(ledger.budget_usd)}</span></p>}
      {ledger.unresolved_usd > 0 && <p>Reported spend includes {usd(ledger.unresolved_usd)} whose final charge is unknown.</p>}
      {quote?.allowed && quote.worst_case_usd !== null && ledger.remaining_usd !== null && quote.worst_case_usd > ledger.remaining_usd + 1e-6 &&
        <p role="status">The current session balance may no longer cover this estimate.</p>}
    </div>}
  </section>;
}

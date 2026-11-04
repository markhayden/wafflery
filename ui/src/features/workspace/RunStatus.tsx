import type { Attempt } from "../../api/workspace";
import { usd } from "../../api/tool-session";
import { runOutcome, upperUsd } from "./run-outcome";
import styles from "./RunStatus.module.css";

type Tone = "idle" | "running" | "done" | "failed" | "muted";
/** The execution state, in a person's words, and the tone its dot takes. */
function describe(execution: string | undefined, status: string | undefined): { text: string; tone: Tone } {
  if (!execution) return { text: "Ready", tone: "idle" };
  if (execution === "unconfirmed") return { text: "Cancellation requested · outcome unconfirmed", tone: "muted" };
  if (["submitted", "running"].includes(execution)) return { text: "Running", tone: "running" };
  if (execution === "cancelling") return { text: "Cancelling", tone: "running" };
  const final = status ?? execution;
  if (final === "completed") return { text: "Completed", tone: "done" };
  if (final === "cancelled") return { text: "Cancelled", tone: "muted" };
  if (execution === "rejected") return { text: "Not run", tone: "muted" };
  return { text: final.charAt(0).toUpperCase() + final.slice(1), tone: "failed" };
}

/** A saved run's state in the same words and dot, for lists of runs. */
export function RunState({ execution }: { execution: string }) {
  const state = describe(execution, undefined);
  return <span className={styles.state} data-tone={state.tone}><span className={styles.dot} aria-hidden="true" />{state.text}</span>;
}

/** The Result panel's corner: where the run stands, and what it reported spending. */
export function RunStatus({ attempt, paid = false }: { attempt: Pick<Attempt, "execution" | "result" | "observations" | "raw_response"> | undefined; paid?: boolean }) {
  const result = attempt?.result && typeof attempt.result === "object" && !Array.isArray(attempt.result) ? attempt.result as { status?: unknown } : undefined;
  const state = describe(attempt?.execution, typeof result?.status === "string" ? result.status : undefined);
  const view = attempt ? runOutcome({ ...attempt, paid }) : undefined;
  const spend = view?.spend;
  const spent = !view ? undefined : spend ? (spend.unresolved > 0 ? `At most ${upperUsd(spend.usd)}` : usd(spend.usd))
    : view.kind === "waiting" ? "Spend pending" : "Spend unavailable";
  return <div className={styles.status}>
    <span role="status" className={styles.state} data-tone={state.tone}><span className={styles.dot} aria-hidden="true" />{state.text}</span>
    {spent && <span className={styles.spend} aria-label="Reported spend" title="Reported spend">{spent}</span>}
  </div>;
}

import type { ComponentProps } from "react";
import type { Attempt } from "../../api/workspace";
import { Button } from "../../components/Button";
import { runOutcome, upperUsd } from "./run-outcome";
import styles from "./RunOutcome.module.css";
type Props = {
  attempt: Pick<Attempt,"execution" | "result" | "observations" | "raw_response">;
  paid?: boolean;
  onSetup: NonNullable<ComponentProps<typeof Button>["onClick"]>;
};
export function RunOutcome({attempt,onSetup,paid = false}: Props) {
  const view = runOutcome({...attempt,paid}), spend = view.spend;
  // A plain completed run has nothing to say here: its status and spend are in the panel heading.
  const unresolved = !!spend && spend.unresolved > 0, estimated = spend?.basis === "estimated" && spend.usd > spend.unresolved;
  if (view.title === "Run completed." && !view.reason && !view.guidance && !view.requirements.length && !view.setup && !unresolved && !estimated) return null;
  return <section className={styles.outcome} aria-label="Run outcome">
    {view.title !== "Run completed." && <p role={view.kind === "no-run" ? "status" : undefined} className={styles.title}>{view.title}</p>}
    {view.reason && <p>{view.reason}</p>}
    {view.guidance && <p>{view.guidance}</p>}
    {view.requirements.length > 0 && <ul>{view.requirements.map((requirement,index) => <li key={index}>
      <code>{requirement.name}</code> · {requirement.state}{requirement.setup && <p>{requirement.setup}</p>}
    </li>)}</ul>}
    {spend && spend.unresolved > 0 && <p>Includes at most {upperUsd(spend.unresolved)} unresolved. The final charge is unknown; cancellation or failure does not erase it.</p>}
    {spend?.basis === "estimated" && spend.usd > spend.unresolved && <p>The settled portion uses the tool’s configured prices.</p>}
    {view.setup && <Button onClick={onSetup}>Review tool setup</Button>}
  </section>;
}

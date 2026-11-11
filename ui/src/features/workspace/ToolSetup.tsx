import { DirectoryAccess } from "./DirectoryAccess";
import type { DirectoryGrants } from "../../api/directory-grants";
import { useEffect, useState } from "react";
import { Button } from "../../components/Button";
import type { Action, Readiness, Requirement, Schema } from "../actions/model";
import { readinessFor, unmet } from "../actions/choices";
import { usd, type ToolSession } from "../../api/tool-session";
import { requirementPurpose } from "./checks";
import styles from "./ToolSetup.module.css";

const readinessLabel: Record<Readiness["state"], string> = {
  ready: "Ready", needs_setup: "Setup needed", unavailable: "Unavailable",
};
const requirementState: Record<Requirement["state"], string> = {
  missing: "Missing", present: "Present", valid: "Validated", invalid: "Invalid", unknown: "Unknown",
};
const requirementKind: Record<Requirement["kind"], string> = {
  config: "Configuration", credential: "Credential", env: "Environment variable", model: "Model", file: "File", dependency: "Dependency",
};
type Props = {
  tool: { name: string; version: string };
  action: Action;
  loadSession: () => Promise<ToolSession>;
  directoryGrants?: DirectoryGrants | null;
  /** The inputs the requirements are for: a requirement for one choice shows only when they pick it. */
  params?: Schema;
};

/** One requirement: its name and state, what kind it is, and why it is listed. */
function RequirementItem({ item }: { item: Requirement }) {
  return <li>
    <p><code>{item.name}</code> · <span>{requirementState[item.state]}{item.optional ? " · optional" : ""}</span></p>
    <p>{requirementKind[item.kind]}</p>
    {item.optional && <p className={styles.optional}>{requirementPurpose(item)}</p>}
    {item.for_choice && <p className={styles.optional}>Needed because {item.for_choice.param} is {item.for_choice.value}.</p>}
    {item.setup && <p>{item.setup}</p>}
  </li>;
}
/** The requirements for the current inputs: what is missing, listed; everything else behind one
 * disclosure, so a ready action reads as ready. "Present" is explained once, not on every line. */
export function Requirements({ requirements, conditional = false, expanded = false, onExpanded }: {
  requirements: Requirement[];
  /** Whether the collapsed requirements are shown; kept by the caller so a refresh keeps it. */
  expanded?: boolean;
  onExpanded?: (expanded: boolean) => void;
  /** Some of the action's requirements are for one choice and are left out unless it is picked. */
  conditional?: boolean;
}) {
  if (requirements.length === 0) return <><h4>Setup requirements</h4><p>No setup requirements are declared for this action.</p></>;
  const missing = requirements.filter(unmet);
  const others = requirements.filter(item => !unmet(item));
  return <>
    <h4>Setup requirements</h4>
    {conditional && <p>Listed for the current settings: a requirement for another choice shows when you pick that choice.</p>}
    {missing.length > 0 && <>
      <p>Needed before this runs:</p>
      <ul aria-label="Missing requirements">{missing.map((item, index) => <RequirementItem key={`${item.kind}:${item.name}:${index}`} item={item} />)}</ul>
    </>}
    {others.length > 0 && <details className={styles.requirements} open={expanded}
      onToggle={event => onExpanded?.(event.currentTarget.open)}>
      <summary>{missing.length > 0 ? `Show other requirements (${others.length})` : `Show requirements (${others.length})`}</summary>
      <ul aria-label="Other requirements">{others.map((item, index) => <RequirementItem key={`${item.kind}:${item.name}:${index}`} item={item} />)}</ul>
      {others.some(item => item.optional) && <p>An optional requirement never stops the action from running; without it, the action runs without that part and says so in its result.</p>}
      {others.some(item => item.state === "present") && <p>Present means the tool found it; it has not checked that it works, so a present credential may still be refused.</p>}
    </details>}
    <p>These states are reported by the tool. Configure credentials outside the playground, then Refresh.</p>
  </>;
}
export function ToolSetup({ tool, action, loadSession, directoryGrants = null, params = {} }: Props) {
  const readiness = readinessFor(action, params);
  const [expanded, setExpanded] = useState(false);
  const [session, setSession] = useState<ToolSession>();
  const [error, setError] = useState<string>();
  const [loading, setLoading] = useState(true);
  const [checked, setChecked] = useState("");
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    let active = true;
    setSession(undefined); setError(undefined); setLoading(true);
    void loadSession().then((value) => {
      if (value.tool.name !== tool.name || value.tool.version !== tool.version)
        throw new Error("The session belongs to a different tool. Refresh after reconnecting the tool.");
      if (active) { setSession(value); setChecked(new Date().toLocaleTimeString()); }
    }).catch((failure: unknown) => {
      if (active) setError(failure instanceof Error ? failure.message : "The tool could not report its session.");
    }).finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [loadSession, tool.name, tool.version, revision]);
  return <div className={styles.setup}>
    <section aria-label="Provider setup" aria-busy={loading}>
      <div className={styles.heading}>
        <h3>{action.title}</h3>
        <Button busy={loading} onClick={() => {
          setSession(undefined); setError(undefined); setLoading(true); setRevision(value => value + 1);
        }}>Refresh</Button>
      </div>
      <p>{action.cost.may_spend
        ? "This action may cost money. The tool applies its own spending limits; you can set a lower one, or check the cost, before Run."
        : "This action is declared free by the tool."}</p>
      <p><strong>{readinessLabel[readiness.state]}</strong></p>
      {readiness.state === "unavailable" && <p>This action is unavailable in the tool’s current environment. Check the requirements below.</p>}
      <Requirements requirements={readiness.requirements} conditional={action.readiness.requirements.some(item => !!item.for_choice)}
        expanded={expanded} onExpanded={setExpanded} />
    </section>
    <section aria-label="Session spending" aria-busy={loading}>
      <h3>Session spending</h3>
      {loading && <p role="status">Refreshing tool setup…</p>}
      {error && <><p role="alert">{error}</p><p>Current allowance unavailable. Refresh does not restart the tool or reset its budget.</p></>}
      {session && <>
        <dl className={styles.amounts}>
          <div><dt>Session limit</dt><dd>{session.budget_usd === null ? "Uncapped" : usd(session.budget_usd)}</dd></div>
          <div><dt>Remaining</dt><dd>{session.remaining_usd === null ? "No session limit" : usd(session.remaining_usd)}</dd></div>
          <div><dt>Reported spend</dt><dd>{usd(session.spent_usd)}</dd></div>
          <div><dt>Reserved for active runs</dt><dd>{usd(session.reserved_usd)}</dd></div>
        </dl>
        {session.unresolved_usd > 0 && <p role="status">Reported spend includes {usd(session.unresolved_usd)} whose final charge is unknown.</p>}
        <p>Checked at {checked}. This allowance belongs to the running tool, not your provider account balance.</p>
        <details><summary>Session identity</summary><dl className={styles.identity}>
          <dt>Session</dt><dd><code>{session.id}</code></dd>
          <dt>Started</dt><dd>{session.started}</dd>
        </dl></details>
      </>}
    </section>
    <DirectoryAccess grants={directoryGrants} effective={loading
      ? {state:"loading"}
      : session && !error ? {state:"reported", allowed_dirs:session.allowed_dirs}
        : {state:"unavailable"}} />
  </div>;
}

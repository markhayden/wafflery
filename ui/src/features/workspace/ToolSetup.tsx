import { DirectoryAccess } from "./DirectoryAccess";
import type { DirectoryGrants } from "../../api/directory-grants";
import { useEffect, useState } from "react";
import { Button } from "../../components/Button";
import type { Action, Readiness, Requirement } from "../actions/model";
import { usd, type ToolSession } from "../../api/tool-session";
import { requirementPurpose } from "./checks";
import styles from "./ToolSetup.module.css";

const readinessLabel: Record<Readiness["state"], string> = {
  ready: "Ready", needs_setup: "Setup may be needed", unavailable: "Unavailable",
};
const requirementState: Record<Requirement["state"], string> = {
  missing: "Missing", present: "Present · not validated", valid: "Validated", invalid: "Invalid", unknown: "Unknown",
};
const requirementKind: Record<Requirement["kind"], string> = {
  config: "Configuration", credential: "Credential", env: "Environment variable", model: "Model", file: "File", dependency: "Dependency",
};
type Props = {
  tool: { name: string; version: string };
  action: Action;
  loadSession: () => Promise<ToolSession>;
  directoryGrants?: DirectoryGrants | null;
};
export function ToolSetup({ tool, action, loadSession, directoryGrants = null }: Props) {
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
      <p><strong>{readinessLabel[action.readiness.state]}</strong></p>
      {action.readiness.state === "unavailable"
        ? <p>This action is unavailable in the tool’s current environment. Check the requirements below.</p>
        : <p>Requirements describe the current configuration. Optional inputs can change what a call needs.</p>}
      <h4>Setup requirements</h4>
      {action.readiness.requirements.length > 0 ? <>
        <ul>{action.readiness.requirements.map((item, index) => <li key={`${item.kind}:${item.name}:${index}`}>
          <p><code>{item.name}</code> · <span>{requirementState[item.state]}</span></p>
          <p>{requirementKind[item.kind]}</p>
          {item.optional && <p className={styles.optional}>{requirementPurpose(item)}</p>}
          {item.setup && <p>{item.setup}</p>}
        </li>)}</ul>
        <p>These states are reported by the tool. A present credential has not necessarily been validated. Configure credentials outside the playground.</p>
        {action.readiness.requirements.some((item) => item.optional) && <p>An optional requirement never stops the action from running; without it, the action runs without that part and says so in its result.</p>}
      </> : <p>No setup requirements are declared for this action.</p>}

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

import { useEffect, useState } from "react";
import { Button } from "../../components/Button";
import type { Action } from "../actions/model";
import { usd, type ToolSession } from "../../api/tool-session";
import styles from "./ToolSetup.module.css";

type Props = {
  tool: { name: string; version: string };
  action: Action;
  loadSession: () => Promise<ToolSession>;
};
export function ToolSetup({ tool, action, loadSession }: Props) {
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
    <section aria-label="Provider setup">
      <h3>{action.title}</h3>
      <p>{action.cost.may_spend
        ? "Paid execution needs provider setup, a current estimate and an enforced run limit. It is not enabled yet."
        : "This action is declared free by the tool."}</p>
      <h4>Credential requirements</h4>
      {action.cost.needs_env.length > 0 ? <>
        <ul>{action.cost.needs_env.map((name) => <li key={name}><code>{name}</code></li>)}</ul>
        <p>These are the variable names declared by the tool. Their availability and validity have not been checked. Connecting credentials is not available here yet.</p>
      </> : <p>No credential variables are declared for this action. This does not verify its other setup requirements.</p>}
    </section>
    <section aria-label="Session spending" aria-busy={loading}>
      <div className={styles.heading}>
        <h3>Session spending</h3>
        <Button busy={loading} onClick={() => {
          setSession(undefined); setError(undefined); setLoading(true); setRevision(value => value + 1);
        }}>Refresh</Button>
      </div>
      {loading && <p role="status">Reading the tool’s session…</p>}
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
  </div>;
}

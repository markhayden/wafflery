import { useState } from "react";
import { Button } from "../../components/Button";
import styles from "./ToolSetup.module.css";

type Props = {
  busy: boolean;
  onRestart: () => Promise<string>;
};
/** Stays mounted when restart temporarily removes the tool's declaration. */
export function RestartTool({ busy, onRestart }: Props) {
  const [interacted, setInteracted] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string>();
  const [session, setSession] = useState<string>();
  async function restart() {
    setConfirming(false); setPending(true); setError(undefined); setSession(undefined);
    try { setSession(await onRestart()); }
    catch (failure) { setError(failure instanceof Error ? failure.message : "Restart could not be confirmed. Check the current session before retrying."); }
    finally { setPending(false); }
  }
  return <div className={styles.setup}>
    <section aria-label="Restart tool" className={styles.restart}>
      <h3>Restart tool session</h3>
      <p>Use this after changing the tool’s configuration. To pick up environment changes, quit and relaunch Wafflery from the updated terminal.</p>
      {busy && !pending && <p role="status">Wait for active or unconfirmed work to stop before restarting.</p>}
      {error && <p role="alert">{error}</p>}
      {session && <p role="status">Tool restarted. New session: <code>{session}</code></p>}
      {pending ? <p role="status">Restarting the tool and checking its setup…</p> : confirming ? <>
        <p>This stops the current tool and starts a new session with a new spending ledger. Earlier runs and unknown charges remain in history.</p>
        <div className={styles.actions}>
          <Button autoFocus onClick={() => setConfirming(false)}>Keep current session</Button>
          <Button disabled={busy} onClick={() => void restart()}>Confirm restart</Button>
        </div>
      </> : <Button autoFocus={interacted} disabled={busy} onClick={() => { setInteracted(true); setConfirming(true); }}>Restart tool session</Button>}
    </section>
  </div>;
}

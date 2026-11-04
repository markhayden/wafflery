import { Button } from "../../components/Button";
import styles from "./RunProgress.module.css";

/** A run that has not finished: what is running, a bar, the tool's latest line, and Cancel.
 * The bar is indeterminate: the tool reports steps as they finish but not how many there are. */
export function RunProgress({ action, message, cancelling, onCancel }: { action: string; message?: string | undefined; cancelling: boolean; onCancel?: (() => void) | undefined }) {
  return <section className={styles.progress} aria-label="Run progress">
    <div className={styles.heading}>
      <p className={styles.title}>{cancelling ? `Cancelling ${action}…` : `Running ${action}…`}</p>
      {onCancel && <Button onClick={onCancel}>Cancel run</Button>}
    </div>
    <div className={styles.track} role="progressbar" aria-label={`${action} progress`} aria-busy="true"><span className={styles.bar} /></div>
    <p className={styles.line} aria-live="polite">{message ? sentence(message) : "Starting…"}</p>
  </section>;
}

/** A tool's progress line reads as a sentence: "run started; 3 s" → "Run started · 3 s". */
function sentence(message: string): string {
  const text = message.replace(/;\s*/g, " · ").trim();
  return text.charAt(0).toUpperCase() + text.slice(1);
}

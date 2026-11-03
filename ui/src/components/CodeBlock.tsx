import { useState } from "react";
import styles from "./CodeBlock.module.css";

/** JSON (or any code) in a monospace box: lines never wrap, long lines scroll sideways, tall
 * content scrolls inside the box, and Copy puts the exact text on the clipboard. */
export function CodeBlock({ value, label }: { value: unknown; label: string }) {
  const text = typeof value === "string" ? value : JSON.stringify(value, null, 2);
  const [copied, setCopied] = useState(false);
  return <div className={styles.block}>
    <button type="button" className={styles.copy} aria-label={`Copy ${label}`} onClick={() => {
      void navigator.clipboard?.writeText(text).then(() => { setCopied(true); setTimeout(() => setCopied(false), 1500); });
    }}>{copied ? "Copied" : "Copy"}</button>
    <pre className={styles.code} aria-label={label} tabIndex={0}><code>{text}</code></pre>
  </div>;
}

import { useMemo, useState } from "react";
import { highlight, type Language } from "./highlight";
import styles from "./CodeBlock.module.css";

/** JSON (or a shell command, or plain text) in a monospace box, coloured by kind: lines never
 * wrap, long lines scroll sideways, tall content scrolls inside the box, and Copy puts the exact
 * text on the clipboard. A value that is not a string is shown as JSON. */
/** `fill` shows the whole text at its own height, for a place that already scrolls (a drawer). */
export function CodeBlock({ value, label, language, copyLabel, fill = false }: { value: unknown; label: string; language?: Language; copyLabel?: string; fill?: boolean }) {
  const text = typeof value === "string" ? value : JSON.stringify(value, null, 2) ?? "";
  const kind: Language = language ?? (typeof value === "string" ? "text" : "json");
  const tokens = useMemo(() => highlight(text, kind), [text, kind]);
  const [copied, setCopied] = useState(false);
  return <div className={styles.block}>
    <button type="button" className={styles.copy} aria-label={copyLabel ?? `Copy ${label}`} onClick={() => {
      void navigator.clipboard?.writeText(text).then(() => { setCopied(true); setTimeout(() => setCopied(false), 1500); });
    }}>{copied ? "Copied" : "Copy"}</button>
    <pre className={fill ? `${styles.code} ${styles.fill}` : styles.code} aria-label={label} tabIndex={0}><code>{tokens.map((token, index) =>
      token.kind ? <span key={index} className={styles[token.kind]}>{token.text}</span> : token.text)}</code></pre>
  </div>;
}

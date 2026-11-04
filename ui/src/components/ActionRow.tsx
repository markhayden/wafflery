import type { ComponentPropsWithoutRef } from "react";
import styles from "./ActionRow.module.css";

/** A row of buttons under a field or at the foot of a panel: right-aligned, wrapping, with a
 * fixed gap and space above so it never sits against the control before it. `flush` drops the
 * space above, for a row that shares a line with other content (a panel footer). */
export function ActionRow({ flush = false, className, ...props }: ComponentPropsWithoutRef<"div"> & { flush?: boolean }) {
  return <div {...props} className={[styles.row, flush && styles.flush, className].filter(Boolean).join(" ")} />;
}

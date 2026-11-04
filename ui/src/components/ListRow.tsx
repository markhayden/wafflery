import { useId, useState, type ReactNode } from "react";
import styles from "./ListRow.module.css";

const chevron = (direction: "right" | "down") => <svg className={styles.chevron} data-direction={direction} width="16" height="16" viewBox="0 0 24 24"
  fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><path d={direction === "right" ? "m9 6 6 6-6 6" : "m6 9 6 6 6-6"} /></svg>;

/** A placeholder mark for the icon tile until actions declare their own. */
export const placeholderIcon = <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
  <path d="M4 8V5a1 1 0 0 1 1-1h3M16 4h3a1 1 0 0 1 1 1v3M20 16v3a1 1 0 0 1-1 1h-3M8 20H5a1 1 0 0 1-1-1v-3M8 12h8" /></svg>;

/** Rows in one quiet panel, divided by lines: a list of things to open. */
export function RowList({ label, children }: { label: string; children: ReactNode }) {
  return <ul className={styles.list} aria-label={label}>{children}</ul>;
}

/** One thing to open, as a row: an icon tile, its name, one line on what it is, quiet facts under
 * that (`meta`), and something at the right such as its state. Its accessible name is the title.
 * With `details`, a chevron beside it shows more without leaving the list. */
export function ListRow({ title, description, meta = [], trailing, icon = placeholderIcon, onOpen, details, disabled = false }: {
  title: string; description?: ReactNode; meta?: ReactNode[]; trailing?: ReactNode; icon?: ReactNode;
  onOpen: () => void; details?: (() => ReactNode) | undefined; disabled?: boolean;
}) {
  const id = useId();
  const [open, setOpen] = useState(false);
  return <li className={styles.row} data-open={open || undefined}>
    <div className={styles.head}>
      <button type="button" className={styles.main} aria-label={title} aria-describedby={description ? `${id}-about` : undefined}
        disabled={disabled} onClick={onOpen}>
        <span className={styles.icon}>{icon}</span>
        <span className={styles.body}>
          <span className={styles.top}><span className={styles.title}>{title}</span>{trailing && <span className={styles.trailing}>{trailing}</span>}</span>
          {description && <span className={styles.description} id={`${id}-about`}>{description}</span>}
          {meta.length > 0 && <span className={styles.meta}>{meta.map((item, index) => <span key={index}>{item}</span>)}</span>}
        </span>
        {!details && chevron("right")}
      </button>
      {details && <button type="button" className={styles.toggle} aria-expanded={open} aria-controls={`${id}-details`}
        aria-label={`${open ? "Hide" : "Show"} details for ${title}`} onClick={() => setOpen(!open)}>{chevron("down")}</button>}
    </div>
    {details && open && <div className={styles.details} id={`${id}-details`}>{details()}</div>}
  </li>;
}

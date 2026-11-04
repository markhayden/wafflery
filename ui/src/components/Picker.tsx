import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import styles from "./Picker.module.css";
export type Choice = {
  id: string;
  title: string;
  description?: string;
  icon?: string;
  meta?: string;
  group?: string;
  disabled?: boolean;
};
type Props = {
  label: string;
  value: string;
  choices: Choice[];
  onChange: (value: string) => void;
  disabled?: boolean;
  compact?: boolean;
  /** The application header's picker: one line, bordered, sized to sit level with the brand. */
  header?: boolean;
  wide?: boolean;
  summary?: string | undefined;
  /** A short type tag beside the selected title (an operation's Image → Location). */
  tag?: string | undefined;
  /** Keep the summary to one line, the full text on hover; for a summary that is a sentence. */
  oneLine?: boolean;
  /** Placed after the label, such as an InfoTip about the selected choice. */
  labelExtra?: ReactNode;
};
export function Picker({ label, value, choices, onChange, disabled, compact = false, header = false, wide = false, summary, tag, oneLine = false, labelExtra }: Props) {
  const id = useId();
  const trigger = useRef<HTMLButtonElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const selected = choices.find((choice) => choice.id === value);
  const matches = choices.filter((choice) =>
    `${choice.title} ${choice.description ?? ""} ${choice.meta ?? ""} ${choice.group ?? ""}`
      .toLowerCase()
      .includes(query.toLowerCase()),
  );
  // A dropdown in the top layer (popover): outside clicks and Escape close it, and the trigger
  // toggles it. It sits under its trigger, at least as wide, and flips above when below is short.
  useEffect(() => {
    const panel = dialog.current;
    if (!panel) return;
    const place = () => {
      const anchor = trigger.current?.getBoundingClientRect();
      if (!anchor) return;
      const gap = 6, edge = 16;
      const rem = Number.parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
      const width = Math.min(wide ? 44 * rem : Math.max(anchor.width, 24 * rem), window.innerWidth - 2 * edge);
      const left = Math.min(Math.max(anchor.left, edge), window.innerWidth - edge - width);
      const below = window.innerHeight - anchor.bottom - gap - edge;
      const above = anchor.top - gap - edge;
      const up = below < 320 && above > below;
      panel.style.width = `${width}px`;
      panel.style.left = `${left}px`;
      panel.style.maxHeight = `${Math.min(up ? above : below, 42 * rem)}px`;
      panel.style.top = up ? "auto" : `${anchor.bottom + gap}px`;
      panel.style.bottom = up ? `${window.innerHeight - anchor.top + gap}px` : "auto";
    };
    const toggled = (event: Event) => {
      const opening = (event as ToggleEvent).newState === "open";
      setOpen(opening);
      if (opening) {
        place();
        search.current?.focus();
      } else if (!document.activeElement || document.activeElement === document.body || panel.contains(document.activeElement)) {
        trigger.current?.focus();
      }
    };
    const follow = () => { if (panel.matches(":popover-open")) place(); };
    panel.addEventListener("toggle", toggled);
    window.addEventListener("resize", follow);
    window.addEventListener("scroll", follow, true);
    return () => {
      panel.removeEventListener("toggle", toggled);
      window.removeEventListener("resize", follow);
      window.removeEventListener("scroll", follow, true);
    };
  }, [wide]);
  const close = () => dialog.current?.hidePopover();
  return (
    <div className={`${styles.picker} ${compact ? styles.compact : ""} ${header ? styles.header : ""}`}>
      <span className={styles.labelRow}>
        <span id={id} className={styles.label}>
          {label}
        </span>
        {labelExtra}
      </span>
      <button
        ref={trigger}
        type="button"
        className={styles.trigger}
        aria-labelledby={`${id} ${id}-value`}
        aria-haspopup="dialog"
        aria-expanded={open}
        disabled={disabled}
        popoverTarget={`${id}-choices`}
        onClick={() => setQuery("")}
      >
        <span>
          <span className={styles.titleRow}>
            <strong id={`${id}-value`}>
              {selected?.title ?? `Choose ${label.toLowerCase()}`}
            </strong>
            {tag && <span className={styles.tag}>{tag}</span>}
          </span>
          {(summary??selected?.description) && <small className={oneLine ? styles.oneLine : undefined} title={oneLine ? summary??selected?.description : undefined}>{summary??selected?.description}</small>}
        </span>
        <svg className={styles.chevron} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg>
      </button>
      <dialog
        ref={dialog}
        id={`${id}-choices`}
        popover="auto"
        className={styles.dialog}
        aria-label={`Choose ${label.toLowerCase()}`}
      >
        <input
          ref={search}
          aria-label={`Search ${label.toLowerCase()}`}
          placeholder="Search…"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        <div className={styles.options}>
          {open && sections(matches).map((section, index) => (
            // A group's heading sticks to the top while its options scroll under it, until the
            // next group's heading pushes it up.
            <div key={`${section.group ?? ""}-${index}`} className={styles.section}>
              {section.group && <p className={styles.group}><span>{section.group}</span></p>}
              {section.choices.map((choice) => (
                <button
                  key={choice.id}
                  type="button"
                  className={choice.meta ? styles.withMeta : undefined}
                  aria-pressed={choice.id === value}
                  disabled={choice.disabled}
                  onClick={() => {
                    onChange(choice.id);
                    close();
                  }}
                >
                  {choice.meta && <span className={styles.mark} aria-hidden="true">{choice.icon && <img src={choice.icon} width="64" height="64" alt="" />}</span>}
                  <span className={styles.optionText}>
                    <span className={styles.optionHeading}><strong>{choice.title}</strong>
                      {choice.meta && <small className={styles.meta} title={choice.meta}>{choice.meta}</small>}
                    </span>
                    {choice.description && <small>{choice.description}</small>}
                  </span>
                </button>
              ))}
            </div>
          ))}
          {open && matches.length === 0 && <p>No matches. Try another search.</p>}
        </div>
      </dialog>
    </div>
  );
}

/** Consecutive choices of one group, in order; ungrouped runs have no group. */
function sections(choices: Choice[]): { group?: string; choices: Choice[] }[] {
  const out: { group?: string; choices: Choice[] }[] = [];
  for (const choice of choices) {
    const last = out[out.length - 1];
    if (last && last.group === choice.group) last.choices.push(choice);
    else out.push({ ...(choice.group ? { group: choice.group } : {}), choices: [choice] });
  }
  return out;
}

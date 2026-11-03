import { useEffect, useId, useRef, useState } from "react";
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
  wide?: boolean;
  summary?: string | undefined;
};
export function Picker({ label, value, choices, onChange, disabled, compact = false, wide = false, summary }: Props) {
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
    <div className={`${styles.picker} ${compact ? styles.compact : ""}`}>
      <span id={id} className={styles.label}>
        {label}
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
          <strong id={`${id}-value`}>
            {selected?.title ?? `Choose ${label.toLowerCase()}`}
          </strong>
          {(summary??selected?.description) && <small>{summary??selected?.description}</small>}
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
          {open && matches.map((choice, index) => (
            <div key={choice.id}>
              {choice.group && choice.group !== matches[index - 1]?.group && (
                <p className={styles.group}>{choice.group}</p>
              )}
              <button
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
            </div>
          ))}
          {open && matches.length === 0 && <p>No matches. Try another search.</p>}
        </div>
      </dialog>
    </div>
  );
}

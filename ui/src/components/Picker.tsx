import { useId, useRef, useState } from "react";
import { Button } from "./Button";
import styles from "./Picker.module.css";
export type Choice = {
  id: string;
  title: string;
  description?: string;
  group?: string;
  disabled?: boolean;
};
type Props = {
  label: string;
  value: string;
  choices: Choice[];
  onChange: (value: string) => void;
  disabled?: boolean;
};
export function Picker({ label, value, choices, onChange, disabled }: Props) {
  const id = useId();
  const trigger = useRef<HTMLButtonElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const [query, setQuery] = useState("");
  const selected = choices.find((choice) => choice.id === value);
  const matches = choices.filter((choice) =>
    `${choice.title} ${choice.description ?? ""} ${choice.group ?? ""}`
      .toLowerCase()
      .includes(query.toLowerCase()),
  );
  return (
    <div className={styles.picker}>
      <span id={id} className={styles.label}>
        {label}
      </span>
      <button
        ref={trigger}
        type="button"
        className={styles.trigger}
        aria-labelledby={`${id} ${id}-value`}
        aria-haspopup="dialog"
        disabled={disabled}
        onClick={() => {
          setQuery("");
          dialog.current?.showModal();
          search.current?.focus();
        }}
      >
        <span>
          <strong id={`${id}-value`}>
            {selected?.title ?? `Choose ${label.toLowerCase()}`}
          </strong>
          {selected?.description && <small>{selected.description}</small>}
        </span>
        <span aria-hidden="true">⌄</span>
      </button>
      <dialog
        ref={dialog}
        onClose={() => trigger.current?.focus()}
        className={styles.dialog}
        aria-label={`Choose ${label.toLowerCase()}`}
      >
        <div className={styles.head}>
          <h2>Choose {label.toLowerCase()}</h2>
          <Button onClick={() => dialog.current?.close()}>Close</Button>
        </div>
        <input
          ref={search}
          aria-label={`Search ${label.toLowerCase()}`}
          placeholder="Search…"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        <div className={styles.options}>
          {matches.map((choice, index) => (
            <div key={choice.id}>
              {choice.group && choice.group !== matches[index - 1]?.group && (
                <p className={styles.group}>{choice.group}</p>
              )}
              <button
                type="button"
                aria-pressed={choice.id === value}
                disabled={choice.disabled}
                onClick={() => {
                  onChange(choice.id);
                  dialog.current?.close();
                }}
              >
                <strong>{choice.title}</strong>
                {choice.description && <small>{choice.description}</small>}
              </button>
            </div>
          ))}
          {matches.length === 0 && <p>No matches. Try another search.</p>}
        </div>
      </dialog>
    </div>
  );
}

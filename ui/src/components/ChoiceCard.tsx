import type { ComponentPropsWithoutRef, ReactNode } from "react";
import { useId } from "react";
import styles from "./ChoiceCard.module.css";

/** One of several things to pick from, as a card: a title, a line on what it is, and a selected
 * state. Its accessible name is the title alone; the line is its description. Lay several out
 * with ChoiceCards. */
export function ChoiceCard({ title, description, selected = false, className, ...props }:
  Omit<ComponentPropsWithoutRef<"button">, "title" | "children"> & { title: string; description?: ReactNode; selected?: boolean }) {
  const id = useId();
  return <button type="button" {...props} aria-pressed={selected} aria-describedby={description ? `${id}-about` : undefined}
    className={[styles.card, className].filter(Boolean).join(" ")}>
    <span className={styles.title}>{title}</span>
    {description && <span className={styles.description} id={`${id}-about`} aria-hidden="true">{description}</span>}
  </button>;
}

/** Choice cards laid out together. `grid`: as many columns as fit at `min` wide, for choices
 * worth a card each. `list`: one row per choice, title in a fixed column and its line beside it,
 * the same shape at any width; for picking one of a few records. */
export function ChoiceCards({ label, layout = "grid", min = "13rem", children }: { label: string; layout?: "grid" | "list"; min?: string; children: ReactNode }) {
  return layout === "list"
    ? <div role="group" aria-label={label} className={styles.list}>{children}</div>
    : <div role="group" aria-label={label} className={styles.grid} style={{ gridTemplateColumns: `repeat(auto-fill, minmax(min(${min}, 100%), 1fr))` }}>{children}</div>;
}

import styles from "./FieldTag.module.css";

/** The small pill beside a field's label: whether a person must fill it in. Every input marks
 * itself, so "Optional" never hides a setting behind a fold. */
export function FieldTag({ required, extra }: { required: boolean; extra?: string | undefined }) {
  return <span className={styles.tag}>{required ? "Required" : "Optional"}{extra ? ` · ${extra}` : ""}</span>;
}

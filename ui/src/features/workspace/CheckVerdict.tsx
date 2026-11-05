import type { Asset } from "../../api/assets";
import type { SavedImage } from "../../api/workspace";
import { PlaceMarks } from "../../components/PlaceMarks";
import type { Json } from "../actions/model";
import { humanize } from "../actions/help";
import { checkView, imageName } from "./checks";
import styles from "./CheckVerdict.module.css";

/** What the tool's check said about a run, near its result: passed, still sees it (with the
 * places drawn on the result), or why the result was not checked. Nothing for a run the tool
 * does not report checking. */
export function CheckVerdict({ result, outputs, load }: {
  result: Json | undefined;
  outputs: SavedImage[];
  load: (asset: Asset) => Promise<Blob>;
}) {
  const view = checkView(result, outputs);
  if (!view) return null;
  return (
    <section className={styles.check} data-kind={view.kind} aria-label="Check">
      <p className={styles.title}>
        <span aria-hidden="true" className={styles.mark}>{view.kind === "pass" ? "✓" : view.kind === "attention" ? "!" : "–"}</span>
        {view.title}
      </p>
      {view.kind === "attention" && <p>{view.detail}</p>}
      {view.kind !== "unchecked" && view.notes.length > 0 && <ul className={styles.notes}>
        {view.notes.map((note, index) => <li key={index}>{note}</li>)}</ul>}
      {view.kind === "attention" && view.image && view.places.length > 0 && (
        <PlaceMarks asset={view.image.asset} image={view.image.image} places={view.places} load={load}
          label={`Where the check still sees it · ${imageName(view.image, humanize)}`} />
      )}
    </section>
  );
}

import type { Asset } from "../api/assets";
import type { Json } from "../features/actions/model";
import type { Coordinates } from "./comparison";
import { useImage } from "./ImageCompare";
import styles from "./PlaceMarks.module.css";

/** A place on an image, as a location's kind and parts (table-manners 9): fractions of the
 * image's width and height from its top-left corner. */
export type Place = { kind: string; parts: Json[]; label?: string };
type Point = [number, number];

const point = (value: Json): Point | undefined =>
  Array.isArray(value) && typeof value[0] === "number" && typeof value[1] === "number" ? [value[0], value[1]] : undefined;
const points = (value: Json | undefined): Point[] => (Array.isArray(value) ? value.flatMap((item): Point[] => { const p = point(item); return p ? [p] : []; }) : []);

/** The outline of one part, in fractions, or nothing for a part with no shape to draw (a mask,
 * an absent thing). */
export function outline(kind: string, part: Json): Point[] | undefined {
  const shape = part && typeof part === "object" && !Array.isArray(part) ? part : {};
  if (kind === "bbox" && Array.isArray(shape.bbox) && shape.bbox.length === 4 && shape.bbox.every((n) => typeof n === "number")) {
    const [x0, y0, x1, y1] = shape.bbox as number[];
    return [[x0!, y0!], [x1!, y0!], [x1!, y1!], [x0!, y1!]];
  }
  if (kind === "polygon") return points(shape.polygon).length >= 3 ? points(shape.polygon) : undefined;
  if ((kind === "line" || kind === "stroke") && points(shape.path).length >= 2) return points(shape.path);
  if (kind === "line" && typeof shape.column === "number") return [[shape.column, 0], [shape.column, 1]];
  return undefined;
}

type Props = {
  asset: Asset;
  image: Coordinates;
  places: Place[];
  load: (asset: Asset) => Promise<Blob>;
  /** What the picture is, for a screen reader and the caption. */
  label: string;
};

/** An image with places drawn on it, each numbered to match a list below it. */
export function PlaceMarks({ asset, image, places, load, label }: Props) {
  const bytes = useImage(asset, load);
  const { width, height } = image;
  const shapes = places.map((place) => ({
    place,
    whole: place.kind === "whole_image",
    lines: place.kind === "line" || place.kind === "stroke",
    parts: place.parts.flatMap((part) => { const shape = outline(place.kind, part); return shape ? [shape] : []; }),
  }));
  return (
    <figure className={styles.marks}>
      <div className={styles.frame} style={{ aspectRatio: `${width}/${height}` }}>
        {bytes.url ? <img src={bytes.url} alt={label} draggable={false} />
          : <p role="status">{bytes.error ?? "Loading image…"}</p>}
        <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" aria-hidden="true">
          {shapes.map(({ whole, lines, parts }, index) => whole
            ? <rect key={index} x={0} y={0} width={width} height={height} />
            : parts.map((shape, part) => {
              const drawn = shape.map(([x, y]) => `${x * width},${y * height}`).join(" ");
              return lines ? <polyline key={`${index}/${part}`} points={drawn} /> : <polygon key={`${index}/${part}`} points={drawn} />;
            }))}
        </svg>
        {shapes.map(({ whole, parts }, index) => {
          const anchor = whole ? [0, 0] : parts[0]?.reduce((a, b) => [Math.min(a[0], b[0]), Math.min(a[1], b[1])]);
          return anchor ? <span key={index} className={styles.badge} aria-hidden="true"
            style={{ left: `${anchor[0]! * 100}%`, top: `${anchor[1]! * 100}%` }}>{index + 1}</span> : null;
        })}
      </div>
      <figcaption>
        <span>{label}</span>
        {places.some((place) => place.label) && <ol>{places.map((place, index) =>
          <li key={index}>{place.label ?? "A place without a description"}</li>)}</ol>}
      </figcaption>
    </figure>
  );
}

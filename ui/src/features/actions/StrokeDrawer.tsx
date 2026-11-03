import { useEffect, useRef, useState } from "react";
import { Button } from "../../components/Button";
import type { Asset } from "../../api/assets";
import type { SelectedImage } from "./selection";
import type { Json } from "./model";
import styles from "./RectangleDrawer.module.css";

/** One painted stroke, in pixels of the original image. */
export type Stroke = { path: [number, number][]; radius: number };

/** At most 256 points a part (table-manners 9.2): thin a long path evenly, keeping both ends. */
export function thinPath(path: [number, number][], limit = 256): [number, number][] {
  if (path.length <= limit) return path;
  const step = (path.length - 1) / (limit - 1);
  return Array.from({ length: limit }, (_, index) => path[Math.round(index * step)]!);
}

/** The contract's `stroke` location: each stroke a part with its path (fractions of the original
 * image, a quarter pixel of precision) and its brush radius in pixels. */
export function strokeLocation(image: SelectedImage, strokes: Stroke[]): Json {
  const decimals = Math.ceil(Math.log10(4 * Math.max(image.width, image.height)));
  const at = (value: number, size: number) => Number((Math.min(Math.max(value, 0), size) / size).toFixed(decimals));
  const frame = image.coordinates ?? { projection: "flat", topology: "bounded" };
  return {
    image: { file_sha256: image.sha256, width: image.width, height: image.height, projection: frame.projection, topology: frame.topology },
    kind: "stroke",
    parts: strokes.map((stroke) => ({
      path: thinPath(stroke.path).map(([x, y]) => [at(x, image.width), at(y, image.height)]),
      radius_px: Math.max(1, Math.round(stroke.radius)),
    })),
    straddles_wrap: false,
  };
}

type Props = { image: SelectedImage; preview: (asset: Asset) => Promise<Blob>; onUse: (location: Json) => void; disabled?: boolean };
export function StrokeDrawer({ image, preview, onUse, disabled = false }: Props) {
  const longest = Math.max(image.width, image.height);
  const [url, setUrl] = useState<string>();
  const [radius, setRadius] = useState(() => Math.max(2, Math.round(longest / 250)));
  const [strokes, setStrokes] = useState<Stroke[]>([]);
  const drawing = useRef<Stroke>(undefined);
  const surface = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let alive = true, made: string | undefined;
    void preview(image).then((blob) => { if (alive) { made = URL.createObjectURL(blob); setUrl(made); } });
    return () => { alive = false; if (made) URL.revokeObjectURL(made); };
  }, [image, preview]);
  const point = (event: React.PointerEvent): [number, number] => {
    const rect = surface.current!.getBoundingClientRect();
    return [
      Math.min(Math.max(0, (event.clientX - rect.left) / rect.width), 1) * image.width,
      Math.min(Math.max(0, (event.clientY - rect.top) / rect.height), 1) * image.height,
    ];
  };
  // A new point only once the brush has moved a little, so a slow drag stays a short path.
  const extend = (next: [number, number]) => {
    const current = drawing.current;
    if (!current) return;
    const last = current.path.at(-1)!;
    if (Math.hypot(next[0] - last[0], next[1] - last[1]) < Math.max(1, radius / 2)) return;
    current.path.push(next);
    setStrokes((values) => [...values.slice(0, -1), { ...current, path: [...current.path] }]);
  };
  const scale = 100 / image.width; // SVG units: percent of the width.
  return <div className={styles.drawer}>
    <p className={styles.hint}>Paint along the damage: press and drag over it. Each drag adds a stroke; make the brush just wide enough to cover it.</p>
    <div
      ref={surface}
      className={styles.surface}
      style={{ aspectRatio: `${image.width} / ${image.height}` }}
      aria-hidden="true"
      onPointerDown={(event) => {
        if (disabled) return;
        event.currentTarget.setPointerCapture(event.pointerId);
        const start = point(event);
        drawing.current = { path: [start, start], radius };
        setStrokes((values) => [...values, drawing.current!]);
      }}
      onPointerMove={(event) => extend(point(event))}
      onPointerUp={() => { drawing.current = undefined; }}
    >
      {url && <img src={url} alt="" draggable={false} />}
      <svg className={styles.paint} viewBox={`0 0 100 ${(image.height / image.width) * 100}`} preserveAspectRatio="none">
        {strokes.map((stroke, index) => <polyline key={index}
          points={stroke.path.map(([x, y]) => `${x * scale},${y * scale}`).join(" ")}
          strokeWidth={stroke.radius * 2 * scale} />)}
      </svg>
    </div>
    <label className={styles.brush}>
      <span>Brush size · {radius * 2} px wide</span>
      <input type="range" min={1} max={Math.max(8, Math.round(longest / 20))} value={radius} disabled={disabled}
        onChange={(event) => setRadius(Number(event.target.value))} />
    </label>
    <div className={styles.actions}>
      <Button type="button" disabled={disabled || !strokes.length} onClick={() => { onUse(strokeLocation(image, strokes)); setStrokes([]); }}>
        {strokes.length > 1 ? `Use these ${strokes.length} strokes` : "Use this stroke"}
      </Button>
      {strokes.length > 0 && <Button type="button" disabled={disabled} onClick={() => setStrokes((values) => values.slice(0, -1))}>Undo last</Button>}
      {strokes.length > 1 && <Button type="button" disabled={disabled} onClick={() => setStrokes([])}>Clear</Button>}
    </div>
  </div>;
}

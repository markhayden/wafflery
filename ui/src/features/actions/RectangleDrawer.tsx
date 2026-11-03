import { useEffect, useRef, useState } from "react";
import { Button } from "../../components/Button";
import type { Asset } from "../../api/assets";
import type { SelectedImage } from "./selection";
import type { Json } from "./model";
import styles from "./RectangleDrawer.module.css";

/** A box in pixels of the original image. */
export type Box = { left: number; top: number; width: number; height: number };

/** The contract's `bbox` location (table-manners 9): fractions of the original image, edges not
 * pixel centres, enough decimals for a quarter pixel at the longest side. */
export function bboxLocation(image: SelectedImage, box: Box): Json {
  const decimals = Math.ceil(Math.log10(4 * Math.max(image.width, image.height)));
  const fraction = (value: number, size: number) => Number((value / size).toFixed(decimals));
  const frame = image.coordinates ?? { projection: "flat", topology: "bounded" };
  return {
    image: { file_sha256: image.sha256, width: image.width, height: image.height, projection: frame.projection, topology: frame.topology },
    kind: "bbox",
    parts: [{ bbox: [fraction(box.left, image.width), fraction(box.top, image.height),
      fraction(box.left + box.width, image.width), fraction(box.top + box.height, image.height)] }],
    straddles_wrap: false,
  };
}

/** Keeps a box inside the image, at least one pixel each way. */
export function clampBox(box: Box, width: number, height: number): Box {
  const left = Math.min(Math.max(0, Math.round(box.left)), width - 1);
  const top = Math.min(Math.max(0, Math.round(box.top)), height - 1);
  return {
    left, top,
    width: Math.min(Math.max(1, Math.round(box.width)), width - left),
    height: Math.min(Math.max(1, Math.round(box.height)), height - top),
  };
}

type Props = { image: SelectedImage; preview: (asset: Asset) => Promise<Blob>; onUse: (location: Json) => void; disabled?: boolean };
export function RectangleDrawer({ image, preview, onUse, disabled = false }: Props) {
  const [url, setUrl] = useState<string>();
  const [box, setBox] = useState<Box>();
  const start = useRef<{ x: number; y: number }>(undefined);
  const surface = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let alive = true, made: string | undefined;
    void preview(image).then((blob) => { if (alive) { made = URL.createObjectURL(blob); setUrl(made); } });
    return () => { alive = false; if (made) URL.revokeObjectURL(made); };
  }, [image, preview]);
  // Pointer position as pixels of the original image (the preview may be scaled).
  const point = (event: React.PointerEvent) => {
    const rect = surface.current!.getBoundingClientRect();
    return {
      x: Math.min(Math.max(0, (event.clientX - rect.left) / rect.width), 1) * image.width,
      y: Math.min(Math.max(0, (event.clientY - rect.top) / rect.height), 1) * image.height,
    };
  };
  const drag = (from: { x: number; y: number }, to: { x: number; y: number }) =>
    setBox(clampBox({ left: Math.min(from.x, to.x), top: Math.min(from.y, to.y), width: Math.abs(to.x - from.x), height: Math.abs(to.y - from.y) }, image.width, image.height));
  const edit = (key: keyof Box, value: string) => {
    const number = Number(value);
    if (!Number.isFinite(number)) return;
    setBox(clampBox({ ...(box ?? { left: 0, top: 0, width: image.width, height: image.height }), [key]: number }, image.width, image.height));
  };
  return <div className={styles.drawer}>
    <p className={styles.hint}>Press on the image and drag a rectangle over the area.</p>
    <div
      ref={surface}
      className={styles.surface}
      style={{ aspectRatio: `${image.width} / ${image.height}` }}
      aria-hidden="true"
      onPointerDown={(event) => { if (disabled) return; event.currentTarget.setPointerCapture(event.pointerId); start.current = point(event); drag(start.current, start.current); }}
      onPointerMove={(event) => { if (start.current) drag(start.current, point(event)); }}
      onPointerUp={() => { start.current = undefined; }}
    >
      {url && <img src={url} alt="" draggable={false} />}
      {box && <div className={styles.box} style={{
        left: `${(box.left / image.width) * 100}%`, top: `${(box.top / image.height) * 100}%`,
        width: `${(box.width / image.width) * 100}%`, height: `${(box.height / image.height) * 100}%`,
      }} />}
    </div>
    <fieldset className={styles.exact} disabled={disabled}>
      <legend>Exact rectangle, in pixels of the {image.width} × {image.height} image</legend>
      {(["left", "top", "width", "height"] as const).map((key) =>
        <label key={key}>{key.charAt(0).toUpperCase() + key.slice(1)}
          <input type="number" min={key === "width" || key === "height" ? 1 : 0} step={1} value={box?.[key] ?? ""} onChange={(event) => edit(key, event.target.value)} />
        </label>)}
    </fieldset>
    <div className={styles.actions}>
      <Button type="button" disabled={disabled || !box} onClick={() => { if (box) { onUse(bboxLocation(image, box)); setBox(undefined); } }}>Use this rectangle</Button>
      {box && <Button type="button" disabled={disabled} onClick={() => setBox(undefined)}>Clear</Button>}
    </div>
  </div>;
}

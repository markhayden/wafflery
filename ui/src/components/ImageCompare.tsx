import { useEffect, useRef, useState, type ReactNode } from "react";
import type { Asset } from "../api/assets";
import { Button } from "./Button";
import { canWipe, type Coordinates } from "./comparison";
import styles from "./ImageCompare.module.css";
export type CompareImage = { asset: Asset; image: Coordinates; label: string };
type Props = {
  left?: CompareImage | undefined;
  right?: CompareImage | undefined;
  load: (asset: Asset) => Promise<Blob>;
  /** Extra controls in the toolbar, such as a toggle for what the right side shows. */
  controls?: ReactNode;
};
/** An asset's bytes as an object URL, read with `load` and revoked when the asset changes. */
export function useImage(asset: Asset | undefined, load: (asset: Asset) => Promise<Blob>) {
  // Tagged with the asset it was read for, so a new label never shows the previous image's bytes
  // in the render before the new read starts.
  const [state, setState] = useState<{ for?: Asset; url?: string; error?: string }>({});
  useEffect(() => {
    let active = true;
    let url: string | undefined;
    setState({});
    if (asset)
      void load(asset)
        .then((blob) => {
          if (active) {
            url = URL.createObjectURL(blob);
            setState({ for: asset, url });
          }
        })
        .catch(() => {
          if (active)
            setState({
              for: asset,
              error: "Image unavailable. The saved bytes could not be read.",
            });
        });
    return () => {
      active = false;
      if (url) URL.revokeObjectURL(url);
    };
  }, [asset, load]);
  return state.for === asset ? state : {};
}

/** Whether an image is a 360° panorama whose left and right edges meet. */
export function isPanorama(image: Coordinates | undefined): boolean {
  return image?.projection === "equirectangular";
}

/** `url` turned half a turn when `enabled`: the right half drawn first, so a panorama's seam,
 * where its left and right edges meet, sits in the middle of the picture where it can be judged.
 * Every mode (wipe, side by side, changes) and zoom work on the turned image unchanged. */
export function useCentredSeam(url: string | undefined, enabled: boolean) {
  const [state, setState] = useState<{ key?: string; url?: string }>({});
  const key = enabled && url ? url : undefined;
  useEffect(() => {
    if (!key) return;
    let active = true;
    let made: string | undefined;
    const image = new Image();
    image.onload = () => {
      if (!active) return;
      const width = image.naturalWidth, height = image.naturalHeight, half = Math.floor(width / 2);
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      const context = canvas.getContext("2d")!;
      context.drawImage(image, -half, 0);
      context.drawImage(image, width - half, 0);
      canvas.toBlob((blob) => {
        if (!active || !blob) return;
        made = URL.createObjectURL(blob);
        setState({ key, url: made });
      }, "image/png");
    };
    image.src = key;
    return () => {
      active = false;
      if (made) URL.revokeObjectURL(made);
    };
  }, [key]);
  if (!enabled) return url;
  return state.key === key ? state.url : undefined;
}

/** Zoom and pan shared by every view and both sides, so the same detail stays in view when the
 * mode changes. `x`, `y` are the content's offset in viewport pixels; the content never leaves a
 * gap at an edge. */
type View = { scale: number; x: number; y: number };
const FIT: View = { scale: 1, x: 0, y: 0 };
const MAX_SCALE = 16;
function clampView(view: View, width: number, height: number): View {
  const scale = Math.min(Math.max(view.scale, 1), MAX_SCALE);
  if (scale === 1) return FIT;
  return {
    scale,
    x: Math.min(0, Math.max(width - width * scale, view.x)),
    y: Math.min(0, Math.max(height - height * scale, view.y)),
  };
}
function zoomAt(view: View, factor: number, cx: number, cy: number, width: number, height: number): View {
  const scale = Math.min(Math.max(view.scale * factor, 1), MAX_SCALE);
  const ratio = scale / view.scale;
  return clampView({ scale, x: cx - (cx - view.x) * ratio, y: cy - (cy - view.y) * ratio }, width, height);
}

/** The after image dimmed, with every pixel that differs from before painted in the highlight. */
function useChanges(before: string | undefined, after: string | undefined, enabled: boolean) {
  const [state, setState] = useState<{ key?: string; url?: string; changed?: number; error?: string }>({});
  const key = enabled && before && after ? `${before}|${after}` : undefined;
  useEffect(() => {
    if (!key || !before || !after) return;
    let active = true;
    const read = (src: string) => new Promise<HTMLImageElement>((resolve, reject) => {
      const image = new Image();
      image.onload = () => resolve(image);
      image.onerror = () => reject(new Error("unreadable"));
      image.src = src;
    });
    void Promise.all([read(before), read(after)]).then(([a, b]) => {
      if (!active) return;
      const width = b.naturalWidth, height = b.naturalHeight;
      if (a.naturalWidth !== width || a.naturalHeight !== height) {
        setState({ key, error: "The two images differ in size, so changes can't be shown pixel by pixel." });
        return;
      }
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      const context = canvas.getContext("2d", { willReadFrequently: true })!;
      context.drawImage(a, 0, 0);
      const first = context.getImageData(0, 0, width, height).data;
      context.drawImage(b, 0, 0);
      const second = context.getImageData(0, 0, width, height);
      const pixels = second.data;
      let changed = 0;
      for (let i = 0; i < pixels.length; i += 4) {
        const difference = Math.abs(pixels[i]! - first[i]!) + Math.abs(pixels[i + 1]! - first[i + 1]!) + Math.abs(pixels[i + 2]! - first[i + 2]!);
        if (difference > 24) {
          changed++;
          pixels[i] = 255; pixels[i + 1] = 214; pixels[i + 2] = 64;
        } else {
          pixels[i] = pixels[i]! * 0.3; pixels[i + 1] = pixels[i + 1]! * 0.3; pixels[i + 2] = pixels[i + 2]! * 0.3;
        }
      }
      context.putImageData(second, 0, 0);
      setState({ key, url: canvas.toDataURL("image/png"), changed });
    }).catch(() => { if (active) setState({ key, error: "Changes can't be computed for these images." }); });
    return () => { active = false; };
  }, [key, before, after]);
  return state.key === key ? state : {};
}

export function ImageCompare({ left, right, load, controls }: Props) {
  const panorama = isPanorama(left?.image) || isPanorama(right?.image);
  const [centred, setCentred] = useState(false);
  const loadedBefore = useImage(left?.asset, load),
    loadedAfter = useImage(right?.asset, load);
  // With the seam centred, each panorama side is shown turned half a turn; a flat side as is.
  const beforeUrl = useCentredSeam(loadedBefore.url, centred && isPanorama(left?.image)),
    afterUrl = useCentredSeam(loadedAfter.url, centred && isPanorama(right?.image));
  const turned = (state: ReturnType<typeof useImage>, url: string | undefined): ReturnType<typeof useImage> =>
    url ? { ...state, url } : (({ url: _, ...rest }) => rest)(state);
  const before = turned(loadedBefore, beforeUrl), after = turned(loadedAfter, afterUrl);
  const [mode, setMode] = useState<"wipe" | "side" | "changes">("wipe"),
    [position, setPosition] = useState(25),
    [view, setView] = useState<View>(FIT);
  const compatible = canWipe(left?.image, right?.image);
  const loaded = !!before.url && !!after.url;
  const changes = useChanges(before.url, after.url, mode === "changes" && compatible);
  const active = !compatible && mode !== "side" ? "side" : mode;
  const viewport = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; view: View } | undefined>(undefined);
  // A new pair of images starts fitted.
  useEffect(() => setView(FIT), [left?.asset, right?.asset]);
  const size = () => { const box = viewport.current?.getBoundingClientRect(); return { width: box?.width ?? 0, height: box?.height ?? 0 }; };
  // The wheel zooms with ⌘/Ctrl (and a trackpad pinch, which arrives as one) at the pointer; once
  // zoomed, a plain wheel pans. Not zoomed, the page scrolls as usual.
  const section = useRef<HTMLElement>(null);
  useEffect(() => {
    const node = section.current;
    if (!node) return;
    const wheel = (event: WheelEvent) => {
      // Over either side's frame (or the wipe), in that frame's coordinates; elsewhere, the page's.
      const frame = (event.target as Element | null)?.closest(`.${styles.frame}, .${styles.wipe}`);
      if (!frame) return;
      const box = frame.getBoundingClientRect();
      if (event.ctrlKey || event.metaKey) {
        event.preventDefault();
        setView((current) => zoomAt(current, Math.exp(-event.deltaY * 0.01), event.clientX - box.left, event.clientY - box.top, box.width, box.height));
      } else {
        setView((current) => {
          if (current.scale === 1) return current;
          event.preventDefault();
          return clampView({ ...current, x: current.x - event.deltaX, y: current.y - event.deltaY }, box.width, box.height);
        });
      }
    };
    node.addEventListener("wheel", wheel, { passive: false });
    return () => node.removeEventListener("wheel", wheel);
  });
  const step = (factor: number) => { const { width, height } = size(); setView((current) => zoomAt(current, factor, width / 2, height / 2, width, height)); };
  const transform = { transform: `translate(${view.x}px, ${view.y}px) scale(${view.scale})` };
  // Drag pans when zoomed, in the views without a divider to drag.
  const pan = active === "wipe" ? {} : {
    onPointerDown: (event: React.PointerEvent<HTMLDivElement>) => { if (view.scale === 1) return; drag.current = { x: event.clientX, y: event.clientY, view }; event.currentTarget.setPointerCapture(event.pointerId); },
    onPointerMove: (event: React.PointerEvent<HTMLDivElement>) => { const start = drag.current; if (!start) return; const { width, height } = size(); setView(clampView({ ...start.view, x: start.view.x + event.clientX - start.x, y: start.view.y + event.clientY - start.y }, width, height)); },
    onPointerUp: () => { drag.current = undefined; },
  };
  // The divider sits at a point of the viewport; the clip is on the zoomed image, so map it across.
  const clip = view.scale === 1 ? 100 - position : (() => {
    const { width } = size();
    if (!width) return 100 - position;
    const content = ((position / 100) * width - view.x) / view.scale / width * 100;
    return 100 - Math.min(100, Math.max(0, content));
  })();
  function pane(value: CompareImage | undefined, image: ReturnType<typeof useImage>, fallback: string, measure = false) {
    return (
      <figure className={styles.pane}>
        <div className={styles.frame} ref={measure ? viewport : undefined} data-zoomed={view.scale > 1 || undefined} {...pan}>
          {image.url ? <img src={image.url} alt={value?.label ?? fallback} style={transform} draggable={false} />
            : <p role="status">{image.error ?? (value ? "Loading image…" : `${fallback} not available.`)}</p>}
        </div>
        <figcaption>{value?.label ?? fallback}</figcaption>
      </figure>
    );
  }
  return (
    <section ref={section} className={styles.viewer} aria-label="Image comparison">
      <div className={styles.toolbar}>
        <span>Compare</span>
        <div>
          {controls}
          {panorama && <Button aria-pressed={centred} title="Turn the panorama half a turn so the place where its edges meet is in the middle"
            onClick={() => setCentred((on) => !on)}>Centre the seam</Button>}
          <Button aria-pressed={active === "wipe"} disabled={!compatible} onClick={() => setMode("wipe")}>Wipe</Button>
          <Button aria-pressed={active === "side"} onClick={() => setMode("side")}>Side by side</Button>
          <Button aria-pressed={active === "changes"} disabled={!compatible || !loaded} onClick={() => setMode("changes")}>Changes</Button>
          <span className={styles.zoom} role="group" aria-label="Zoom">
            <Button aria-label="Zoom out" disabled={view.scale <= 1} onClick={() => step(1 / 1.5)}>−</Button>
            <Button aria-label="Fit" title="Fit (reset zoom)" onClick={() => setView(FIT)}>{Math.round(view.scale * 100)}%</Button>
            <Button aria-label="Zoom in" disabled={view.scale >= MAX_SCALE} onClick={() => step(1.5)}>+</Button>
          </span>
        </div>
      </div>
      {!compatible && left && right && (
        <p className={styles.note}>These images have different dimensions or coordinate systems. Compare them side by side.</p>
      )}
      {active === "wipe" && loaded ? (
        <div ref={viewport} className={styles.wipe} style={{ aspectRatio: `${left!.image.width}/${left!.image.height}` }}>
          <img src={after.url} alt={right?.label} style={transform} draggable={false} />
          <img src={before.url} alt={left?.label} style={{ ...transform, clipPath: `inset(0 ${clip}% 0 0)` }} draggable={false} />
          <div className={styles.divider} style={{ left: `${position}%` }}><span aria-hidden="true">↔</span></div>
          <input type="range" min={0} max={100} value={position} aria-label="Before and after divider"
            aria-valuetext={`${position}% ${left?.label ?? "source"}`} onChange={(event) => setPosition(Number(event.target.value))} />
        </div>
      ) : active === "changes" ? (
        <figure className={styles.pane}>
          <div ref={viewport} className={styles.frame} data-zoomed={view.scale > 1 || undefined} {...pan}
            style={left ? { aspectRatio: `${left.image.width}/${left.image.height}` } : undefined}>
            {changes.url ? <img src={changes.url} alt={`Changes from ${left?.label} to ${right?.label}`} style={transform} draggable={false} />
              : <p role="status">{changes.error ?? "Finding changes…"}</p>}
          </div>
          <figcaption>{changes.changed === undefined ? "Highlighted: pixels that changed"
            : changes.changed === 0 ? "No pixels changed." : `Highlighted: ${changes.changed.toLocaleString()} pixels that changed`}</figcaption>
        </figure>
      ) : (
        <div className={styles.side}>
          {pane(left, before, "Source", true)}
          {pane(right, after, "Result")}
        </div>
      )}
      {active === "wipe" && loaded && (
        <div className={styles.labels}><span>{left?.label}</span><span>{right?.label}</span></div>
      )}
      {panorama && centred && <p className={styles.note}>Turned half a turn: the seam, where the panorama's edges meet, runs down the middle.</p>}
      <p className={styles.note}>
        {active === "wipe" && loaded ? "Drag to compare · Arrow keys move the divider · " : ""}⌘/Ctrl + scroll or pinch to zoom{view.scale > 1 ? (active === "wipe" ? " · Scroll to pan" : " · Drag or scroll to pan") : ""}
      </p>
    </section>
  );
}

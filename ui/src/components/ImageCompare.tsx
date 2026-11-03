import { useEffect, useState, type ReactNode } from "react";
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
function useImage(asset: Asset | undefined, load: Props["load"]) {
  const [state, setState] = useState<{ url?: string; error?: string }>({});
  useEffect(() => {
    let active = true;
    let url: string | undefined;
    setState({});
    if (asset)
      void load(asset)
        .then((blob) => {
          if (active) {
            url = URL.createObjectURL(blob);
            setState({ url });
          }
        })
        .catch(() => {
          if (active)
            setState({
              error: "Image unavailable. The saved bytes could not be read.",
            });
        });
    return () => {
      active = false;
      if (url) URL.revokeObjectURL(url);
    };
  }, [asset, load]);
  return state;
}
export function ImageCompare({ left, right, load, controls }: Props) {
  const before = useImage(left?.asset, load),
    after = useImage(right?.asset, load);
  const [mode, setMode] = useState<"wipe" | "side">("wipe"),
    [position, setPosition] = useState(50);
  const compatible = canWipe(left?.image, right?.image);
  const wipe = mode === "wipe" && compatible && before.url && after.url;
  function pane(
    value: CompareImage | undefined,
    loaded: ReturnType<typeof useImage>,
    fallback: string,
  ) {
    return (
      <figure className={styles.pane}>
        {loaded.url ? (
          <img src={loaded.url} alt={value?.label ?? fallback} />
        ) : (
          <p role="status">
            {loaded.error ??
              (value ? "Loading image…" : `${fallback} not available.`)}
          </p>
        )}
        <figcaption>{value?.label ?? fallback}</figcaption>
      </figure>
    );
  }
  return (
    <section className={styles.viewer} aria-label="Image comparison">
      <div className={styles.toolbar}>
        <span>Compare</span>
        <div>
          {controls}
          <Button
            aria-pressed={mode === "wipe" && compatible}
            disabled={!compatible}
            onClick={() => setMode("wipe")}
          >
            Wipe
          </Button>
          <Button
            aria-pressed={mode === "side" || !compatible}
            onClick={() => setMode("side")}
          >
            Side by side
          </Button>
        </div>
      </div>
      {!compatible && left && right && (
        <p className={styles.note}>
          These images have different dimensions or coordinate systems. Compare
          them side by side.
        </p>
      )}
      {wipe ? (
        <div
          className={styles.wipe}
          style={{ aspectRatio: `${left!.image.width}/${left!.image.height}` }}
        >
          <img src={after.url} alt={right?.label} />
          <img
            src={before.url}
            alt={left?.label}
            style={{ clipPath: `inset(0 ${100 - position}% 0 0)` }}
          />
          <div className={styles.divider} style={{ left: `${position}%` }}>
            <span aria-hidden="true">↔</span>
          </div>
          <input
            type="range"
            min={0}
            max={100}
            value={position}
            aria-label="Before and after divider"
            aria-valuetext={`${position}% ${left?.label ?? "source"}`}
            onChange={(event) => setPosition(Number(event.target.value))}
          />
        </div>
      ) : (
        <div className={styles.side}>
          {pane(left, before, "Source")}
          {pane(right, after, "Result")}
        </div>
      )}
      {wipe && (
        <div className={styles.labels}>
          <span>{left?.label}</span>
          <span>{right?.label}</span>
        </div>
      )}
      {wipe && (
        <p className={styles.note}>
          Drag to compare · Arrow keys move the divider
        </p>
      )}
    </section>
  );
}

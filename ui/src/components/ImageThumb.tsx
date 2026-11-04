import { useEffect, useState } from "react";
import type { Asset } from "../api/assets";
import styles from "./ImageThumb.module.css";

/** An object URL for an image's preview, revoked when the image changes or the caller unmounts. */
export function useThumbnail(asset: Asset | undefined, preview: ((asset: Asset) => Promise<Blob>) | undefined): string | undefined {
  const [url, setUrl] = useState<string>();
  useEffect(() => {
    if (!asset || !preview) return;
    let alive = true, made: string | undefined;
    void preview(asset).then((blob) => { if (alive) { made = URL.createObjectURL(blob); setUrl(made); } }).catch(() => undefined);
    return () => { alive = false; if (made) URL.revokeObjectURL(made); setUrl(undefined); };
  }, [asset, preview]);
  return url;
}

/** An image as a thumbnail button: picking it puts the image in the input. */
export function ImageThumb({ asset, preview, label, disabled = false, onPick }: { asset: Asset; preview?: ((asset: Asset) => Promise<Blob>) | undefined; label: string; disabled?: boolean; onPick: () => void }) {
  const url = useThumbnail(asset, preview);
  return <button type="button" className={styles.thumb} aria-label={label} disabled={disabled} onClick={onPick}>
    {url ? <img src={url} alt="" /> : <span />}
    <small>{asset.width} × {asset.height}</small>
  </button>;
}

/** The chosen image, shown small beside its details (not a control). */
export function ImageTile({ asset, preview }: { asset: Asset | undefined; preview?: ((asset: Asset) => Promise<Blob>) | undefined }) {
  const url = useThumbnail(asset, preview);
  return url ? <img className={styles.tile} src={url} alt="" /> : <span className={styles.tile} aria-hidden="true" />;
}

/** The thumbnails of several images to choose from, as a wrapping grid. */
export function ImageThumbGrid({ children }: { children: React.ReactNode }) {
  return <div className={styles.grid}>{children}</div>;
}

import { useEffect, useId, useRef, useState } from "react";
import { cardinalityError, moveImage, type Asset } from "../api/assets";
import { Button } from "./Button";
import styles from "./ImageInput.module.css";

type Props = {
  label: string;
  value: readonly Asset[];
  min: number;
  max: number;
  onChange: (images: Asset[]) => void;
  upload: (file: File) => Promise<Asset>;
  preview: (asset: Asset) => Promise<Blob>;
  disabled?: boolean;
};

function Preview({ asset, load, position }: { asset: Asset; load: Props["preview"]; position: number }) {
  const [url, setUrl] = useState<string>();
  const [error, setError] = useState(false);
  useEffect(() => {
    let active = true;
    let objectUrl: string | undefined;
    setUrl(undefined);
    setError(false);
    void load(asset).then(blob => {
      if (active) { objectUrl = URL.createObjectURL(blob); setUrl(objectUrl); }
    }).catch(() => { if (active) setError(true); });
    return () => { active = false; if (objectUrl) URL.revokeObjectURL(objectUrl); };
  }, [asset, load]);
  return url ? <img className={styles.preview} src={url} alt={`Selected image ${position}`} />
    : <div className={styles.preview}>{error ? "Preview unavailable" : "Loading image…"}</div>;
}

export function ImageInput({ label, value, min, max, onChange, upload, preview, disabled = false }: Props) {
  const inputId = useId();
  const fileInput = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const locked = busy || disabled;
  async function add(files: File[]) {
    if (locked || files.length === 0) return;
    const countError = cardinalityError(value.length + files.length, 0, max);
    if (countError) { setError(countError); return; }
    setBusy(true);
    setError(undefined);
    const next = [...value];
    try {
      for (const file of files) {
        if (file.size > 128 * 1024 * 1024) throw new Error("Images must be no larger than 128 MiB.");
        next.push(await upload(file));
      }
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : "The image could not be imported.");
    } finally {
      if (next.length !== value.length) onChange(next);
      setBusy(false);
    }
  }
  return <section className={styles.input} aria-labelledby={`${inputId}-label`} aria-busy={busy}>
    <div className={styles.heading}><label id={`${inputId}-label`} htmlFor={inputId}>{label}</label><span>{value.length} / {max}</span></div>
    <p className={styles.hint} id={`${inputId}-help`}>PNG or JPEG · Up to 128 MiB each · {min > 0 ? `${min} required` : "Optional"}</p>
    <div className={styles.file}><Button disabled={locked || value.length >= max} busy={busy} onClick={() => fileInput.current?.click()}>{max === 1 ? "Choose image" : "Add images"}</Button></div>
    <input ref={fileInput} id={inputId} hidden type="file" accept="image/png,image/jpeg" multiple={max > 1}
      disabled={locked || value.length >= max} aria-describedby={`${inputId}-help`}
      onChange={event => { const files = Array.from(event.currentTarget.files ?? []); event.currentTarget.value = ""; void add(files); }} />
    {busy && <p role="status">Importing image…</p>}
    {error && <p role="alert">{error}</p>}
    {value.length > 0 && <ol className={styles.images}>
      {value.map((asset, index) => <li className={styles.image} key={`${asset.id}-${index}`}>
        <Preview asset={asset} load={preview} position={index + 1} />
        <div className={styles.details}><strong>Image {index + 1}</strong><span>{asset.width} × {asset.height} · {Math.max(1, Math.round(asset.size / 1024))} KB</span>
          <div className={styles.actions}>
            {max > 1 && <><Button disabled={locked || index === 0} onClick={() => onChange(moveImage(value, index, index - 1))} aria-label={`Move image ${index + 1} earlier`}>Earlier</Button>
              <Button disabled={locked || index === value.length - 1} onClick={() => onChange(moveImage(value, index, index + 1))} aria-label={`Move image ${index + 1} later`}>Later</Button></>}
            <Button disabled={locked} onClick={() => { setError(undefined); onChange(value.filter((_, position) => position !== index)); }} aria-label={`Remove image ${index + 1}`}>Remove</Button>
          </div>
        </div>
      </li>)}
    </ol>}
  </section>;
}

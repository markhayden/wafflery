import { useEffect, useId, useState, type ReactNode } from "react";
import { InfoTip } from "../../components/InfoTip";
import { imageHelp, imageLabel, paramHelp, paramLabel } from "./help";
import { ImageInput } from "../../components/ImageInput";
import type { Asset } from "../../api/assets";
import type { ImageSelection, SelectedImage } from "./selection";
import {
  fieldMode,
  object,
  type Action,
  type Json,
  type Schema,
} from "./model";
import styles from "./ActionForm.module.css";
type Props = {
  action: Action;
  params: Schema;
  onChange: (params: Schema) => void;
  images: ImageSelection;
  onImages: (slot: string, images: SelectedImage[]) => void;
  upload: (file: File) => Promise<Asset>;
  preview: (asset: Asset) => Promise<Blob>;
  /** Where the action is applied (location inputs), shown after the images they are drawn on. */
  locations?: ReactNode;
  /** Images saved earlier in this library, offered in each image step. */
  saved?: Asset[];
  disabled?: boolean;
};
function JsonField({
  label,
  info,
  value,
  onChange,
  disabled,
}: {
  label: string;
  info?: ReactNode;
  value: Json | undefined;
  onChange: (value: Json | undefined) => void;
  disabled?: boolean;
}) {
  const labelId = useId();
  const [text, setText] = useState(() =>
    value === undefined ? "" : JSON.stringify(value, null, 2),
  );
  const [error, setError] = useState<string>();
  return (
    <label className={styles.field}>
      <span className={styles.fieldLabel}><span id={labelId}>{label}</span>{info}</span>
      <textarea
        aria-labelledby={labelId}
        value={text}
        disabled={disabled}
        spellCheck={false}
        aria-invalid={!!error}
        rows={6}
        onChange={(event) => {
          setText(event.target.value);
          event.target.setCustomValidity("");
          try {
            onChange(
              event.target.value.trim()
                ? (JSON.parse(event.target.value) as Json)
                : undefined,
            );
            setError(undefined);
          } catch (failure) {
            const message = failure instanceof SyntaxError ? "Enter valid JSON." : failure instanceof Error ? failure.message : "Enter valid JSON.";
            setError(message);
            event.target.setCustomValidity(message);
          }
        }}
      />
      {error && (
        <small role="alert">
          {error} The last valid value is retained.
        </small>
      )}
    </label>
  );
}
export function ActionForm({
  action,
  params,
  onChange,
  images,
  onImages,
  upload,
  preview,
  locations,
  saved = [],
  disabled = false,
}: Props) {
  const id = useId();
  const schema = action.params_schema;
  const properties = object(schema.properties);
  const required = Array.isArray(schema.required) ? schema.required : [];
  function set(key: string, value: Json | undefined) {
    const next = { ...params };
    if (value === undefined) delete next[key];
    else next[key] = value;
    onChange(next);
  }
  const simple =
    schema.type === "object" &&
    properties &&
    !schema.anyOf &&
    !schema.oneOf &&
    !schema.allOf &&
    !schema.if &&
    !schema.$ref;
  // The order a person fills things in: images, then where (locations), then required settings,
  // optional settings (free-form JSON last), and the spending limit at the very end.
  const fieldEntries = simple ? Object.entries(properties).filter(([key]) => !action.cost.may_spend || key !== "max_spend_usd") : [];
  const isJson = ([, entry]: [string, Json])  => fieldMode(object(entry) ?? {}) === "json";
  const spendFields = fieldEntries.filter(([key]) => key === "max_spend_usd");
  const requiredFields = fieldEntries.filter(([key]) => key !== "max_spend_usd" && required.includes(key));
  const optionalFields = fieldEntries.filter(([key]) => key !== "max_spend_usd" && !required.includes(key))
    .sort((a, b) => Number(isJson(a)) - Number(isJson(b)));
  // What an empty field means, shown in it.
  const emptyText = (key: string, field: Schema) => key === "max_spend_usd"
    ? "No extra limit: the tool's own per-call cap applies"
    : field.default !== undefined ? `Not set: ${String(field.default)} (the tool's default)` : "Not set: the tool decides";
  const renderField = ([key, entry]: [string, Json]) => {
          const field = object(entry) ?? {};
          const label = paramLabel(key, field);
          const info = <InfoTip label={label}>{paramHelp(field, required.includes(key))}</InfoTip>;
          const tag = required.includes(key) ? <span className={styles.tag}>Required</span> : null;
          const mode = fieldMode(field);
          const value = params[key];
          // Free-form JSON settings need the tool's own syntax: they sort last, marked advanced.
          if (mode === "json")
            return (
              <JsonField
                key={key}
                label={`${label} · advanced`}
                info={info}
                value={value}
                onChange={(value) => set(key, value)}
                disabled={disabled}
              />
            );
          return (
            <label className={styles.field} key={key} htmlFor={`${id}-${key}`}>
              <span className={styles.fieldLabel}><span id={`${id}-${key}-label`}>{label}</span>{tag}{info}</span>
              {mode === "enum" ? (
                <select
                  id={`${id}-${key}`}
                  aria-labelledby={`${id}-${key}-label`}
                  required={required.includes(key)}
                  value={value === undefined ? "" : JSON.stringify(value)}
                  disabled={disabled}
                  onChange={(event) =>
                    set(
                      key,
                      event.target.value
                        ? (JSON.parse(event.target.value) as Json)
                        : undefined,
                    )
                  }
                >
                  <option value="">{emptyText(key, field)}</option>
                  {(field.enum as Json[]).map((option, index) => (
                    <option key={index} value={JSON.stringify(option)}>
                      {String(option)}
                    </option>
                  ))}
                </select>
              ) : mode === "boolean" ? (
                <select
                  id={`${id}-${key}`}
                  aria-labelledby={`${id}-${key}-label`}
                  required={required.includes(key)}
                  value={value === undefined ? "" : String(value)}
                  disabled={disabled}
                  onChange={(event) =>
                    set(
                      key,
                      event.target.value
                        ? event.target.value === "true"
                        : undefined,
                    )
                  }
                >
                  <option value="">{emptyText(key, field)}</option>
                  <option value="true">Yes</option>
                  <option value="false">No</option>
                </select>
              ) : (
                <input
                  id={`${id}-${key}`}
                  aria-labelledby={`${id}-${key}-label`}
                  required={required.includes(key)}
                  type={mode === "number" ? "number" : "text"}
                  placeholder={emptyText(key, field)}
                  step={field.type === "integer" ? 1 : "any"}
                  value={
                    typeof value === "string" || typeof value === "number"
                      ? value
                      : ""
                  }
                  disabled={disabled}
                  onChange={(event) =>
                    set(
                      key,
                      mode === "number"
                        ? event.target.value === ""
                          ? undefined
                          : Number(event.target.value)
                        : event.target.value,
                    )
                  }
                />
              )}
              {typeof field.description === "string" && (
                <small>{field.description}</small>
              )}
            </label>
          );
  };
  return (
    <div className={`${styles.form} ${styles.steps}`}>
      <p className={styles.whatItDoes}><span>What this does</span>{action.purpose}</p>
      {(action.images ?? []).map((slot) => (
        <section key={slot.slot} className={`${styles.form} ${styles.step}`}>
          <ImageInput
            label={imageLabel(slot)}
            info={<InfoTip label={imageLabel(slot)}>{imageHelp(slot)}</InfoTip>}
            value={images[slot.slot] ?? []}
            min={slot.required ? 1 : 0}
            max={slot.many ? 32 : 1}
            onChange={(value) => onImages(slot.slot, value)}
            upload={upload}
            preview={preview}
            disabled={disabled}
            renderDetails={(index, locked) => {
              const selected = images[slot.slot]?.[index];
              const coordinates = selected?.coordinates;
              const value = coordinates?.projection === "equirectangular" ? "equirectangular"
                : coordinates?.topology === "wrap" ? "flat-wrap" : "flat";
              return <label className={styles.field}>
                {slot.many ? `${imageLabel(slot)} ${index + 1} coordinates` : `${imageLabel(slot)} coordinates`}
                <select value={value} disabled={locked} onChange={event => {
                  const projection = event.target.value === "equirectangular" ? "equirectangular" : "flat";
                  const topology = event.target.value === "flat" ? "bounded" : "wrap";
                  onImages(slot.slot, (images[slot.slot] ?? []).map((image, position) => position === index
                    ? {...image, coordinates: {projection, topology}} : image));
                }}>
                  <option value="flat">Flat image · bounded edges</option>
                  <option value="flat-wrap">Flat image · horizontal wrap</option>
                  <option value="equirectangular">Panorama · equirectangular, horizontal wrap</option>
                </select>
              </label>;
            }}
          />
          {saved.length > 0 && (!images[slot.slot]?.length || slot.many) && <details className={styles.advanced}>
            <summary>Saved images ({saved.length})</summary>
            <div className={styles.savedGrid}>
              {saved.slice(0, 12).map((asset) => <SavedThumb key={asset.id} asset={asset} preview={preview}
                label={`Use saved image ${asset.width} × ${asset.height} as ${imageLabel(slot).toLowerCase()}`}
                disabled={disabled || (!!slot.many && (images[slot.slot]?.length ?? 0) >= 32)}
                onPick={() => onImages(slot.slot, slot.many ? [...(images[slot.slot] ?? []), asset] : [asset])} />)}
            </div>
          </details>}
        </section>
      ))}
      {locations && <section className={styles.step}>{locations}</section>}
      {simple ? (<>
        {requiredFields.length > 0 && <section className={styles.step}><h3 className={styles.stepTitle}>Settings</h3>{requiredFields.map(renderField)}</section>}
        {optionalFields.length > 0 && <section className={styles.step}>
          <details className={styles.optional}>
            <summary className={styles.stepTitle}>Settings <span className={styles.note}>· {optionalFields.length} optional</span></summary>
            <div className={styles.form}>{optionalFields.map(renderField)}</div>
          </details>
        </section>}
        {spendFields.length > 0 && <section className={styles.step}><h3 className={styles.stepTitle}>Spending limit</h3>{spendFields.map(renderField)}</section>}
      </>) : (
        <JsonField
          label="Parameters (JSON)"
          value={action.cost.may_spend ? Object.fromEntries(Object.entries(params).filter(([key]) => key !== "max_spend_usd")) : params}
          onChange={(value) => {
            const next = object(value);
            if (!next) throw new Error("Parameters must be an object");
            if (action.cost.may_spend) {
              if ("max_spend_usd" in next) throw new Error("Use the Spending cap field for max_spend_usd.");
              if (params.max_spend_usd !== undefined) next.max_spend_usd = params.max_spend_usd;
            }
            onChange(next);
          }}
          disabled={disabled}
        />
      )}
    </div>
  );
}

/** A saved image as a small thumbnail button that puts it in the step. */
function SavedThumb({ asset, preview, label, disabled, onPick }: { asset: Asset; preview: (asset: Asset) => Promise<Blob>; label: string; disabled: boolean; onPick: () => void }) {
  const [url, setUrl] = useState<string>();
  useEffect(() => {
    let alive = true, made: string | undefined;
    void preview(asset).then((blob) => { if (alive) { made = URL.createObjectURL(blob); setUrl(made); } }).catch(() => undefined);
    return () => { alive = false; if (made) URL.revokeObjectURL(made); };
  }, [asset, preview]);
  return <button type="button" className={styles.thumb} aria-label={label} disabled={disabled} onClick={onPick}>
    {url ? <img src={url} alt="" /> : <span />}
    <small>{asset.width} × {asset.height}</small>
  </button>;
}

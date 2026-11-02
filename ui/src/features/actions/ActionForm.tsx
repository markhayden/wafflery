import { useId, useState } from "react";
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
  disabled?: boolean;
};
function JsonField({
  label,
  value,
  onChange,
  disabled,
}: {
  label: string;
  value: Json | undefined;
  onChange: (value: Json | undefined) => void;
  disabled?: boolean;
}) {
  const labelId = useId();
  const [text, setText] = useState(() =>
    value === undefined ? "" : JSON.stringify(value, null, 2),
  );
  const [error, setError] = useState(false);
  return (
    <label className={styles.field}>
      <span id={labelId}>{label}</span>
      <textarea
        aria-labelledby={labelId}
        value={text}
        disabled={disabled}
        spellCheck={false}
        aria-invalid={error}
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
            setError(false);
          } catch {
            setError(true);
            event.target.setCustomValidity("Enter valid JSON.");
          }
        }}
      />
      {error && (
        <small role="alert">
          Enter valid JSON before running. The last valid value is retained.
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
  return (
    <div className={styles.form}>
      <p className={styles.purpose}>{action.purpose}</p>
      {(action.images ?? []).map((slot) => (
        <section key={slot.slot} className={styles.form}>
          <ImageInput
            label={
              slot.slot === slot.role
                ? slot.slot
                : `${slot.slot} · ${slot.role}`
            }
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
                {slot.many ? `${slot.slot} image ${index + 1} coordinates` : `${slot.slot} coordinates`}
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
        </section>
      ))}
      {simple ? (
        Object.entries(properties).map(([key, entry]) => {
          const field = object(entry) ?? {};
          const label = `${typeof field.title === "string" ? field.title : key}${required.includes(key) ? " *" : ""}`;
          const mode = fieldMode(field);
          const value = params[key];
          if (mode === "json")
            return (
              <JsonField
                key={key}
                label={label}
                value={value}
                onChange={(value) => set(key, value)}
                disabled={disabled}
              />
            );
          return (
            <label className={styles.field} key={key} htmlFor={`${id}-${key}`}>
              <span id={`${id}-${key}-label`}>{label}</span>
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
                  <option value="">Choose…</option>
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
                  <option value="">Not set</option>
                  <option value="true">Yes</option>
                  <option value="false">No</option>
                </select>
              ) : (
                <input
                  id={`${id}-${key}`}
                  aria-labelledby={`${id}-${key}-label`}
                  required={required.includes(key)}
                  type={mode === "number" ? "number" : "text"}
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
        })
      ) : (
        <JsonField
          label="Parameters (JSON)"
          value={params}
          onChange={(value) => {
            const next = object(value);
            if (!next) throw new Error("Parameters must be an object");
            onChange(next);
          }}
          disabled={disabled}
        />
      )}
      {(action.locations ?? []).length > 0 && (
        <p>
          Location bindings require the operations interface. Actions requiring
          those bindings are unavailable in this foundation.
        </p>
      )}
      <p className={styles.purpose}>
        Inputs are checked against the tool’s complete schema before execution.
      </p>
    </div>
  );
}

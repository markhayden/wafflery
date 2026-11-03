import { useEffect, useState } from "react";
import { workspaceClient, type Attempt } from "../../api/workspace";
import { Button } from "../../components/Button";
import { ImageCompare } from "../../components/ImageCompare";
import type { Asset } from "../../api/assets";
import { object } from "../actions/model";
import { humanize } from "../actions/help";
import { CodeBlock } from "../../components/CodeBlock";
import styles from "./Workspace.module.css";

export function SavedComparison({
  attempt,
  client,
  load,
  revision,
}: {
  attempt: Attempt;
  client: ReturnType<typeof workspaceClient>;
  load: (asset: Asset) => Promise<Blob>;
  revision: unknown;
}) {
  const [baseline, setBaseline] = useState<Attempt | null>(null);
  const [compare, setCompare] = useState(false);
  // A tool's preview of an output (rascal: the mask drawn over the image) shows by default;
  // turning it off shows the raw output (the black-and-white mask).
  const [overlay, setOverlay] = useState(true);
  const [error, setError] = useState<string>();
  const { tool, action } = attempt.request;
  useEffect(() => {
    let active = true;
    void client
      .baseline(tool, action)
      .then((value) => {
        if (active) setBaseline(value);
      })
      .catch((failure) => {
        if (active) setError(String(failure));
      });
    return () => {
      active = false;
    };
  }, [client, tool, action, revision]);
  async function pin(id: string | null) {
    try {
      await client.pin(tool, action, id);
      setBaseline(id ? attempt : null);
      setError(undefined);
    } catch (failure) {
      setError(String(failure));
    }
  }
  const comparing = compare && baseline && baseline.id !== attempt.id;
  const source = Object.values(attempt.inputs).flat()[0];
  const preview = attempt.outputs.find((output) => output.role === "preview");
  const plain = attempt.outputs.find((output) => output.role !== "preview");
  const right = (overlay && preview) || plain || attempt.outputs[0];
  const rightLabel = right === preview ? `${humanize(plain?.role ?? "result")} overlay` : humanize(right?.role ?? "result");
  const left = comparing ? baseline.outputs.find((output) => output.role === right?.role) ?? baseline.outputs[0] : source;
  const result = object(attempt.result);
  const declaredImages = object(result?.images)?.out;
  const dataOnly =
    result && (!Array.isArray(declaredImages) || declaredImages.length === 0);
  const settings = (run: Attempt) => ({
    parameters: run.request.params,
    images: run.request.images,
    coordinates: run.request.frames ?? {},
  });
  return (
    <section aria-label="Saved result comparison">
      {dataOnly ? (
        <div>
          <p>This run returned data without an output image.</p>
          <CodeBlock label="Result data" value={result.body ?? {}} />
        </div>
      ) : (
        <ImageCompare
          left={
            left
              ? { ...left, label: comparing ? "Baseline" : "Source" }
              : undefined
          }
          right={right ? { ...right, label: rightLabel } : undefined}
          load={load}
          controls={preview && plain ? (
            <button type="button" role="switch" aria-checked={overlay} className={styles.switch} onClick={() => setOverlay((value) => !value)}>
              <span className={styles.track} aria-hidden="true"><span /></span>
              {`${humanize(plain.role)} overlay`}
            </button>
          ) : undefined}
        />
      )}
      <div className="comparison-actions">
        {attempt.execution === "completed" && baseline?.id !== attempt.id && (
          <Button onClick={() => void pin(attempt.id)}>Pin as baseline</Button>
        )}
        {baseline && (
          <>
            <span>
              {baseline.id === attempt.id
                ? "This run is the baseline"
                : `Baseline · ${baseline.id.slice(0, 8)}`}
            </span>
            <Button onClick={() => void pin(null)}>Clear baseline</Button>
          </>
        )}
        {baseline && baseline.id !== attempt.id && (
          <Button
            aria-pressed={!!comparing}
            onClick={() => setCompare((value) => !value)}
          >
            {comparing ? "Compare with source" : "Compare with baseline"}
          </Button>
        )}
      </div>
      {error && <p role="alert">{error}</p>}
      {comparing && (
        <details>
          <summary>Compare saved settings</summary>
          <div className="settings-comparison">
            <section>
              <h3>Baseline settings</h3>
              <CodeBlock label="Baseline settings" value={settings(baseline)} />
            </section>
            <section>
              <h3>Selected run settings</h3>
              <CodeBlock label="Selected run settings" value={settings(attempt)} />
            </section>
          </div>
        </details>
      )}
    </section>
  );
}

import { useEffect, useState } from "react";
import { workspaceClient, type Attempt } from "../../api/workspace";
import { Button } from "../../components/Button";
import { ImageCompare } from "../../components/ImageCompare";
import type { Asset } from "../../api/assets";
import { object } from "../actions/model";
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
  const left = comparing ? baseline.outputs[0] : source;
  const right = attempt.outputs[0];
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
          <pre className={styles.resultData} aria-label="Result data">
            {JSON.stringify(result.body ?? {}, null, 2)}
          </pre>
        </div>
      ) : (
        <ImageCompare
          left={
            left
              ? { ...left, label: comparing ? "Baseline" : "Source" }
              : undefined
          }
          right={
            right
              ? {
                  ...right,
                  label:
                    right.role.charAt(0).toUpperCase() + right.role.slice(1),
                }
              : undefined
          }
          load={load}
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
              <pre>{JSON.stringify(settings(baseline), null, 2)}</pre>
            </section>
            <section>
              <h3>Selected run settings</h3>
              <pre>{JSON.stringify(settings(attempt), null, 2)}</pre>
            </section>
          </div>
        </details>
      )}
    </section>
  );
}

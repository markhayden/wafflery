import { useEffect, useState } from "react";
import { workspaceClient, type Attempt, type SavedImage } from "../../api/workspace";
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
  // An operation fed an earlier run's output (a mask, a location) has no image input: its "before"
  // is that run's preview, so the comparison shows what this operation changed.
  const [upstream, setUpstream] = useState<{ id: string; image: SavedImage; title: string }>();
  // A submitted run records where each input came from in `lineage` (the binding's own origin is
  // dropped at submission); a draft's binding still carries it.
  const origins = (value: Attempt) => [...Object.values(value.request.lineage ?? {}).flat(), ...Object.values(value.request.bindings ?? {}).flat().map((binding) => binding.origin)]
    .flatMap((origin) => origin && "attempt" in origin ? [origin.attempt] : []);
  const upstreamId = origins(attempt)[0];
  // The nearest earlier run with a picture: the run that made this input, or, when that run drew
  // none (fan's band profile), the runs before it (cut's canvas), a few steps back at most. An
  // image the size of this run's own output is preferred, so Wipe and Changes line up.
  useEffect(() => {
    let active = true;
    setUpstream(undefined);
    if (!upstreamId || Object.values(attempt.inputs).flat().length) return;
    const target = attempt.outputs.find((output) => output.role === "preview") ?? attempt.outputs[0];
    const fits = (image: SavedImage) => !target || (image.image.width === target.image.width && image.image.height === target.image.height);
    void (async () => {
      let frontier = [upstreamId], seen = new Set<string>(), fallback: { id: string; image: SavedImage; title: string } | undefined;
      for (let depth = 0; depth < 4 && frontier.length; depth++) {
        const next: string[] = [];
        for (const id of frontier) {
          if (seen.has(id)) continue;
          seen.add(id);
          const producer = await client.attempt(id).catch(() => undefined);
          if (!producer) continue;
          const title = producer.declaration?.document?.actions?.find((candidate) => candidate.name === producer.request.action)?.title ?? producer.request.action;
          const images = [...producer.outputs.filter((output) => output.role === "preview"), ...producer.outputs.filter((output) => output.role !== "preview")];
          const match = images.find(fits);
          if (match) { if (active) setUpstream({ id: upstreamId, image: match, title }); return; }
          if (!fallback && images[0]) fallback = { id: upstreamId, image: images[0], title };
          next.push(...origins(producer));
        }
        frontier = next;
      }
      if (active && fallback) setUpstream(fallback);
    })();
    return () => { active = false; };
  }, [client, upstreamId, attempt.inputs, attempt.outputs]);
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
  const source = Object.values(attempt.inputs).flat()[0] ?? (upstream?.id === upstreamId ? upstream?.image : undefined);
  const preview = attempt.outputs.find((output) => output.role === "preview");
  const plain = attempt.outputs.find((output) => output.role !== "preview");
  const right = (overlay && preview) || plain || attempt.outputs[0];
  const rightLabel = right === preview ? (plain ? `${humanize(plain.role)} overlay` : "Preview") : humanize(right?.role ?? "result");
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
              ? { ...left, label: comparing ? "Baseline" : left === upstream?.image ? `Before · from ${upstream.title}` : "Source" }
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
            <Button variant="destructive" onClick={() => void pin(null)}>Clear baseline</Button>
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

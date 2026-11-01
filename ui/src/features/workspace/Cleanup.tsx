import { useEffect, useState } from "react";
import type { LocalApi } from "../../api/client";
import { workspaceClient, type AttemptSummary } from "../../api/workspace";
import { exampleClient, type ExampleSummary } from "../../api/examples";
import { Button } from "../../components/Button";
type Selection = {
  attempts: string[];
  examples: string[];
  unused_assets: boolean;
};
type Preview = {
  token: string;
  selection: Selection;
  reclaimable_bytes: number;
  total_asset_bytes: number;
  baselines_cleared: number;
};
const empty: Selection = { attempts: [], examples: [], unused_assets: false };
const mib = (bytes: number) => `${(bytes / 1024 / 1024).toFixed(2)} MiB`;
export function Cleanup({
  api,
  onDone,
}: {
  api: LocalApi;
  onDone: () => Promise<void>;
}) {
  const [runs, setRuns] = useState<AttemptSummary[]>([]),
    [runCursor, setRunCursor] = useState<number | null>(null);
  const [selection, setSelection] = useState<Selection>(empty),
    [preview, setPreview] = useState<Preview>();
  const [examples, setExamples] = useState<ExampleSummary[]>([]),
    [cursor, setCursor] = useState<number | null>(null);
  const [usage, setUsage] = useState<number>(),
    [busy, setBusy] = useState(false),
    [notice, setNotice] = useState("");
  async function request(path: string, body: unknown) {
    return (
      await api.request(path, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      })
    ).json() as Promise<unknown>;
  }
  useEffect(() => {
    let active = true;
    void Promise.all([
      workspaceClient(api).history(),
      exampleClient(api).list(),
      api
        .request("/api/cleanup/preview", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(empty),
        })
        .then((response) => response.json() as Promise<Preview>),
    ])
      .then(([history, page, usage]) => {
        if (active) {
          setRuns(history.items);
          setRunCursor(history.cursor);
          setExamples(page.items);
          setCursor(page.cursor);
          setUsage(usage.total_asset_bytes);
        }
      })
      .catch((failure) => {
        if (active) setNotice(String(failure));
      });
    return () => {
      active = false;
    };
  }, [api]);
  function toggle(kind: "attempts" | "examples", id: string) {
    setPreview(undefined);
    setSelection((previous) => ({
      ...previous,
      [kind]: previous[kind].includes(id)
        ? previous[kind].filter((value) => value !== id)
        : [...previous[kind], id],
    }));
  }
  async function review() {
    setBusy(true);
    setNotice("");
    try {
      const value = (await request(
        "/api/cleanup/preview",
        selection,
      )) as Preview;
      setPreview(value);
      setUsage(value.total_asset_bytes);
    } catch (failure) {
      setNotice(String(failure));
    } finally {
      setBusy(false);
    }
  }
  async function confirm() {
    if (!preview) return;
    setBusy(true);
    setNotice("");
    try {
      const result = (await request("/api/cleanup/confirm", {
        selection: preview.selection,
        token: preview.token,
      })) as { reclaimed_bytes: number; failures: string[] };
      setSelection(empty);
      setPreview(undefined);
      await onDone();
      const history = await workspaceClient(api).history();
      setRuns(history.items);
      setRunCursor(history.cursor);
      const page = await exampleClient(api).list();
      setExamples(page.items);
      setCursor(page.cursor);
      setUsage((previous) =>
        Math.max(0, (previous ?? 0) - result.reclaimed_bytes),
      );
      setNotice(
        `Cleanup complete. Reclaimed ${mib(result.reclaimed_bytes)}.${result.failures.length ? ` ${result.failures.join(" ")}` : ""}`,
      );
    } catch (failure) {
      setPreview(undefined);
      setNotice(String(failure));
    } finally {
      setBusy(false);
    }
  }
  async function olderRuns() {
    if (runCursor === null) return;
    try {
      const page = await workspaceClient(api).history(runCursor);
      setRuns((previous) => [...previous, ...page.items]);
      setRunCursor(page.cursor);
    } catch (failure) {
      setNotice(String(failure));
    }
  }
  async function more() {
    if (cursor === null) return;
    try {
      const page = await exampleClient(api).list(cursor);
      setExamples((previous) => [...previous, ...page.items]);
      setCursor(page.cursor);
    } catch (failure) {
      setNotice(String(failure));
    }
  }
  return (
    <div className="examples">
      <p>
        {usage === undefined
          ? "Reading local image usage…"
          : `Saved image storage: ${mib(usage)}.`}{" "}
        SQLite metadata and backups use additional space.
      </p>
      <p>
        Shared images stay while a surviving run or example references them.
        Tool workspaces are separate: their size is excluded and nothing there
        is deleted.
      </p>
      <fieldset disabled={busy}>
        <legend>Runs</legend>
        {runs.map((run) => (
          <label key={run.id}>
            <input
              type="checkbox"
              checked={selection.attempts.includes(run.id)}
              onChange={() => toggle("attempts", run.id)}
            />{" "}
            Delete run {run.id.slice(0, 8)} · {run.tool} / {run.action}
          </label>
        ))}
      </fieldset>
      {runCursor !== null && (
        <Button onClick={() => void olderRuns()}>Load older runs</Button>
      )}
      <fieldset disabled={busy}>
        <legend>Examples</legend>
        {examples.map((example) => (
          <label key={example.id}>
            <input
              type="checkbox"
              checked={selection.examples.includes(example.id)}
              onChange={() => toggle("examples", example.id)}
            />{" "}
            Delete example {example.name}
          </label>
        ))}
      </fieldset>
      {cursor !== null && (
        <Button onClick={() => void more()}>Load older examples</Button>
      )}
      <label>
        <input
          type="checkbox"
          disabled={busy}
          checked={selection.unused_assets}
          onChange={(event) => {
            setPreview(undefined);
            setSelection((previous) => ({
              ...previous,
              unused_assets: event.target.checked,
            }));
          }}
        />{" "}
        Include unreferenced saved images, including images only selected in
        drafts
      </label>
      <p>
        Up to 1,000 image files per cleanup. History never expires
        automatically.
      </p>
      <Button busy={busy} onClick={() => void review()}>
        Preview cleanup
      </Button>
      {preview && (
        <section aria-label="Cleanup preview">
          <p>
            {preview.selection.attempts.length} runs and{" "}
            {preview.selection.examples.length} examples will be deleted.{" "}
            {preview.baselines_cleared} baseline pins will clear.
          </p>
          <p>
            Reclaimable image bytes: {mib(preview.reclaimable_bytes)}. Shared
            image bytes are excluded.
          </p>
          <Button disabled={busy} onClick={() => void confirm()}>
            Confirm cleanup
          </Button>
        </section>
      )}
      {notice && <p role="status">{notice}</p>}
    </div>
  );
}

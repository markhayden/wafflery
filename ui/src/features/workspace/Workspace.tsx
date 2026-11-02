import { RestartTool } from "./RestartTool";
import { ToolSetup } from "./ToolSetup";
import { readToolSession } from "../../api/tool-session";
import { Cleanup } from "./Cleanup";
import { Pipeline } from "./Pipeline";
import { exampleClient, type Example } from "../../api/examples";
import { Examples } from "./Examples";
import { watchWorkspace } from "../../api/events";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { LocalApi } from "../../api/client";
import { assetClient, type Asset } from "../../api/assets";
import {
  workspaceClient,
  type Attempt,
  type Snapshot,
  type Submission,
} from "../../api/workspace";
import {
  initialParameters,
  object,
  type Schema,
} from "../actions/model";
import { encodeSelection, restoreSelection, type ImageSelection } from "../actions/selection";
import { ActionForm } from "../actions/ActionForm";
import { Button } from "../../components/Button";
import { Picker } from "../../components/Picker";
import { Drawer } from "../../components/Drawer";
import { SavedComparison } from "./SavedComparison";
import { History } from "./History";
import { Inspector } from "./Inspector";
import styles from "./Workspace.module.css";
export function Workspace({
  api,
  initial,
}: {
  api: LocalApi;
  initial: Snapshot;
}) {
  const [snapshot, setSnapshot] = useState(initial);
  const [connection, setConnection] = useState("");
  const client = useMemo(() => workspaceClient(api), [api]),
    assets = useMemo(() => assetClient(api), [api]);
  const examples = useMemo(() => exampleClient(api), [api]);
  const [draftExample, setDraftExample] = useState<Example>();
  const preferred = new URLSearchParams(window.location.search).get("tool");
  const [toolId, setToolId] = useState(
    preferred ?? snapshot.tools[0]?.id ?? "",
  );
  const snapshotReads = useRef<Promise<void>>(Promise.resolve());
  const updateSnapshot = useCallback((signal?: AbortSignal): Promise<void> => {
    // All snapshot reads are serialized so older responses cannot replace newer state.
    const next = snapshotReads.current.catch(() => {}).then(async () => {
      if (signal?.aborted) return;
      const value = await client.snapshot();
      if (!signal?.aborted) setSnapshot(value);
    });
    snapshotReads.current = next;
    return next;
  }, [client]);
  const loadToolSession = useCallback(async () => {
    // The refresh emits an invalidation. Only ordered snapshots update the declaration.
    await client.refresh(toolId);
    return readToolSession(api, toolId);
  }, [api, client, toolId]);
  const tool = snapshot.tools.find((tool) => tool.id === toolId);
  const declaration = tool?.declaration;
  const sessionId = object(declaration?.tool_session)?.id;
  const [restartingTools, setRestartingTools] = useState<Set<string>>(() => new Set());
  const restarting = restartingTools.has(toolId);
  const restartToolSession = useCallback(async () => {
    setRestartingTools(current => new Set(current).add(toolId));
    try {
      const restarted = await client.restart(toolId);
      const id = object(restarted.tool_session)?.id;
      if (typeof id !== "string") throw new Error("The new session could not be identified.");
      return id;
    } finally {
      // Only ordered snapshots publish declarations, including a failed replacement.
      try { await updateSnapshot(); }
      finally { setRestartingTools(current => { const next = new Set(current); next.delete(toolId); return next; }); }
    }
  }, [client, toolId, updateSnapshot]);
  const [actionId, setActionId] = useState(
    declaration?.document.actions[0]?.name ?? "",
  );
  const action = declaration?.document.actions.find((a) => a.name === actionId);
  const [draftRevision, setDraftRevision] = useState(0);
  const [params, setParams] = useState<Schema>(() =>
    initialParameters(action?.params_schema ?? {}),
  );
  const [images, setImages] = useState<ImageSelection>({});
  const [savedImages, setSavedImages] = useState<Asset[]>([]);
  const [selected, setSelected] = useState<Attempt | undefined>();
  const recent = snapshot.attempts;
  const [selectedId, setSelectedId] = useState<string | null>();
  const [busy, setBusy] = useState(false),
    [error, setError] = useState<string>();
  const [drawer, setDrawer] = useState<{
    opener: HTMLElement;
    view: "inspect" | "history" | "examples" | "pipeline" | "cleanup" | "setup";
  } | null>(null);
  const submitted = useRef<{ body: string; request: Submission } | null>(null);
  useEffect(() => {
    void assets
      .list()
      .then(setSavedImages)
      .catch(() => {});
  }, [assets]);
  useEffect(() => {
    const controller = new AbortController();
    if (initial.event_session)
      void watchWorkspace(
        api,
        initial.event_session,
        () => updateSnapshot(controller.signal),
        (message) => {
          if (!controller.signal.aborted) setConnection(message);
        },
        controller.signal,
      );
    return () => controller.abort();
  }, [api, updateSnapshot, initial.event_session]);
  useEffect(() => {
    let active = true;
    const id =
      selectedId === null
        ? undefined
        : (selectedId ??
          snapshot.attempts.find(
            (a) => a.tool === toolId && a.action === actionId,
          )?.id);
    if (!id) {
      setSelected(undefined);
      return;
    }
    void client
      .attempt(id)
      .then((attempt) => {
        if (active) setSelected(attempt);
      })
      .catch((failure) => {
        if (active) {
          setSelected(undefined);
          setError(
            failure instanceof Error ? failure.message : "Unable to load run.",
          );
        }
      });
    return () => {
      active = false;
    };
  }, [client, snapshot, selectedId, toolId, actionId]);
  function openAttempt(id: string) {
    void client
      .attempt(id)
      .then((attempt) => {
        const restored = restoreSelection(attempt.inputs);
        setDraftRevision((value) => value + 1);
        setDraftExample(undefined);
        setToolId(attempt.request.tool);
        setActionId(attempt.request.action);
        setParams(structuredClone(attempt.request.params));
        setImages(restored);
        submitted.current = null;
        setSelectedId(id);
        setSelected(attempt);
        setDrawer(null);
      })
      .catch((failure) => setError(String(failure)));
  }
  function openExample(example: Example) {
    setDraftRevision((value) => value + 1);
    const current = snapshot.tools.find(
      (tool) => tool.id === example.request.tool,
    )?.declaration;
    if (
      !current?.document.actions.some(
        (action) => action.name === example.request.action,
      )
    )
      throw new Error(
        "This example needs a tool action that is not currently available.",
      );
    const restored = restoreSelection(example.inputs);
    setToolId(example.request.tool);
    setActionId(example.request.action);
    setParams(structuredClone(example.request.params));
    setImages(restored);
    setSelectedId(null);
    setSelected(undefined);
    setDraftExample(example);
    setError(undefined);
    submitted.current = null;
    setDrawer(null);
  }
  function chooseTool(id: string) {
    setDraftRevision((value) => value + 1);
    setDraftExample(undefined);
    setToolId(id);
    const action = snapshot.tools.find((t) => t.id === id)?.declaration
      ?.document.actions[0];
    setActionId(action?.name ?? "");
    setParams(initialParameters(action?.params_schema ?? {}));
    setImages({});
    setSelectedId(undefined);
    setSelected(undefined);
    setError(undefined);
    submitted.current = null;
  }
  function chooseAction(id: string) {
    setDraftRevision((value) => value + 1);
    setDraftExample(undefined);
    setActionId(id);
    setParams(
      initialParameters(
        declaration?.document.actions.find((a) => a.name === id)
          ?.params_schema ?? {},
      ),
    );
    setImages({});
    setSelectedId(undefined);
    setSelected(undefined);
    setError(undefined);
    submitted.current = null;
  }
  async function run() {
    if (!action || !declaration || busy) return;
    setError(undefined);
    setBusy(true);
    try {
      const {images: bindings, frames} = encodeSelection(
        declaration.document.contract, action.images ?? [], images,
      );
      const body = JSON.stringify({
        tool: toolId,
        action: actionId,
        revision: declaration.revision,
        params,
        images: bindings,
        frames,
      });
      if (submitted.current?.body !== body)
        submitted.current = {
          body,
          request: {
            id: crypto.randomUUID(),
            tool: toolId,
            action: actionId,
            revision: declaration.revision,
            params: structuredClone(params),
            images: bindings,
            frames,
          },
        };
      const attempt = await client.submit(submitted.current.request);
      // The submission is acknowledged even if refreshing history fails afterward.
      submitted.current = null;
      setSelectedId(attempt.id);
      setSelected(attempt);
      await updateSnapshot();
    } catch (failure) {
      setError(
        failure instanceof Error
          ? failure.message
          : "The attempt could not be submitted.",
      );
    } finally {
      setBusy(false);
    }
  }
  const noRun = selected?.execution === "rejected";
  const rpcError = object(object(selected?.raw_response)?.error);
  const refusalReason = object(object(rpcError?.data)?.error)?.reason ?? rpcError?.message;
  const working =
    selected &&
    (["submitted", "running", "unconfirmed", "cancelling"].includes(
      selected.execution,
    ) ||
      selected.preservation.contract === "pending");
  return (
    <div className={styles.workspace}>
      <div>
        <Button
          onClick={(event) =>
            setDrawer({ opener: event.currentTarget, view: "history" })
          }
        >
          History
        </Button>{" "}
        <Button
          onClick={(event) =>
            setDrawer({ opener: event.currentTarget, view: "examples" })
          }
        >
          Examples
        </Button>{" "}
        <Button
          onClick={(event) =>
            setDrawer({ opener: event.currentTarget, view: "cleanup" })
          }
        >
          Clean up library
        </Button>
      </div>
      <div className={styles.pickers}>
        <Picker
          label="Tool"
          value={toolId}
          onChange={chooseTool}
          choices={snapshot.tools.map((t) => ({
            id: t.id,
            title: t.declaration?.document.tool.name ?? t.id,
            description:
              t.error ??
              `Version ${t.declaration?.document.tool.version ?? "unknown"}`,
          }))}
        />
        <Picker
          label="Action"
          value={actionId}
          onChange={chooseAction}
          disabled={!declaration}
          choices={(declaration?.document.actions ?? []).map((a) => ({
            id: a.name,
            title: a.title,
            description: (a.images ?? []).length
              ? "Image → Result"
              : "Parameters → Result",
          }))}
        />
      </div>
      {tool && (
        <div className={styles.toolContext}>
          <p className={styles.note}>{declaration?.document.tool.purpose ?? "Tool setup is unavailable."}</p>
          <Button onClick={event => setDrawer({ opener: event.currentTarget, view: "setup" })}>Tool setup</Button>
        </div>
      )}
      {!tool && toolId && (
        <p role="status">
          Tool “{toolId}” is not registered in this workspace. Register it while
          the host is stopped, or choose a connected tool.
        </p>
      )}
      {connection && <p role="status">{connection}</p>}
      {tool?.error && <p role="alert">{tool.error}</p>}
      <div className={styles.columns}>
        <form
          className={styles.panel}
          onSubmit={(event) => {
            event.preventDefault();
            void run();
          }}
        >
          <header className={styles.panelHeading}>
            <h2>Input</h2>
          </header>
          <div
            className={styles.panelBody}
            role="region"
            aria-label="Input settings"
            tabIndex={0}
            key={draftRevision}
          >
            {draftExample && (
              <p className={styles.note}>
                From example: {draftExample.name}
                {draftExample.notes ? ` · ${draftExample.notes}` : ""}
              </p>
            )}
            {action ? (
              <ActionForm
                key={`${toolId}/${actionId}/${draftRevision}/${declaration?.revision}`}
                action={action}
                params={params}
                onChange={setParams}
                images={images}
                onImages={(slot, value) =>
                  setImages((values) => ({ ...values, [slot]: value }))
                }
                upload={async (file) => {
                  const asset = await assets.upload(file);
                  setSavedImages(await assets.list());
                  return asset;
                }}
                preview={assets.preview}
              />
            ) : (
              <p>No supported action is available.</p>
            )}
            {action?.images?.length && savedImages.length > 0 ? (
              <details>
                <summary>Saved images ({savedImages.length})</summary>
                <div className={styles.saved}>
                  {action.images.map((slot) => (
                    <div key={slot.slot}>
                      <p>Use for {slot.slot}</p>
                      {savedImages.slice(0, 12).map((asset, index) => (
                        <Button
                          key={asset.id}
                          disabled={slot.many && (images[slot.slot]?.length ?? 0) >= 32}
                          onClick={() =>
                            setImages((values) => ({
                              ...values,
                              [slot.slot]: slot.many ? [...(values[slot.slot] ?? []), asset] : [asset],
                            }))
                          }
                        >
                          Image {index + 1} · {asset.width} × {asset.height}
                        </Button>
                      ))}
                    </div>
                  ))}
                </div>
              </details>
            ) : null}
            {error && <p role="alert">{error}</p>}
          </div>
          <div className={styles.run}>
            <p>
              {action?.cost.may_spend
                ? "Paid execution needs provider setup and a run limit."
                : "Free action · Inputs are saved locally"}
            </p>
            <Button
              type="submit"
              busy={busy}
              disabled={
                !action ||
                action.cost.may_spend ||
                action.readiness.state === "unavailable" ||
                !!working ||
                !!tool?.busy ||
                (action.images ?? []).some(
                  (slot) => slot.required && !images[slot.slot]?.length,
                ) ||
                (action.locations ?? []).some((slot) => slot.required)
              }
            >
              Run
            </Button>
          </div>
        </form>
        <section className={styles.panel} aria-label="Result">
          <div className={`${styles.resultHeading} ${styles.panelHeading}`}>
            <h2>Result</h2>
            <span role="status">
              {selected
                ? selected.execution === "unconfirmed"
                  ? "Cancellation requested · outcome unconfirmed"
                  : selected.execution
                : "Ready"}
            </span>
          </div>
          <div
            className={styles.panelBody}
            role="region"
            aria-label="Result content"
            tabIndex={0}
          >
            {selected &&
              ["submitted", "running"].includes(selected.execution) && (
                <Button
                  onClick={() => {
                    void client
                      .cancel(selected.id)
                      .then(setSelected)
                      .catch((failure) =>
                        setError(
                          failure instanceof Error
                            ? failure.message
                            : "Unable to request cancellation.",
                        ),
                      );
                  }}
                >
                  Cancel run
                </Button>
              )}
            {selected ? (
              <>
                {working && tool?.progress?.message && (
                  <p className={styles.note}>{tool.progress.message}</p>
                )}
                {noRun ? <>
                  <p role="status">No run created.</p>
                  {typeof refusalReason === "string" && <p>{refusalReason}</p>}
                </> : <>
                <SavedComparison
                  attempt={selected}
                  client={client}
                  load={assets.preview}
                  revision={snapshot}
                />
                <p className={styles.note}>
                  Contract evidence: {selected.preservation.contract}. Editing
                  the inputs starts a new draft.
                </p>
                </>}
                {selected.notices.map((notice, index) => (
                  <p className={styles.note} key={index}>
                    {notice}
                  </p>
                ))}
              </>
            ) : (
              <div className={styles.empty}>
                {draftExample ? (
                  <>
                    <strong>Example loaded: {draftExample.name}</strong>
                    <p>
                      Your saved inputs are ready. Click Run to create a new
                      result.
                    </p>
                    <Button
                      onClick={(event) =>
                        setDrawer({
                          opener: event.currentTarget,
                          view: "history",
                        })
                      }
                    >
                      Browse run history
                    </Button>
                  </>
                ) : (
                  <>
                    <p>Choose your inputs and run an action.</p>
                    <p>The result and its saved evidence will appear here.</p>
                  </>
                )}
              </div>
            )}
          </div>
          {selected && (
            <div className={`${styles.resultHeading} ${styles.panelFooter}`}>
              <p>Saved attempt · {selected.id.slice(0, 8)}</p>
              <Button
                onClick={(event) =>
                  setDrawer({ opener: event.currentTarget, view: "inspect" })
                }
              >
                Inspect run
              </Button>
              <Button
                onClick={(event) =>
                  setDrawer({ opener: event.currentTarget, view: "pipeline" })
                }
              >
                Use in your pipeline
              </Button>
            </div>
          )}
        </section>
      </div>
      {recent.length > 0 && (
        <section className={styles.recent} aria-label="Recent attempts">
          <h2>Recent attempts</h2>
          {recent.slice(0, 6).map((attempt) => (
            <Button key={attempt.id} onClick={() => openAttempt(attempt.id)}>
              {attempt.action} · {attempt.execution} · {attempt.id.slice(0, 8)}
            </Button>
          ))}
        </section>
      )}
      <Drawer
        open={drawer !== null}
        opener={drawer?.opener ?? null}
        onClose={() => setDrawer(null)}
        title={
          drawer?.view === "setup"
            ? "Tool setup"
            : drawer?.view === "history"
            ? "History"
            : drawer?.view === "examples"
              ? "Examples"
              : drawer?.view === "pipeline"
                ? "Use in your pipeline"
                : drawer?.view === "cleanup"
                  ? "Clean up library"
                  : "Run details"
        }
        description={
          drawer?.view === "setup"
            ? `${declaration?.document.tool.name ?? toolId} · Requirements and session spending`
            : drawer?.view === "history"
            ? "Saved local attempts. Opening a run never executes it."
            : drawer?.view === "examples"
              ? "Reusable inputs, saved locally."
              : drawer?.view === "cleanup"
                ? "Preview changes before deleting saved records."
                : selected
                  ? `${selected.request.tool} / ${selected.request.action} · ${selected.id}`
                  : ""
        }
      >
        {drawer?.view === "setup" ? (
          <>
            {restarting ? <p role="status">Checking the replacement session…</p> : declaration && action ? (
              <ToolSetup
                key={`${toolId}:${actionId}:${sessionId}`}
                tool={declaration.document.tool}
                action={action}
                loadSession={loadToolSession}
              />
            ) : (
              <p role="status">Setup is unavailable because this tool or action is no longer available.</p>
            )}
            {tool && <RestartTool key={toolId} busy={tool.busy || restarting} onRestart={restartToolSession} />}
          </>
        ) : drawer?.view === "history" ? (
          <History client={client} onOpen={openAttempt} />
        ) : drawer?.view === "examples" ? (
          <Examples
            client={examples}
            selectedId={selected?.id}
            onOpen={openExample}
          />
        ) : drawer?.view === "pipeline" && selected ? (
          <Pipeline attempt={selected} api={api} />
        ) : drawer?.view === "cleanup" ? (
          <Cleanup
            api={api}
            onDone={async () => {
              setSelectedId(null);
              setSelected(undefined);
              await updateSnapshot();
              setSavedImages(await assets.list());
            }}
          />
        ) : (
          selected && <Inspector key={selected.id} attempt={selected} />
        )}
      </Drawer>
    </div>
  );
}

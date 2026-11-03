import {useArtifactRestore} from "./useArtifactRestore";
import {NativePreservation} from "./NativePreservation";
import {useSavedArtifacts} from "./useSavedArtifacts";
import {compatibleDestinations,transferBinding,encodePortBindings,savedSources,type ArtifactSource} from "./artifact-transfer";
import {ArtifactOutputs} from "./ArtifactOutputs";
import {artifactOccurrences} from "./artifacts";
import {PortEditor} from "../actions/PortEditor";
import {validatePorts,type PortBindings} from "../actions/ports";
import {ApiError,bindingIssues as parseBindingIssues,type BindingIssue} from "../../api/client";
import { ActivityPicker } from "./ActivityPicker";
import { firstActivity } from "./activities";
import type { ActivityKind } from "../actions/model";
import { WorkspaceFrame } from "../../components/WorkspaceFrame";
import { Shutdown } from "../../components/Shutdown";
import { reportClient } from "../../api/reports";
import { ReportOwner } from "./report-owner";
import { RunOutcome } from "./RunOutcome";
import { CostReview } from "./CostReview";
import { useEstimateDraft } from "./useEstimateDraft";
import { positiveCap } from "../../api/estimates";
import { parseToolSession } from "../../api/tool-session";
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
  const [sendSource,setSendSource]=useState<ArtifactSource>();
  const [connection, setConnection] = useState("");
  const client = useMemo(() => workspaceClient(api), [api]),
    assets = useMemo(() => assetClient(api), [api]);
  const reports = useMemo(() => new ReportOwner(reportClient(api)), [api]);
  const examples = useMemo(() => exampleClient(api), [api]);
  const [draftExample, setDraftExample] = useState<Example>();
  const preferred = new URLSearchParams(window.location.search).get("tool");
  const [toolId, setToolId] = useState(
    preferred ?? snapshot.tools[0]?.id ?? "",
  );
  const [drawer, setDrawer] = useState<{
    opener: HTMLElement;
    tool?: string;
    action?: string;
    view: "inspect" | "history" | "examples" | "pipeline" | "cleanup" | "setup";
  } | null>(null);
  const setupToolId = drawer?.view === "setup" ? drawer.tool ?? toolId : toolId;
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
    await client.refresh(setupToolId);
    return readToolSession(api, setupToolId);
  }, [api, client, setupToolId]);
  const tool = snapshot.tools.find((tool) => tool.id === toolId);
  const declaration = tool?.declaration;
  const sessionId = object(declaration?.tool_session)?.id;
  const [restartingTools, setRestartingTools] = useState<Set<string>>(() => new Set());
  const restarting = restartingTools.has(toolId);
  const restartToolSession = useCallback(async () => {
    setRestartingTools(current => new Set(current).add(setupToolId));
    try {
      const restarted = await client.restart(setupToolId);
      const id = object(restarted.tool_session)?.id;
      if (typeof id !== "string") throw new Error("The new session could not be identified.");
      return id;
    } finally {
      // Only ordered snapshots publish declarations, including a failed replacement.
      try { await updateSnapshot(); }
      finally { setRestartingTools(current => { const next = new Set(current); next.delete(setupToolId); return next; }); }
    }
  }, [client, setupToolId, updateSnapshot]);
  const [actionId, setActionId] = useState(
    firstActivity(declaration?.document.actions ?? [])?.name ?? "",
  );
  const action = declaration?.document.actions.find((a) => a.name === actionId);
  const [emptyKind, setEmptyKind] = useState<ActivityKind>("task");
  const activityKind = action?.kind ?? emptyKind;
  const [draftRevision, setDraftRevision] = useState(0);
  const [estimateEditRevision, setEstimateEditRevision] = useState(0);
  const [params, setParams] = useState<Schema>(() =>
    initialParameters(action?.params_schema ?? {}),
  );
  const [portImporting,setPortImporting]=useState(false);
  const [portBindings,setPortBindings]=useState<PortBindings>({});
  useEffect(()=>setPortImporting(false),[toolId,actionId,draftRevision,declaration?.revision]);
  const [bindingIssues,setBindingIssues]=useState<BindingIssue[]>([]);
  const [images, setImages] = useState<ImageSelection>({});
  const [savedImages, setSavedImages] = useState<Asset[]>([]);
  const [selected, setSelected] = useState<Attempt | undefined>();
  const artifactRestore=useArtifactRestore(client,`${toolId}/${actionId}/${draftRevision}`);
  const retryNative=async(id:string)=>{await client.preserveNative(id);const saved=await client.attempt(id);setSelected(current=>current?.id===id?saved:current);};
  const libraryArtifacts=useSavedArtifacts(api,snapshot,JSON.stringify([selected?.id,selected?.preservation,selected?.outputs.length]));
  const savedRefusal=object(object(selected?.observations)?.refusal);
  const savedBindingIssues=selected?.request.action===actionId && JSON.stringify(selected.request.params)===JSON.stringify(params) && JSON.stringify(selected.request.bindings??{})===JSON.stringify(encodePortBindings(portBindings).bindings) && savedRefusal ? parseBindingIssues({code:"binding",bindings:savedRefusal.bindings}) : [];
  const recent = snapshot.attempts;
  const [selectedId, setSelectedId] = useState<string | null>();
  const [busy, setBusy] = useState(false),
    [error, setError] = useState<string>();
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
        setImages(attempt.declaration.document.actions.find(a=>a.name===attempt.request.action)?.kind === "operation" ? {} : restored);
        setPortBindings(Object.fromEntries(Object.entries(attempt.request.bindings??{}).map(([port,items])=>[port,items.map((item,index)=>({...structuredClone(item),...(attempt.request.lineage?.[port]?.[index]?{origin:attempt.request.lineage[port]![index]!}:{} )}))])));
        setBindingIssues([]);
        submitted.current = null;
        setSelectedId(id);
        setSelected(attempt);
        setDrawer(null);
      })
      .catch((failure) => setError(String(failure)));
  }
  async function openExample(example: Example) {
    const candidates=snapshot.tools.filter(tool=>tool.declaration?.document.tool.name===example.declaration.document?.tool.name&&tool.declaration.document.actions.some(action=>action.name===example.request.action));
    const receiving=candidates.find(tool=>tool.id===example.request.tool)??candidates.find(tool=>tool.id===toolId)??(candidates.length===1?candidates[0]:undefined);
    if(!receiving?.declaration)throw Error("Choose an installed compatible tool before opening this example.");
    const current=receiving.declaration;
    setBusy(true);setDrawer(null);
    try{
      const bindings:PortBindings={};
      for(const [port,items] of Object.entries(example.request.bindings??{})){
        bindings[port]=[];
        for(const [index,item] of items.entries()){
          const candidate={...structuredClone(item),origin:{example:example.id,port,occurrence:index}};
          const resolved=await artifactRestore.choose(candidate,receiving.id);
          if(!resolved)return;
          bindings[port]!.push(resolved);
        }
      }
      setDraftRevision(value=>value+1);
      setToolId(receiving.id);setActionId(example.request.action);
      setParams(structuredClone(example.request.params));
      setImages(current.document.actions.find(action=>action.name===example.request.action)?.kind==="operation"?{}:restoreSelection(example.inputs));
      setPortBindings(bindings);setBindingIssues([]);setSelectedId(null);setSelected(undefined);
      setDraftExample(example);setError(undefined);submitted.current=null;
    }finally{setBusy(false);}
  }
  function chooseTool(id: string) {
    setDraftRevision((value) => value + 1);
    setDraftExample(undefined);
    setToolId(id);
    const action = firstActivity(snapshot.tools.find((t) => t.id === id)?.declaration
      ?.document.actions ?? []);
    setEmptyKind("task");
    setActionId(action?.name ?? "");
    setParams(initialParameters(action?.params_schema ?? {}));
    setImages({});
    setPortBindings({});setBindingIssues([]);setPortImporting(false);
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
    setPortBindings({});setBindingIssues([]);setPortImporting(false);
    setSelectedId(undefined);
    setSelected(undefined);
    setError(undefined);
    submitted.current = null;
  }
  async function run(retry = false) {
    if (busy) return;
    if (!retry && (!action || !declaration)) return;
    if (!retry && (cost.submission || (action?.cost.may_spend && !cost.canRun))) return;
    if(portImporting)return;
    setError(undefined);
    setBusy(true);
    try {
      if (retry || action?.cost.may_spend) {
        const attempt = await cost.submit();
        setSelectedId(attempt.id);
        setSelected(attempt);
        await updateSnapshot();
        return;
      }
      if (!action || !declaration) return;
      const portError=validatePorts(action.ports?.in??[],portBindings);if(portError){const invalid=(action.ports?.in??[]).find(port=>validatePorts([port],portBindings));if(invalid)document.querySelector<HTMLElement>(`[data-port="${CSS.escape(invalid.name)}"]`)?.focus();throw new Error(portError);}
      setBindingIssues([]);
      const {images: bindings, frames} = encodeSelection(
        declaration.document.contract, action.images ?? [], images,
      );
      const body = JSON.stringify({
        tool: toolId,
        action: actionId,
        revision: declaration.revision,
        params,
        images: bindings,
        ...encodePortBindings(portBindings),
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
            ...encodePortBindings(portBindings),
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
      if(failure instanceof ApiError)setBindingIssues(failure.bindings);
      setError(
        failure instanceof Error
          ? failure.message
          : "The attempt could not be submitted.",
      );
    } finally {
      setBusy(false);
    }
  }
  const estimateKey = JSON.stringify({toolId,actionId,revision:declaration?.revision,sessionId,restarting,
    draftRevision,estimateEditRevision,params,portBindings,images:Object.fromEntries(Object.entries(images).map(([slot,items]) =>
      [slot,items.map(image => ({id:image.id,coordinates:image.coordinates}))]))});
  const cost = useEstimateDraft(api, estimateKey, typeof sessionId === "string" && !restarting ? sessionId : undefined, () => {
    if (!declaration || !action?.cost.may_spend) throw new Error("Choose a paid action before estimating.");
    const cap = positiveCap(params.max_spend_usd);
    if (cap === undefined) throw new Error("Choose a positive spending cap before estimating.");
    const portError=validatePorts(action.ports?.in??[],portBindings);if(portError){const invalid=(action.ports?.in??[]).find(port=>validatePorts([port],portBindings));if(invalid)document.querySelector<HTMLElement>(`[data-port="${CSS.escape(invalid.name)}"]`)?.focus();throw new Error(portError);}
    const bindings = encodeSelection(declaration.document.contract, action.images ?? [], images);
    return {id:crypto.randomUUID(),tool:toolId,action:actionId,revision:declaration.revision,
      params:{...structuredClone(params),max_spend_usd:cap},...bindings,...encodePortBindings(portBindings)};
  }, updateSnapshot);
  let costSession;
  try { costSession = parseToolSession(declaration?.tool_session); } catch { /* No fabricated allowance. */ }
  const capText = typeof params.max_spend_usd === "string" || typeof params.max_spend_usd === "number" ? String(params.max_spend_usd) : "";
  const noRun = selected?.execution === "rejected";
  const setupTool = snapshot.tools.find(tool => tool.id === setupToolId);
  const setupDeclaration = setupTool?.declaration;
  const setupAction = setupDeclaration?.document.actions.find(action => action.name === (drawer?.action ?? actionId));
  const setupRestarting = restartingTools.has(setupToolId);
  const working =
    selected &&
    (["submitted", "running", "unconfirmed", "cancelling"].includes(
      selected.execution,
    ) ||
      selected.preservation.contract === "pending");
  return (
    <WorkspaceFrame connected connection="Connected locally" controls={<Shutdown api={api} />}
      toolPicker={<Picker compact label="Tool" value={toolId} onChange={chooseTool}
        choices={snapshot.tools.map(t => ({id:t.id,title:t.declaration?.document.tool.name ?? t.id,
          description:t.error ?? `Version ${t.declaration?.document.tool.version ?? "unknown"}`}))} />}>
    <div className={styles.workspace}>
      <div className={styles.toolContext}>
        <div className={styles.toolHeading}>
          <h1>{declaration?.document.tool.name ?? toolId ?? "Your workshop"}</h1>
          <p className={styles.note}>{declaration?.document.tool.purpose ?? "Tool setup is unavailable."}</p>
        </div>
        <nav className={styles.library} aria-label="Workspace library">
          <Button onClick={event => setDrawer({opener:event.currentTarget,view:"history"})}>History</Button>
          <Button onClick={event => setDrawer({opener:event.currentTarget,view:"examples"})}>Examples</Button>
          <Button onClick={event => setDrawer({opener:event.currentTarget,view:"cleanup"})}>Clean up library</Button>
          {tool && <Button onClick={event => setDrawer({opener:event.currentTarget,view:"setup"})}>Tool setup</Button>}
        </nav>
      </div>
      <ActivityPicker actions={declaration?.document.actions ?? []}
        groups={declaration?.document.vocabulary?.group ?? []} kind={activityKind} value={actionId}
        disabled={!declaration} onAction={chooseAction} onKind={kind=>{
          setEmptyKind(kind);
          chooseAction(declaration?.document.actions.find(action=>action.kind===kind)?.name ?? "");
        }}/>
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
            onChangeCapture={() => setEstimateEditRevision(value => value + 1)}
            tabIndex={0}
            key={draftRevision}
          >
            {draftExample && (
              <p className={styles.note}>
                From example: {draftExample.name}{draftExample.workspace_dependent?" · Workspace-dependent":""}
                {draftExample.notes ? ` · ${draftExample.notes}` : ""}
              </p>
            )}
            {action?.kind==="operation" && action.ports && <PortEditor key={`ports/${toolId}/${actionId}/${draftRevision}/${declaration?.revision}`} ports={action.ports.in} resolveArtifact={binding=>artifactRestore.choose(binding,toolId)} value={portBindings} onBusy={setPortImporting} onChange={value=>{setPortBindings(value);setBindingIssues([]);}} assets={savedImages} artifacts={libraryArtifacts.sources.flatMap((source,index)=>{const binding=transferBinding(source,declaration!.document.tool.name);return binding?[{id:String(index),label:`${source.port} item ${(source.index??0)+1} · ${source.attempt?.slice(0,8)} · ${String(source.reference.type)}`,reference:source.reference,binding}]:[];})} issues={cost.state.kind==="error"?cost.state.bindings??bindingIssues:bindingIssues.length?bindingIssues:savedBindingIssues} upload={async file=>{const asset=await assets.upload(file);setSavedImages(await assets.list());return asset;}} disabled={busy||!!working||portImporting} />}
            {action?.kind==="operation" && <><p role="status">{libraryArtifacts.loading?"Loading saved artifacts…":libraryArtifacts.error??"Select a saved artifact to check its availability. Run still validates compatibility."}</p>{libraryArtifacts.more&&<Button type="button" onClick={libraryArtifacts.loadMore}>Load older artifacts</Button>}</>}
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
            {action?.cost.may_spend && <CostReview cap={capText} state={cost.state}
              onCapChange={value => setParams(current => ({...current,max_spend_usd:value}))}
              onEstimate={() => { void cost.estimate(); }}
              onSetup={opener => setDrawer({opener,view:"setup"})}
              disabled={busy || !!cost.submission || restarting || typeof sessionId !== "string"}
              {...(costSession ? {session:costSession} : {})} />}
            {cost.submission === "uncertain" && <div role="status">
              <p>The submission was not acknowledged. Retry sends the original reviewed inputs with
                the same run ID; it cannot create a second attempt. Your current edits are kept separately.</p>
              <Button busy={busy} onClick={() => { void run(true); }}>Retry submission</Button>
            </div>}
            {error && <p role="alert">{error}</p>}
          </div>
          <div className={styles.run}>
            <p>
              {action?.cost.may_spend
                ? "Paid action · Review the estimate before Run"
                : "Free action · Inputs are saved locally"}
            </p>
            <Button
              type="submit"
              busy={busy}
              disabled={
                portImporting || !action ||
                (action.cost.may_spend && !cost.canRun) ||
                !!cost.submission ||
                restarting ||
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
                <RunOutcome attempt={selected}
                  paid={selected.declaration?.document?.actions?.some(action => action.name === selected.request.action && action.cost.may_spend) ?? false}
                  onSetup={event => setDrawer({opener:event.currentTarget,
                  view:"setup",tool:selected.request.tool,action:selected.request.action})} />
                {!noRun && <>
                <ArtifactOutputs attempt={selected} restore={async index=>{const source=savedSources(selected).find(source=>source.index===index);if(!source)return;const binding=transferBinding(source,selected.declaration.document.tool.name);if(!binding)return;if(await artifactRestore.choose(binding,selected.request.tool))return client.artifact(selected.id,index);}} load={assets.preview} inspect={index=>client.artifact(selected.id,index)} disabled={busy||!!working} onSend={index=>setSendSource(savedSources(selected).find(source=>source.index===index))} />
                <NativePreservation key={`native/${selected.id}`} attempt={selected} retry={retryNative} />
                {(!artifactOccurrences(selected.result).length || selected.outputs.some(output=>output.role!=="preview")) && <SavedComparison
                  attempt={selected}
                  client={client}
                  load={assets.preview}
                  revision={snapshot}
                />}
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
      {artifactRestore.drawer}
      <Drawer open={!!sendSource} title="Send artifact to" description="Choose a destination port. This fills a new draft without running any tool." onClose={()=>setSendSource(undefined)}>
        {sendSource && compatibleDestinations(sendSource,snapshot.tools).map(destination=><Button key={`${destination.tool}/${destination.action}/${destination.port}`} onClick={async()=>{
          const target=snapshot.tools.find(tool=>tool.id===destination.tool)?.declaration;
          const action=target?.document.actions.find(action=>action.name===destination.action);
          if(!target||!action)return;
          const candidate=transferBinding(sendSource,target.document.tool.name);if(!candidate)return;
          setSendSource(undefined);
          const binding=await artifactRestore.choose(candidate,destination.tool);if(!binding)return;
          chooseTool(destination.tool);setActionId(destination.action);setParams(initialParameters(action.params_schema));
          setPortBindings({[destination.port]:[binding]});setSelectedId(null);setSelected(undefined);setSendSource(undefined);
        }}>{destination.title}</Button>)}
        {sendSource&&!compatibleDestinations(sendSource,snapshot.tools).length&&<p>No installed operation declares a compatible receiving port. No tool was run.</p>}
      </Drawer>
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
            ? `${setupDeclaration?.document.tool.name ?? setupToolId} · Requirements, folders and session spending`
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
            {setupRestarting ? <p role="status">Checking the replacement session…</p> : setupDeclaration && setupAction ? (
              <ToolSetup
                key={`${setupToolId}:${setupAction.name}:${object(setupDeclaration.tool_session)?.id}`}
                tool={setupDeclaration.document.tool}
                action={setupAction}
                loadSession={loadToolSession}
                directoryGrants={setupTool?.directory_grants ?? null}
              />
            ) : (
              <p role="status">Setup is unavailable because this tool or action is no longer available.</p>
            )}
            {setupTool && <RestartTool key={setupToolId} busy={setupTool.busy || setupRestarting} onRestart={restartToolSession} />}
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
          drawer?.view === "inspect" && selected && <Inspector key={selected.id} attempt={selected} reports={reports} retryNative={retryNative} />
        )}
      </Drawer>
    </div>
    </WorkspaceFrame>
  );
}

import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { ApiError, type LocalApi } from "../../api/client";
import { estimateClient, estimateDeadline, type Preparation } from "../../api/estimates";
import { workspaceClient, type Submission } from "../../api/workspace";
import type { CostState } from "./CostReview";

type Draft = { key: string; request: Submission; receipt?: Preparation };
export function useEstimateDraft(api: LocalApi, key: string, session: string | undefined,
  makeRequest: () => Submission, refresh: () => Promise<void>) {
  const client = useMemo(() => estimateClient(api), [api]);
  const workspace = useMemo(() => workspaceClient(api), [api]);
  const submitted = useRef<Draft | undefined>(undefined);
  const sending = useRef(false);
  const [submission, setSubmission] = useState<"sending" | "uncertain">();
  const owned = useRef<Draft | undefined>(undefined);
  const currentKey = useRef(key), alive = useRef(true), requesting = useRef(false);
  const [pending, setPending] = useState<string>();
  const [display, setDisplay] = useState<{key:string; state:CostState}>({key,state:{kind:"idle"}});
  // Disposal is best effort. The host TTL still bounds copies if the browser disconnects.
  const discard = (draft: Draft) => { void client.discard(draft.request.id).catch(() => {}); };
  useLayoutEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      if (owned.current) { discard(owned.current); owned.current = undefined; }
    };
  }, [client]);
  useLayoutEffect(() => {
    currentKey.current = key;
    if (owned.current && owned.current.key !== key) {
      discard(owned.current); owned.current = undefined;
    }
  }, [key, client]);
  useEffect(() => {
    if (display.key !== key || display.state.kind !== "quoted" || !owned.current?.receipt) return;
    const draft = owned.current;
    const deadline = estimateDeadline({preparation:draft.receipt!,estimate:display.state.estimate});
    const expire = () => {
      if (alive.current && owned.current === draft && currentKey.current === key) {
        owned.current = undefined; discard(draft);
        setDisplay({key,state:{kind:"stale"}});
      }
    };
    const visible = () => { if (Date.now() >= deadline) expire(); };
    const timer = window.setTimeout(expire, Math.max(0, deadline - Date.now()));
    document.addEventListener("visibilitychange", visible);
    return () => { clearTimeout(timer); document.removeEventListener("visibilitychange", visible); };
  }, [display, key, client]);
  async function estimate() {
    if (requesting.current || submitted.current || !session) return;
    requesting.current = true;
    setPending(key);
    setDisplay({key,state:{kind:"stale"}});
    let draft: Draft | undefined;
    const current = () => alive.current && currentKey.current === key && owned.current === draft;
    try {
      if (owned.current?.receipt && owned.current.receipt.expires_ms <= Date.now()) {
        discard(owned.current); owned.current = undefined;
      }
      draft = owned.current;
      if (!draft || draft.key !== key) {
        draft = {key,request:makeRequest()}; owned.current = draft;
      }
      draft.receipt ??= await client.prepare(draft.request);
      if (!current()) { discard(draft); return; }
      const quote = await client.estimate(draft.receipt, session);
      if (!current()) { discard(draft); return; }
      // Refresh ordered workspace observations before displaying this quote's ledger.
      await refresh().catch(() => {});
      if (!current()) { discard(draft); return; }
      if (estimateDeadline(quote) <= Date.now()) {
        owned.current = undefined; discard(draft);
        setDisplay({key,state:{kind:"stale"}});
      } else setDisplay({key,state:{kind:"quoted",estimate:quote.estimate}});
    } catch (failure) {
      if (alive.current && currentKey.current === key)
        setDisplay({key,state:{kind:"error",message:failure instanceof Error ? failure.message : "The estimate could not be read.",...(failure instanceof ApiError?{bindings:failure.bindings}:{})}});
      else if (draft) discard(draft);
    } finally {
      requesting.current = false;
      if (alive.current) setPending(undefined);
    }
  }
  const state: CostState = pending !== undefined ? {kind:"pending",stale:pending !== key}
    : display.key === key ? display.state
    : {kind:display.state.kind === "idle" ? "idle" : "stale"};
  const canRun = !submission && !!session && state.kind === "quoted" && state.estimate.allowed &&
    owned.current?.key === key && !!owned.current.receipt &&
    estimateDeadline({preparation:owned.current.receipt,estimate:state.estimate}) > Date.now();
  async function submit() {
    if (sending.current) throw new Error("This submission is already in progress.");
    let draft = submitted.current;
    if (!draft) {
      if (!canRun || !owned.current) throw new Error("Estimate these inputs before running.");
      draft = owned.current;
      // Ownership transfers to submission: edits and quote expiry must not delete its inputs.
      submitted.current = draft; owned.current = undefined;
      setDisplay({key,state:{kind:"stale"}});
    }
    sending.current = true;
    setSubmission("sending");
    try {
      const attempt = await workspace.submitPrepared(draft.request);
      submitted.current = undefined;
      if (alive.current) setSubmission(undefined);
      return attempt;
    } catch (failure) {
      // Definitive host refusals allow a new estimate. Transport/server uncertainty retains
      // the exact request ID; only an explicit retry can send that same submission again.
      if (failure instanceof ApiError && [400,404,409,422].includes(failure.status)) {
        submitted.current = undefined; discard(draft);
        if (alive.current) setSubmission(undefined);
      } else if (alive.current) setSubmission("uncertain");
      throw failure;
    } finally { sending.current = false; }
  }
  return {state, estimate, canRun, submit, submission};
}

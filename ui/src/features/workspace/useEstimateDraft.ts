import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { LocalApi } from "../../api/client";
import { estimateClient, estimateDeadline, type Preparation } from "../../api/estimates";
import type { Submission } from "../../api/workspace";
import type { CostState } from "./CostReview";

type Draft = { key: string; request: Submission; receipt?: Preparation };
export function useEstimateDraft(api: LocalApi, key: string, session: string | undefined,
  makeRequest: () => Submission, refresh: () => Promise<void>) {
  const client = useMemo(() => estimateClient(api), [api]);
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
    if (requesting.current || !session) return;
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
        setDisplay({key,state:{kind:"error",message:failure instanceof Error ? failure.message : "The estimate could not be read."}});
      else if (draft) discard(draft);
    } finally {
      requesting.current = false;
      if (alive.current) setPending(undefined);
    }
  }
  const state: CostState = pending !== undefined ? {kind:"pending",stale:pending !== key}
    : display.key === key ? display.state
    : {kind:display.state.kind === "idle" ? "idle" : "stale"};
  return {state, estimate};
}

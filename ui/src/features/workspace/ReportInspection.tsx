import { useEffect, useState, useSyncExternalStore } from "react";
import type { Attempt } from "../../api/workspace";
import { ReportPanel } from "./ReportPanel";
import { ReportOwner } from "./report-owner";
/** Mounted only for the open drawer's report tab; the owner outlives this component. */
export function ReportInspection({ attempt, owner }: { attempt: Attempt; owner: ReportOwner }) {
  const choices = attempt.declaration.document.actions.find(action => action.name === attempt.request.action)?.renders ?? [];
  const [selection, setSelection] = useState(choices[0]?.name ?? "");
  const snapshot = useSyncExternalStore(owner.subscribe, owner.snapshot);
  useEffect(() => {
    owner.inspect();
    const refresh = () => { if (document.visibilityState === "visible" && owner.snapshot().view?.phase !== "exited") void owner.refresh(); };
    const activity = () => { if (document.visibilityState === "visible") void owner.activity(); };
    const visible = () => { refresh(); activity(); };
    const poll = window.setInterval(refresh, 500);
    const heartbeat = window.setInterval(activity, 30_000);
    document.addEventListener("visibilitychange", visible);
    refresh();
    return () => {
      window.clearInterval(poll); window.clearInterval(heartbeat);
      document.removeEventListener("visibilitychange", visible);
      void owner.dismiss();
    };
  }, [owner, attempt.id, selection]);
  return <ReportPanel attempt={attempt.id} choices={choices} selection={selection} snapshot={snapshot}
    onSelect={setSelection}
    onOpen={() => void owner.open(attempt.id, selection)} onClose={() => void owner.close()}
    onRecover={() => void owner.recover()} onForce={() => void owner.force(true)} />;
}

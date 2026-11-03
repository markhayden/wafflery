import {NativePreservation} from "./NativePreservation";
import styles from "./Evidence.module.css";
import { savedInvocation } from "./invocation";
import { ReportInspection } from "./ReportInspection";
import type { ReportOwner } from "./report-owner";
import { useState } from "react";
import type { Attempt } from "../../api/workspace";
import { Button } from "../../components/Button";
export function Inspector({ attempt, reports, retryNative }: { attempt: Attempt; reports: ReportOwner; retryNative?:((id:string)=>Promise<unknown>)|undefined }) {
  const [view, setView] = useState<"inputs" | "result" | "call" | "invocation" | "journal" | "session" | "reports">(
    "inputs",
  );
  const invocation = savedInvocation(attempt.result);
  const data =
    view === "inputs"
      ? { parameters: attempt.request.params, images: attempt.inputs,...(attempt.request.source_bundle?{source_bundle:attempt.request.source_bundle}:{}) }
      : view === "result"
        ? attempt.raw_response ?? attempt.result
        : view === "invocation"
          ? invocation.state === "resolved" ? invocation.value : null
        : view === "call"
          ? attempt.wire
          : view === "session"
            ? { before: attempt.declaration.tool_session, observations: attempt.observations }
            : attempt.journal;
  return (
    <div className={styles.inspector}>
      <nav aria-label="Run evidence">
        <Button
          aria-pressed={view === "inputs"}
          onClick={() => setView("inputs")}
        >
          Saved inputs
        </Button>
        <Button
          aria-pressed={view === "result"}
          onClick={() => setView("result")}
        >
          Raw result
        </Button>
        <Button aria-pressed={view === "call"} onClick={() => setView("call")}>
          Recorded call
        </Button>
        <Button aria-pressed={view === "invocation"} onClick={() => setView("invocation")}>
          Resolved invocation
        </Button>
        <Button
          aria-pressed={view === "journal"}
          onClick={() => setView("journal")}
        >
          Journal
        </Button>
        <Button aria-pressed={view === "session"} onClick={() => setView("session")}>
          Session
        </Button>
        <Button aria-pressed={view === "reports"} onClick={() => setView("reports")}>Reports</Button>
      </nav>
      {view === "reports" ? <ReportInspection attempt={attempt} owner={reports} /> : <>
      <p>
        Tool outcome: {attempt.execution}. Contract evidence:{" "}
        {attempt.preservation.contract}.
      </p>
      {view === "invocation" && <p>{invocation.state === "resolved"
        ? "The tool’s resolved arguments, effective settings and requirement names for this saved run."
        : invocation.state === "unresolved"
          ? "The tool rejected the inputs before resolving an invocation."
          : "No resolved invocation is available for this saved attempt."}</p>}
      {(view !== "invocation" || invocation.state !== "unavailable") &&
        <pre aria-label={view === "invocation" ? "Resolved invocation" : "Saved evidence"}>
          {JSON.stringify(data, null, 2) ?? "No evidence available yet."}
        </pre>}

      <NativePreservation attempt={attempt} retry={retryNative}/>
      <p>Tool reports remain dependent on the tool's saved report files.</p>
      </>}
      {attempt.notices.map((notice, index) => (
        <p key={index}>{notice}</p>
      ))}
    </div>
  );
}

import { ReportInspection } from "./ReportInspection";
import type { ReportOwner } from "./report-owner";
import { useState } from "react";
import type { Attempt } from "../../api/workspace";
import { Button } from "../../components/Button";
export function Inspector({ attempt, reports }: { attempt: Attempt; reports: ReportOwner }) {
  const [view, setView] = useState<"inputs" | "result" | "call" | "journal" | "session" | "reports">(
    "inputs",
  );
  const data =
    view === "inputs"
      ? { parameters: attempt.request.params, images: attempt.inputs }
      : view === "result"
        ? attempt.raw_response ?? attempt.result
        : view === "call"
          ? attempt.wire
          : view === "session"
            ? { before: attempt.declaration.tool_session, observations: attempt.observations }
            : attempt.journal;
  return (
    <div className="inspector">
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
      <pre aria-label="Saved evidence">
        {JSON.stringify(data, null, 2) ?? "No evidence available yet."}
      </pre>
      <p>
        Native dependencies and tool reports are not included in the saved evidence.
      </p>
      </>}
      {attempt.notices.map((notice, index) => (
        <p key={index}>{notice}</p>
      ))}
    </div>
  );
}

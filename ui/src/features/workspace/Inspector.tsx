import { useState } from "react";
import type { LocalApi } from "../../api/client";
import type { Attempt } from "../../api/workspace";
import { Button } from "../../components/Button";
import { CodeBlock } from "../../components/CodeBlock";
import { ChoiceCard, ChoiceCards } from "../../components/ChoiceCard";
import { CommandHandoff } from "./CommandHandoff";
import { NativePreservation } from "./NativePreservation";
import { ReportInspection } from "./ReportInspection";
import type { ReportOwner } from "./report-owner";
import { savedInvocation } from "./invocation";
import styles from "./Evidence.module.css";

type Tab = "command" | "inputs" | "result" | "advanced";
type Record = "call" | "invocation" | "journal" | "session" | "reports";

/** The records a person rarely needs, each with what it is, so the names mean something. */
const RECORDS: { id: Record; title: string; about: string }[] = [
  { id: "call", title: "Recorded call", about: "The exact request Wafflery sent to the tool." },
  { id: "invocation", title: "Resolved invocation", about: "The arguments and settings the tool ran with, defaults filled in." },
  { id: "journal", title: "Journal", about: "The tool's own step-by-step record of the run." },
  { id: "session", title: "Session", about: "The tool session's spending before this run, and what the run reported." },
  { id: "reports", title: "Reports", about: "Report pages the tool wrote for this run." },
];

/** Run details: how to run it again yourself first, then what went in and what came out, with
 * the full evidence one tab further. */
export function Inspector({ attempt, reports, retryNative, api }: { attempt: Attempt; reports: ReportOwner; retryNative?: ((id: string) => Promise<unknown>) | undefined; api?: LocalApi | undefined }) {
  const [tab, setTab] = useState<Tab>(api ? "command" : "inputs");
  const [record, setRecord] = useState<Record>("call");
  const invocation = savedInvocation(attempt.result);
  const action = attempt.declaration.document?.actions.find((candidate) => candidate.name === attempt.request.action);
  const evidence = record === "invocation" ? (invocation.state === "resolved" ? invocation.value : null)
    : record === "call" ? attempt.wire
    : record === "session" ? { before: attempt.declaration.tool_session, observations: attempt.observations }
    : attempt.journal;
  const tabs: { id: Tab; title: string }[] = [
    ...(api ? [{ id: "command" as const, title: "Command" }] : []),
    { id: "inputs", title: "Inputs" }, { id: "result", title: "Result" }, { id: "advanced", title: "Advanced" },
  ];
  return (
    <div className={styles.inspector}>
      <nav aria-label="Run evidence">
        {tabs.map((item) => <Button key={item.id} aria-pressed={tab === item.id} onClick={() => setTab(item.id)}>{item.title}</Button>)}
      </nav>

      {tab === "command" && api && <>
        <p>Run this same call from your terminal. It uses the inputs and settings saved with this run.</p>
        <CommandHandoff id={attempt.id} api={api} tool={attempt.declaration.document?.tool} maySpend={action?.cost.may_spend}
          revision={JSON.stringify([attempt.execution, !!attempt.result, attempt.preservation.contract])} />
      </>}

      {tab === "inputs" && <>
        <p>The images and settings this run was given.</p>
        <CodeBlock fill label="Saved evidence" copyLabel="Copy inputs"
          value={{ parameters: attempt.request.params, images: attempt.inputs, ...(attempt.request.source_bundle ? { source_bundle: attempt.request.source_bundle } : {}) }} />
      </>}

      {tab === "result" && <>
        <p>Everything the tool sent back, as it arrived.</p>
        <CodeBlock fill label="Saved evidence" copyLabel="Copy result" value={attempt.raw_response ?? attempt.result ?? "No result yet."} />
      </>}

      {tab === "advanced" && <>
        <p>Tool outcome: {attempt.execution}. Contract evidence: {attempt.preservation.contract}.</p>
        <ChoiceCards label="Evidence records" layout="list">
          {RECORDS.map((item) => <ChoiceCard key={item.id} title={item.title} description={item.about} selected={record === item.id}
            onClick={() => setRecord(item.id)} />)}
        </ChoiceCards>
        {record === "reports" ? <ReportInspection attempt={attempt} owner={reports} /> : <>
          {record === "invocation" && <p>{invocation.state === "resolved"
            ? "The tool’s resolved arguments, effective settings and requirement names for this saved run."
            : invocation.state === "unresolved"
              ? "The tool rejected the inputs before resolving an invocation."
              : "No resolved invocation is available for this saved attempt."}</p>}
          {(record !== "invocation" || invocation.state !== "unavailable") &&
            <CodeBlock fill label={record === "invocation" ? "Resolved invocation" : "Saved evidence"} copyLabel="Copy this record"
              value={record === "invocation" ? evidence : evidence ?? "No evidence available yet."} />}
        </>}
        <NativePreservation attempt={attempt} retry={retryNative} />
        <p>Tool reports remain dependent on the tool's saved report files.</p>
      </>}

      {attempt.notices.map((notice, index) => <p key={index}>{notice}</p>)}
    </div>
  );
}

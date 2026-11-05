import { usd } from "../../api/tool-session";
import type { Attempt } from "../../api/workspace";
import { isReadiness } from "../../api/workspace";
import { object, type Json, type Requirement } from "../actions/model";
type Record = Pick<Attempt,"execution" | "result" | "observations" | "raw_response"> & {paid?:boolean};
type Spend = {usd:number; unresolved:number; basis:"measured" | "estimated"};
const amount = (value:Json | undefined): value is number => typeof value === "number" && Number.isFinite(value) && value >= 0;
const guidance: {[code:string]:{title:string; guidance:string; setup:boolean}} = {
  input:{title:"Input refused by tool.",guidance:"Review the tool’s reason and any suggested remedy before starting another run. Original inputs have not been resized.",setup:false},
  stale_estimate:{title:"Estimate no longer matches.",guidance:"The checked cost no longer matches these inputs. Check again, or run without it.",setup:false},
  credential:{title:"Provider rejected authentication.",guidance:"Check the tool’s credential setup, then run again. No retry was started.",setup:true},
  cap:{title:"Spending cap reached.",guidance:"Review the cap and request a new estimate. The cap has not been increased.",setup:false},
  budget:{title:"Session budget reached.",guidance:"Review the tool’s session spending before another attempt. Its budget has not been reset.",setup:true},
};
export function runOutcome(attempt: Record) {
  const paid = attempt.paid === true;
  const result = object(attempt.result), observations = object(attempt.observations);
  const refusal = object(observations?.refusal);
  const noRun = attempt.execution === "rejected" && !result;
  const waiting = ["submitted","running","cancelling"].includes(attempt.execution) || result?.status === "running";
  const terminal = result && ["completed","cancelled","invalid","error"].includes(String(result.status));
  const error = object(noRun ? refusal?.error : result?.error);
  const code = typeof error?.code === "string" ? error.code : undefined;
  const detail = error?.reason ?? (noRun ? object(object(attempt.raw_response)?.error)?.message : undefined);
  const reason = typeof detail === "string" ? detail : undefined;
  const readiness = {state:"needs_setup",requirements:refusal?.requirements};
  const requirements: Requirement[] = noRun && isReadiness(readiness) ? readiness.requirements : [];
  const setup = noRun ? refusal?.reason === "not_ready" : !!(code && guidance[code]?.setup);
  const reported = object(result?.spend);
  let spend: Spend | undefined;
  if (noRun && observations?.spent_usd === 0) spend = {usd:0,unresolved:0,basis:"measured"};
  else if (terminal && !waiting && amount(reported?.usd) && amount(reported.unresolved_usd) &&
    reported.unresolved_usd <= reported.usd && (reported.basis === "measured" || reported.basis === "estimated"))
    spend = {usd:reported.usd,unresolved:reported.unresolved_usd,basis:reported.basis};
  const action = code ? guidance[code] : undefined;
  return {kind:noRun ? "no-run" : waiting ? "waiting" : terminal ? "finished" : "unknown",
    title:noRun ? "No run created." : waiting ? "Run in progress." : !terminal ? "Outcome unconfirmed."
      : action?.title ?? (result.status === "completed" ? "Run completed." : result.status === "cancelled" ? "Run cancelled." : "Run failed."),
    guidance:noRun ? (setup ? (paid ? "Complete tool setup before estimating again." : "Complete tool setup before another run.") : "Correct the request before starting a new attempt.")
      : waiting ? undefined : !terminal ? "Inspect the saved evidence before starting another attempt."
      : code === "credential" && !paid ? "Check the tool’s credential setup before another run. No retry was started."
      : action?.guidance ?? (result.status === "completed" ? undefined : paid
        ? "Review the result, then Run again if you want to. No retry was started."
        : "Review the result before starting another run. No retry was started."),
    reason,code,setup,requirements,spend};
}

/** An upper bound must not be rounded down by the normal currency formatter. */
export function upperUsd(value: number): string {
  const scaled = value * 1_000_000;
  if (!Number.isFinite(scaled)) return usd(value);
  let upper = Math.ceil(scaled) / 1_000_000;
  if (upper < value) upper += 0.000001;
  return usd(upper);
}

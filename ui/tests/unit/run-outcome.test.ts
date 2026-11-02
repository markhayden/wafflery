import { test, expect } from "bun:test";
import { runOutcome, upperUsd } from "../../src/features/workspace/run-outcome";
import type { Json } from "../../src/features/actions/model";
const input = (result:Json, execution = "error", observations:Json = {}) => ({result,execution,observations,raw_response:null,paid:true});
const result = (code:string | null, usd = 0, unresolved = 0, status = "error") => ({status,
  error:code ? {code,reason:"Reported by the tool",retryable:true} : null,
  spend:{usd,unresolved_usd:unresolved,basis:"measured"}});
test("paid failure guidance distinguishes setup, cap, budget and stale estimates", () => {
  for (const [code,expected] of [["credential","authentication"],["cap","cap"],["budget","budget"],["stale_estimate","Estimate"]]) {
    const view = runOutcome(input(result(code!)));
    expect(view.title).toContain(expected!);
    expect(view.reason).toBe("Reported by the tool");
    expect(view.guidance).toBeTruthy();
  }
  expect(runOutcome(input(result("credential"))).setup).toBe(true);
  expect(runOutcome(input(result("budget"))).setup).toBe(true);
  expect(runOutcome(input(result("stale_estimate"))).setup).toBe(false);
});
test("failed and cancelled runs retain actual and unresolved charges without treating an upper bound as exact", () => {
  for (const status of ["error","cancelled"]) {
    const unknown = runOutcome(input(result(null,0.12,0.12,status),status));
    expect(unknown.spend).toEqual({usd:0.12,unresolved:0.12,basis:"measured"});
    const known = runOutcome(input(result(null,0.08,0,status),status));
    expect(known.spend).toEqual({usd:0.08,unresolved:0,basis:"measured"});
  }
});
test("a no-run setup refusal stays separate from an allocated credential error", () => {
  const refused = runOutcome(input(null,"rejected",{refusal:{reason:"not_ready",run:null,
    requirements:[{kind:"credential",name:"EXAMPLE_KEY",state:"missing",setup:"Configure outside the playground."}]},spent_usd:0}));
  expect(refused.title).toBe("No run created.");
  expect(refused.setup).toBe(true);
  expect(refused.spend?.usd).toBe(0);
  expect(refused.requirements).toHaveLength(1);
  expect(runOutcome(input(result("credential"))).title).not.toBe(refused.title);
});
test("absent, contradictory and unfinished spending remains unavailable rather than zero", () => {
  for (const spend of [null,{}, {usd:-1,unresolved_usd:0,basis:"measured"},
    {usd:0.08,unresolved_usd:0.12,basis:"measured"}, {usd:0,unresolved_usd:0,basis:"unknown"}]) {
    expect(runOutcome(input({status:"error",spend})).spend).toBeUndefined();
  }
  expect(runOutcome(input(result(null,0,0,"running"),"running")).spend).toBeUndefined();
  expect(runOutcome(input(null,"unconfirmed")).spend).toBeUndefined();
});

test("upper-bound amounts round upward instead of understating an unresolved charge", () => {
  expect(upperUsd(0.1234564)).toBe("$0.123457");
  expect(upperUsd(0.00000001)).toBe("$0.000001");
  expect(upperUsd(0.12)).toBe("$0.12");
});

test("free failure recovery does not ask for an estimate control that is unavailable", () => {
  const view = runOutcome({...input(result(null,0,0,"cancelled"),"cancelled"),paid:false});
  expect(view.guidance).not.toContain("estimate");
  expect(view.guidance).toContain("another run");
});

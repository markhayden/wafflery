import { expect, test } from "bun:test";
import { runBlocker, type RunState } from "../../src/features/workspace/runBlocker";

const clear: RunState = {
  tool: "rascal", chainClaimed: false, importing: false, unresolved: 0, submitting: false, restarting: false,
  readiness: { state: "ready", requirements: [] }, toolBusy: false,
};

test("nothing blocks a ready form", () => {
  expect(runBlocker(clear)).toBeUndefined();
});

test("every blocking condition says what to resolve", () => {
  const cases: [Partial<RunState>, string][] = [
    [{ restarting: true }, "Restarting rascal…"],
    [{ readiness: { state: "unavailable", requirements: [{ kind: "model", name: "fillers.bria-erase", state: "missing" }] } },
      "rascal isn't set up for this yet: fillers.bria-erase is missing. Open Tool setup to fix it."],
    [{ readiness: { state: "unavailable", requirements: [] } }, "rascal can't run this action right now. Open Tool setup to see why."],
    [{ selected: { execution: "running", saving: false } }, "A run is in progress. Wait for it to finish, or cancel it."],
    [{ selected: { execution: "submitted", saving: false } }, "A run is in progress. Wait for it to finish, or cancel it."],
    [{ selected: { execution: "cancelling", saving: false } }, "Cancelling the last run…"],
    [{ selected: { execution: "unconfirmed", saving: false } }, "The last run wasn't confirmed. Retry or discard it in the result panel first."],
    [{ selected: { execution: "completed", saving: true } }, "Saving the last run's evidence…"],
    [{ toolBusy: true }, "rascal is busy with another run. Run is available when it finishes."],
    [{ toolBusy: true, toolHeld: true },
      "rascal is still reserved by a run whose result couldn't be saved, so Wafflery won't start another. Quit Wafflery and open it again to free it."],
    [{ submitting: true }, "Sending the run…"],
    [{ importing: true }, "Adding an input…"],
    [{ unresolved: 1 }, "One input from the bundle isn't in your library yet. Choose it above to run."],
    [{ unresolved: 2 }, "2 inputs from the bundle aren't in your library yet. Choose them above to run."],
    [{ chainClaimed: true }, "This step of the bundle already has its run. Choose the next step to continue."],
    [{ missingStep: "Choose the image to run" }, "Choose the image to run"],
  ];
  for (const [change, says] of cases) expect(runBlocker({ ...clear, ...change })).toBe(says);
});

test("an optional requirement never blocks, and a finished run does not either", () => {
  expect(runBlocker({ ...clear, readiness: { state: "ready", requirements: [
    { kind: "credential", name: "ANTHROPIC_API_KEY", state: "missing", optional: true, for: "checking the result" }] } })).toBeUndefined();
  expect(runBlocker({ ...clear, selected: { execution: "completed", saving: false } })).toBeUndefined();
});

/** Why Run is unavailable, in words a person can act on. Every condition that disables Run has a
 * sentence here, so the button is never greyed out without a reason beside it. */
import { blocking } from "./checks";
import type { Requirement } from "../actions/model";

export type RunState = {
  /** The tool's name as people see it ("rascal"). */
  tool: string;
  /** This bundle replay step already has its run. */
  chainClaimed: boolean;
  /** An input is still being added. */
  importing: boolean;
  /** Bundle inputs not found in the library yet. */
  unresolved: number;
  /** The run is being sent. */
  submitting: boolean;
  /** The tool is restarting. */
  restarting: boolean;
  /** The action's readiness, as the tool described it. */
  readiness: { state: string; requirements: Requirement[] } | undefined;
  /** The selected run's execution and whether its evidence is still being saved. */
  selected?: { execution: string; saving: boolean } | undefined;
  /** The tool is serving another run. */
  toolBusy: boolean;
  /** The tool is still reserved though none of its runs is in progress: the host kept it after a
   * run whose result it could not save, and frees it only when it restarts. */
  toolHeld?: boolean;
  /** The first required input still missing, already in words ("Choose the image to run"). */
  missingStep?: string | undefined;
};

/** The first thing standing between the form and Run, or nothing when Run is available. */
export function runBlocker(state: RunState): string | undefined {
  if (state.restarting) return `Restarting ${state.tool}…`;
  if (state.readiness?.state === "unavailable") {
    const needed = blocking(state.readiness.requirements).find((r) => r.state !== "present" && r.state !== "valid");
    return needed
      ? `${state.tool} isn't set up for this yet: ${needed.name} is ${needed.state}. Open Tool setup to fix it.`
      : `${state.tool} can't run this action right now. Open Tool setup to see why.`;
  }
  const selected = state.selected;
  if (selected) {
    if (selected.execution === "submitted" || selected.execution === "running")
      return "A run is in progress. Wait for it to finish, or cancel it.";
    if (selected.execution === "cancelling") return "Cancelling the last run…";
    if (selected.execution === "unconfirmed")
      return "The last run wasn't confirmed. Retry or discard it in the result panel first.";
    if (selected.saving) return "Saving the last run's evidence…";
  }
  if (state.toolHeld)
    return `${state.tool} is still reserved by a run whose result couldn't be saved, so Wafflery won't start another. Quit Wafflery and open it again to free it.`;
  if (state.toolBusy) return `${state.tool} is busy with another run. Run is available when it finishes.`;
  if (state.submitting) return "Sending the run…";
  if (state.importing) return "Adding an input…";
  if (state.unresolved > 0)
    return state.unresolved === 1
      ? "One input from the bundle isn't in your library yet. Choose it above to run."
      : `${state.unresolved} inputs from the bundle aren't in your library yet. Choose them above to run.`;
  if (state.chainClaimed) return "This step of the bundle already has its run. Choose the next step to continue.";
  return state.missingStep;
}

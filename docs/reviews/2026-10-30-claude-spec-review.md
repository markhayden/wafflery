Review supplied by the owner after running the prepared Claude CLI prompt outside the agent sandbox.
Received 2026-10-30. This is the unedited reviewer output; file/line references describe the
pre-review snapshot and may now be stale. No model/version was reported. Findings are advisory,
not verified implementation failures. See [disposition](2026-10-30-spec-review-disposition.md).

---

# Wafflehouse pre-implementation spec review

## 1. Verdict

**Not ready for a full implementation plan. Ready to plan a narrow first milestone once findings 1–3 are resolved.**

- The product decisions (Q1–Q18) are coherent, and the documents are disciplined about not inventing contract fields.
- Gates G1–G6 and C1–C4/H1 are already acknowledged. I do not repeat them below except where they create a new contradiction.
- The main new problems are in `ENGINEERING.md`:
  - It never says how a host attempt is linked to a tool run.
  - It never defines what "preservation complete" means under v0.1.
  - Its storage layout makes the accepted storage-location setting unusable in practice.

**Can fake-tool-first work begin without inventing fields?** Partly. Using only v0.1, a fake tool can drive:

- describe and `params_schema` forms
- image slots and image outs (`path` and `inline`, with `file_sha256`)
- progress, cancellation, run resources and the journal
- spend headers and the `estimate` action
- input snapshots, history, examples and baselines

It cannot drive these without new fields:

- the Playground/Operations split
- per-action readiness
- binding estimate arguments to a run
- typed artifact bindings
- report readiness
- portable exports

Those need C1, R3, C2 or C3. AC22 (`ACCEPTANCE.md:286-287`) lists reports and exports as fake-driven, which is premature. The fake must stay on v0.1 until owners publish draft schemas.

**Evidence I did not have:** `INTERFACE.md`, `TOOL-REQUIREMENTS.md`, `BRIEF.md`, the mocks, and the contract's `schemas/` directory. I cannot confirm whether the schemas define an estimate request/result shape beyond what `CONTRACT.md:213-214` says.

## 2. Findings

### Must fix before the build plan

**1. High: No agreed link between a host attempt and the tool's run.** (NEW)
- **Citations:** `ENGINEERING.md:72-73`, `ENGINEERING.md:174-175`; `ACCEPTANCE.md:124-125`; `CONTRACT.md:184`, `CONTRACT.md:190-192`, `CONTRACT.md:196-199`, `CONTRACT.md:232-233`.
- **What breaks:**
  - The host learns the tool's `run.id` only from the first progress notification or from the final result.
  - If Stop is pressed before the first progress notification arrives, the cancelled call returns no result. The host then has no ID for the "terminal run resource" it is told to read.
  - Matching against `<tool>://runs` by action and start time is guessing, which the spec forbids elsewhere.
  - The contract can also fail a call with a JSON-RPC error before any run exists (for example, a schema rejection). None of the execution states in `ENGINEERING.md:81-83` fits that case without fabricating a tool status.
- **Smallest fix:**
  - Always send a progress token, and persist `run.resource` from the first notification.
  - If no run ID is known, mark the outcome *unknown*. Never match against the runs list heuristically.
  - Add a host-only terminal state, e.g. `rejected`, for failures before a run exists.
  - Ask C4 for a client-supplied correlation ID or echo, but do not block on it.

**2. High: "Preservation complete" is undefined, yet v0.1 already supports a baseline.** (NEW)
- **Citations:** `ENGINEERING.md:77-79`, `ENGINEERING.md:94-106`; `ACCEPTANCE.md:156-162`; `CONTRACT.md:286-293`, `CONTRACT.md:414-428`, `CONTRACT.md:196-202`.
- **What breaks:** There is no rule for what counts as "all declared required assets." Implementers will either:
  - mark every run `unsupported` until C2 lands, which fails Q15 for ordinary output images; or
  - invent their own per-tool list, which breaks the generic-host rule.
- **Smallest fix:** Define the v0.1 baseline set explicitly:
  - `run.json` as returned
  - the journal resource, stored as opaque bytes
  - every `images.out`, fetched by `path` or `inline` and verified against `file_sha256`, `width` and `height`
  - `locations`, including any `mask` renderings they reference

  `complete` then means "the baseline set is verified." Native records (e.g. Rascal Mask zones) and report dependencies get an explicit `unsupported` label until C2/C3 define them. This makes most of G1 implementable now.

**3. High: Storage relocation is effectively impossible by design.** (NEW contradiction with Q14)
- **Citations:** `ENGINEERING.md:38`, `ENGINEERING.md:129-131`; `SPEC.md:150-151`; `ACCEPTANCE.md:175`.
- **What breaks:**
  - Tool workspaces (`tools/<installation-id>/`) live inside `Library/`.
  - No tool declares relocation support in v0.1, and unsupported tool-native path dependencies block relocation.
  - So after the first real Rascal run, every relocation attempt is blocked. The confirmed "choose another storage location" setting exists but never works.
- **Smallest fix:** Move tool workspaces outside the relocatable library, e.g. to `Wafflehouse/tools/`. Relocation then moves only the SQLite database and host-owned assets. Runs whose native artifacts depend on a tool workspace keep their existing workspace-dependent label.
  - Trade-off: bulky tool data stays on the original volume. Show it separately in the disk-usage view, as `ENGINEERING.md:123` already intends.

**4. Medium-High: Export promises generated client programs.** (NEW accidental promise)
- **Citations:** `ACCEPTANCE.md:230-231`, `ACCEPTANCE.md:241-242`; `USER-JOURNEYS.md:255-256`, `USER-JOURNEYS.md:263-265`.
- **What breaks:** AC17 requires "a full client example" covering startup, initialization, roots and dependencies. That pushes Wafflehouse toward generating MCP client code in some language. `USER-JOURNEYS.md:264` forbids exactly that without a verified execution path. Implementers will either build a code generator or fail AC17.
- **Smallest fix:** Make the first-release export a data bundle:
  - a manifest
  - the inputs
  - the recorded `tools/call` JSON
  - the server startup argv and the required roots
  - environment-variable names (no values)

  Mark it "runnable via CLI" only when C2 provides a CLI mapping. Reword AC17 so "full client example" means this setup data, not generated code.

### Must fix before the relevant milestone

**5. Medium: Behavior of estimates under serial tools is undefined.** (NEW)
- **Citations:** `ENGINEERING.md:157-164`; `ACCEPTANCE.md:107-112`; `CONTRACT.md:213-215`, `CONTRACT.md:196`.
- **What breaks:**
  - `estimate` is an ordinary action on a serial server. While a 10-minute paid run is active, a new draft cannot get an estimate, or the estimate call waits behind the run.
  - Revalidating the estimate at Run needs a second tool call while the execution is reserved.
  - If `estimate` creates run resources, it pollutes the tool's runs list. It is also unclear whether it creates host history.
  - Separately, AC02's "recheck after configuration change" may require a server restart, which silently refills the session budget (`ENGINEERING.md:165-166`).
- **Smallest fix:**
  - State that estimates are never attempts and are stored only as context on the attempt they authorized.
  - While the tool is busy, show "estimate unavailable: tool busy."
  - Make recheck an explicit, labeled session restart.
  - Ask C1/C4 whether `estimate` is exempt from the serial limit and whether it creates a run.

**6. Medium: Tool launchers must not reimplement host attachment.** (NEW clarification for H1)
- **Citations:** `ENGINEERING.md:135-148`; `docs/handoffs/table-manners.md:112-125`; `CONTRACT.md:260-261`.
- **What breaks:** "The launcher" attaches using the lock, authenticates the endpoint, and mints the bootstrap secret. If that means `rascal playground` and `squint playground`, each tool reimplements host-private protocol and auth logic, and any change needs coordinated releases across three repositories.
- **Smallest fix:** In H1, limit the tool's `playground` subcommand to three steps: locate the Wafflehouse binary, offer installation, and run it (exec) with a fixed argv such as `--tool <name>`. All lock handling, attachment, secret creation and browser opening then lives only in the Wafflehouse binary.

**7. Medium: Untrusted bytes are served from the authenticated origin.** (NEW local-security gap)
- **Citations:** `ENGINEERING.md:146-150`, `ENGINEERING.md:202-203`; `ACCEPTANCE.md:288-289`.
- **What breaks:** A user-imported or tool-returned file served same-origin (e.g. SVG, or HTML sniffed as an image) can run script with the session's mutation authority. Tool HTML is already excluded (`ENGINEERING.md:190-191`), but asset bytes are not.
- **Smallest fix:**
  - Serve assets only with an allowlisted raster `Content-Type`, plus `X-Content-Type-Options: nosniff`.
  - Use `Content-Disposition: attachment` for anything else.
  - Apply a restrictive CSP to the UI.
  - Add an SVG/HTML-disguised fixture to AC22.

### Optional / lower priority

**8. Low: Accidental scope for importing tool history.** (NEW)
- **Citations:** `ACCEPTANCE.md:154-155`; `USER-JOURNEYS.md:160-161`; `CONTRACT.md:201-202`.
- **What breaks:** AC11 implies Wafflehouse imports runs it did not start. Nothing designs that feature. Because H1 gives tools isolated managed workspaces, such runs would not even be visible to the host.
- **Smallest fix:** Explicitly defer importing runs started outside Wafflehouse, and drop or mark that AC11 bullet.

## 3. Simplify or defer (accepted requirements preserved)

- **Report viewers** (`ENGINEERING.md:186-189`): drop cross-tab sharing and lease tracking. Use one viewer per run with an idle timeout, and stop all viewers on quit. Q8/Q12 do not require leases.
- **Input materialization** (`ENGINEERING.md:67-69`): on macOS, use APFS `clonefile` into `execution/<attempt>/` and mark the copies read-only. Delete the directory once the attempt is terminal and preservation has finished, and sweep leftovers at startup. The cleanup timing is currently unspecified.
- **Capture stability** (`ENGINEERING.md:62-64`): drop "reject unstable capture when detectable." Hashing the copied bytes already defines what was submitted.
- **Asset verification state** (`ENGINEERING.md:49`): verify once at publish time. Defer any re-verification.
- **Fake tool scope** (`ACCEPTANCE.md:286-287`): limit the first fake to v0.1 features. Add C1–C3 behavior only after owners accept the draft schemas.

## 4. Owner decision needed next

**What should Playground show before C1 provides task/operation metadata?**

The recommended interim is:

- list every advertised action in Operations, using the shared selector;
- have Playground show a labeled "tasks pending tool declaration" state.

This lets a v0.1 fake-tool milestone ship without hardcoded action lists and without reopening Q3. The alternative is to block all UI planning until C1/R3 land.

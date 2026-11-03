# V21 tool chooser

Scope: Choose tool presentation only. The owner-selected Rascal and Squint sticker WebPs are
copied unchanged into `ui/src/assets/tools/rascal-sticker.webp` and `squint-sticker.webp`, replacing
the initial blue icons. They render decoratively in 40 × 40 CSS pixel boxes with aspect ratio retained.
The tool dialog is 44 rem wide (previously 34 rem), bounded by a 1 rem viewport margin on each side.
The map uses the declared tool name. Unknown tools use their contract purpose with reserved,
aligned logo space; errors replace the description. No sibling, contract or invocation changes.

Final owner copy:

- rascal: "Fully automated mask generation for precise image edits. Describe what to change and rascal finds the region, builds a high-quality mask and protects everything else, across a wide range of use cases."
- squint: "Image evaluation you can trust. Checks an image against the brief and references it was made from, and shows the evidence behind every verdict."

Each row puts name and right-aligned version on its first line, then description/error. The final
owner amendment uses the full two-sentence descriptions above. Descriptions wrap without a clamp
or truncation; the logo stays top-aligned with the name. The narrow dialog scrolls to additional
rows rather than shortening the copy. Browser checks cover the final clauses and unclamped dimensions.
Version strings are retained verbatim, including build hashes. Long versions ellipsize with the
full value in the title and accessible button name; long names may wrap. Search matches names,
descriptions and versions. Ordinary action/report picker layout and the selected workspace's
declared purpose remain unchanged.

The images are emitted as separate assets rather than inline data URLs. The existing image CSP
is unchanged. The embedded host serves these assets as image/webp; a browser test verifies exact
source bytes, MIME, actual image decoding under the real page's CSP and a clean error console.
This matters because the host also uses nosniff.

## Screenshots

The open chooser is captured at normal width and at the narrowest supported 320 px viewport:

- [Normal picker](../../ui/tests/browser/tool-picker.spec.ts-snapshots/tool-picker-1440-chromium-darwin.png)
- [Narrow picker](../../ui/tests/browser/tool-picker.spec.ts-snapshots/tool-picker-320-webkit-darwin.png)
- [Long names/versions](../../ui/tests/browser/tool-picker.spec.ts-snapshots/tool-picker-long-320-webkit-darwin.png)

These use the production Picker and presentation map through Storybook. Unknown/error rows are
illustrative. The separate packaged-host test uses a real local host and its declared sample peer;
logo decoding is tested without adding fake protocol registrations.

## Verification and rollback

The presentation unit test failed before implementation. Browser checks cover both known logos,
exact amended copy, unknown-purpose fallback, error priority, aligned rows, no name/version overlap,
name/description/hash search, selection and focus restoration in Chromium and WebKit. New chooser
regions use the existing visual policy; the existing workspace regions remain unchanged.

Coordinator amendment 3 narrows the final gate to UI unit/story tests and the touched picker browser
spec only. All 102 UI unit tests, the Storybook build and all 10 Chromium/WebKit picker cases
passed with amendment 2. Earlier Rust/clippy/typecheck and broader browser checks passed; the final full-matrix
rerun was stopped as requested. Independent review approved the required corrections. Generated
assets are included in the single task commit. No push, CI wait or screenshot delivery is part of
this local UI review. Revert the single V21 commit to roll back; no data/schema migration is needed.

## Owner review follow-up

Sticker art is copied as repository-owned original WebPs; there is no runtime dependency on a
sibling checkout or website. The tool dialog widens to 44 rem; action/report dialogs retain their
existing width. All eight normal/narrow/long-version baselines are refreshed for this change.

Squint's apparent unknown version was an MCP startup failure: the demo selected a normal binary
while passing the test-only `--provider-script` flag. The helper now checks for that capability
before creating tool folders or starting the host and provides a testkit setup explanation. The
separate local `target/demo-squint/debug/squint` build was verified through a real host to report
`0.1.0 (2d90b69c3)`; Rascal reported `0.1.1`. The review demo must use that executable when restarted.
Declared versions remain unchanged and no version is inferred from the registration name.

The follow-up passed 103 unit tests, focused demo/presentation checks, UI/script type checking,
Storybook build and all 10 picker cases in Chromium/WebKit. Source copies match the selected
sticker originals byte-for-byte. Targeted checks only; no push, full matrix or paid calls.

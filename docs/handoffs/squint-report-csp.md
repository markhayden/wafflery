# Squint server-report CSP/style correction

Status: resolved by approved Squint `54b1f41`; Chromium and WebKit strict checks pass.
The original defect and requested correction below are retained as historical evidence.

V18 uses coordinator-approved Squint `db5cf78`, built with `testkit` in an isolated archive.
The scripted reference-consistency run succeeds and both reports load inside Wafflery's drawer.
A strict browser-console check reveals a server-viewer rendering defect in Chromium and WebKit.

## Reproduction and source

Open the completed run's declared `report` server view with the exact host frame ancestor.
The response policy from `crates/squint-cli/src/viewer.rs:88` is:

```text
default-src 'self'; img-src 'self' data:; frame-ancestors <host-origin>
```

`crates/squint-cli/viewer/viewer.js:34` sets attributes through `node.setAttribute(k, v)`.
The viewer supplies inline styles for legend colors at lines 255–258 and timeline bars at
line 382. With no `style-src` override, the server's `default-src 'self'` blocks these attributes.
Chromium reports repeated “Applying inline style violates … default-src 'self'” errors at
`/viewer.js:34`; WebKit reports the corresponding stylesheet refusal. Report heading, decision
and images load, so checking only visible content misses the broken styling.

The declared `export` document is served under Wafflery's isolated self-contained-document
policy, which allows inline style; that is a separate path and is not the failing policy.
Wafflery neither rewrites the tool's HTML nor relaxes the tool server's response headers.

## Requested follow-up

The coordinator should route a Squint-side fix: align the viewer's style construction with its
CSP and verify the real server page in a browser, including legend colors and timeline placement.
Keep its exact `frame-ancestors` restriction and other security controls. The choice of style
classes versus a narrowly appropriate style policy belongs to Squint/the coordinator.
Provide a newly approved Squint revision for V18's isolated rebuild and rerun. No sibling files
were edited. The new opt-in `real-peer-reports.spec.ts` console check remained strict and initially
failed for this issue; the existing saved-evidence, report-close and restart checks pass.

A separate connection-refused console entry was the test keeping its host page alive during
intentional server shutdown. The fixture now navigates away before stopping the host; that
expected lifecycle disconnect is not the Squint defect. Raw traces and screenshots remain local
because they contain transient run paths and frame capabilities; no credentials were supplied.

## Resolution

Squint `54b1f41` includes `25d2192`: CSSOM for computed positions and external CSS classes
for legend colors; the response CSP and exact frame ancestor are unchanged. Wafflery rebuilt
that approved revision in isolation and reran four Rust probes and eight browser journeys.
All passed, including unfiltered console/page-error checks and distinct nontransparent legend colors
in Chromium and WebKit. No sibling checkout was modified.

One remaining WebKit error came from test instrumentation: installed Playwright's
`inPagePrepareForScreenshots` inserts a `body {}` inline stylesheet when
`shouldToggleStyleSheetToSyncAnimations()` is true. The trace places this violation inside
`page.screenshot`, with no viewer source location. Optional screenshots were removed from the
strict test; console collection is unchanged. Existing local screenshot inspection is retained
as separate evidence, not a claim that screenshot instrumentation obeys a tool's CSP.

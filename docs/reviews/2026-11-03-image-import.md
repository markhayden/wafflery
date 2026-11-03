# V20 image import and derived previews

Original files, dimensions and SHA-256 fingerprints remain authoritative. Header feedback is
provisional: the browser reads at most 1 MiB of encoded PNG/JPEG header data without creating a
raster. During upload/validation the input shows the filename, dimensions, estimated RGBA8 memory
and an indeterminate progress bar. This is phase feedback, not invented percentage completion.
Tasks and operation image ports share the feedback component. Unsupported or incomplete headers
still go through the host's complete validation; successful headers do not make an image ready.

The host keeps its existing 128 MiB / 128 megapixel library boundary. It does not infer Squint's
private limit. An input refusal shows the tool's own reason/remedy verbatim as text, with a distinct
input-refusal heading. The proposed optional tool pixel limits remain a coordinator-owned future
contract addition.

## Decode and storage

PNG validation streams every first-frame scanline (including Adam7), retaining selected samples in
an at-most-1024-edge raster before producing an at-most-512-edge PNG. Palette, transparency,
grayscale, RGB and 16-bit samples are covered; original pixels and bit depths are never rewritten.
JPEG uses scaled DCT decode followed by a final bounded resample. Small sources are not enlarged.
A preview is a display aid; sampling, 8-bit conversion and colour conversion are not fidelity evidence.
EXIF orientation is applied to the display copy, while raw stored dimensions/bytes remain unchanged.
Full-resolution inspection/comparison and downloads continue to load verified original bytes.

The preview is generated before an imported asset becomes ready. PNG chunk/data checksums are
validated explicitly. The JPEG header preflight rejects malformed marker layout more strictly than the codec. Scan
decoding still follows codec recovery rules; this is not a promise to reject every recoverable
damaged scan. Its private atomic derived cache
is associated with the original fingerprint, without a new database identity or schema migration.
Legacy assets regenerate lazily from hash-verified originals. Missing, corrupt and symlinked derived
files are repaired without following them; asset cleanup removes the associated cache. Generation
holds the storage lease and bounded decoder semaphore through its blocking worker. Cancellation
cannot release that ownership while the worker is publishing. Cached previews are decoded only
within 512-pixel edges and a small allocation limit before serving; they cannot become run inputs.
They do not re-attest the original file on every thumbnail read.

The existing `image` JPEG backend has no scaled-decode interface. The additional pinned pure-Rust
`jpeg-decoder = 0.3.2` (MIT/Apache-2.0, default features disabled) supplies it; no native library,
installer or runtime process is added. `png = 0.18.1` was already a transitive dependency. Primary
API sources: [JPEG scale](https://docs.rs/jpeg-decoder/0.3.2/jpeg_decoder/struct.Decoder.html#method.scale),
[PNG rows](https://docs.rs/png/0.18.1/png/struct.Reader.html#method.next_interlaced_row).
The JPEG crate is in maintenance mode. Scaled output does not promise constant working memory:
progressive JPEG retains source-sized coefficient arrays. Lossless/hierarchical/arithmetic JPEG
remain unsupported, as with the previous import decoder; use a PNG or supported DCT JPEG copy. Before codec allocation, a small SOF-header parser validates supported channels/sampling
and bounds progressive coefficients using MCU padding and the actual sampling factors. It reserves 64 MiB for output/workers/metadata in the existing
512 MiB decoder budget; over-budget files are refused with an explicit smaller-copy remedy. The
128 megapixel cap also remains. Cached thumbnails bypass the source decoder queue, with their own
small bounded validation. The measurements below cover ordinary sequential JPEG, without a claim
of constant source working memory.

## Release measurement

Machine: Apple M4 Max (Mac16,5), 64 GiB RAM. Rust release build, locked dependencies; same local
filesystem and disposable SQLite library. Smooth synthetic RGB gradients, generated separately
from the timed process. `/usr/bin/time -l` measures process peak RSS, including runtime/database.
No provider calls, credentials, real user images or filesystem paths are recorded here. Single
warm-cache observations illustrate the difference; they are not p95 estimates or a photographic
codec benchmark. Small imports now include preview generation, so some are slower.

| Input | Encoded bytes | Import before / after (ms) | Peak RSS before / after (MiB) | Cached preview (ms / bytes) |
| --- | ---: | ---: | ---: | ---: |
| PNG 2048 × 1024 | 33,651 | 22.39 / 29.45 | 10.92 / 11.59 | 0.67 / 3,848 |
| JPEG 2048 × 1024 | 71,800 | 14.09 / 23.51 | 10.98 / 6.38 | 0.88 / 100,141 |
| PNG 15000 × 7500 | 1,655,510 | 71.10 / 72.28 | 328.19 / 12.33 | 0.70 / 3,847 |
| JPEG 15000 × 7500 | 3,188,475 | 132.77 / 101.29 | 334.80 / 26.06 | 1.18 / 104,943 |

A second owned fixture uses deterministic 4 × 4 colour blocks to exercise much larger encoded
payloads. The retained pre-change release executable and the new executable read identical files,
in separate timed processes. These are also single warm-cache observations:

| Textured input | Encoded bytes | Import before / after (ms) | Peak RSS before / after (MiB) | Cached preview (ms / bytes) |
| --- | ---: | ---: | ---: | ---: |
| PNG 2048 × 1024 | 774,757 | 22.73 / 25.94 | 11.48 / 12.89 | 1.21 / 517,389 |
| JPEG 2048 × 1024 | 1,397,135 | 24.00 / 35.35 | 12.23 / 8.31 | 0.28 / 393,570 |
| PNG 15000 × 7500 | 41,507,436 | 229.99 / 235.57 | 368.53 / 49.70 | 1.24 / 439,177 |
| JPEG 15000 × 7500 | 74,952,127 | 695.23 / 725.72 | 471.69 / 91.89 | 0.83 / 275,661 |

Peak RSS includes the helper's subsequent verified original read, not just the decode stage. The
large textured JPEG trades about 30 ms extra import work for substantially lower peak memory and
a small cached browser preview. This is not an across-the-board latency win. These results also
explain why a local import need not be instantaneous: validation, hashing, durable ownership and
preview generation still perform real work.

Original verified reads after import remain available (0.09–1.70 ms in this sample). Previously
input cards sent those full originals to the browser; cards now request only bounded previews.
Reproduce using the opt-in `image-import-bench` example: generate owned fixtures with `--generate
<folder>`, then run a separate timed process for each image. Generation is excluded from RSS/timing.

## Optional resized copy

Deferred deliberately. A faithful explicit copy workflow needs output dimensions/resampling choice,
coordinate interpretation and separate ownership. It would not be a cheap addition to this import
feedback task. No automatic resizing or original overwrite was introduced. A user can prepare a
smaller copy in their image editor and import it explicitly; the tool's reported remedy stays visible.

## Verification

Header/unit tests, streamed codec/reference comparisons, cache repair and cleanup, immutable
original/download checks, and authenticated bounded-preview API tests cover the new boundaries.
Both Chromium and WebKit exercise pending feedback, failure recovery, ordered images, operation
ports and paid-run regressions. Full locked Rust suite, clippy and UI typecheck are the task gate;
hosted CI remains the final full-browser gate.

Review corrections were reproduced before implementation: corrupt PNG compressed checksum with
valid chunk CRC, dropped import waiter releasing publication ownership, and thumbnail-write failure
publishing an untracked original. All now pass. Import workers remain owned through metadata commit, and graceful host shutdown takes the storage
write lock before closing SQLite. A database preflight under the read lock refuses late imports
before they can publish after shutdown. Malformed JPEG stuffed markers cannot hide a different
frame from the working-memory guard; the preflight is deliberately stricter than a decoder that
accepts junk between segments. Lossless JPEG is refused before its unsafe scaled decode path.
Thumbnail publication precedes original publication. Source/file mutation, disk errors and decoder
failures still prevent ready metadata. Crash/failed-database orphan recovery is unchanged and remains
conservative. Additional tests cover sampling-aware JPEG budget refusal before allocation, cached
preview reads during a held source decoder, larger interlaced downsampling against an independent
full decode, and EXIF orientation without original-byte mutation.

No automatic startup orphan sweep was added: crash recovery and deletion of unreferenced bytes
remain the existing conservative policy, consistent with the owner's exploratory-workflow scope.
This task fixes graceful shutdown and dropped-waiter publication races. A failed database write or
process crash can retain immutable unreferenced bytes, just as before; a future recovery task must
specify and review deletion ownership separately.

Final verification: full locked Rust workspace suite; formatting and all-target clippy; TypeScript
checks; all 100 UI unit tests; 58 zero-retry Chromium/WebKit browser cases, including paid specs;
byte-identical clean UI builds/check-ui; release benchmark build. Pending/import-refusal region
snapshots use the existing visual policy, masking only the native indeterminate progress animation.
Independent review approved the required corrections. Hosted CI remains the full push gate.

## Rollback

Revert the UI-consumer commit before the backend-preview commit, as one task batch. SQLite schema,
original image bytes and original fingerprints did not change. Derived cache files can remain inert
when rolling back; they are not inputs or portability evidence. No automatic file migration or
startup deletion is needed.

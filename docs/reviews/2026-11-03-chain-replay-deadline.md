# Guided replay test setup and observation deadline

Hosted run [41635204792](https://github.com/markhayden/wafflery/actions/runs/41635204792) failed the
WebKit native-chain test twice. The original test created four source runs through the UI, selected
and built an archive, then reviewed and ran two replay steps under one 15-second body deadline.

The trace and allowlisted pre-shutdown diagnostics show no stuck preservation worker. All six
attempts in the first failure finished with complete contract evidence and preserved native bytes.
The final consumer was preserved approximately 0.4 seconds before the body deadline. Its next
500 ms chain-status request was aborted when the deadline began teardown. In the retry, only five
attempts existed; time ran out while preparing/opening the second consumer. Completed producer
state was being refreshed normally. The apparent load error followed deadline-triggered teardown.

Source-chain creation now has an explicitly bounded, per-test fixture lifetime of 15 seconds,
matching the existing host fixture pattern. The fixture waits for all source outputs and the saved
bundle's enabled Start chain review control before handing over. The replay body retains the
ordinary 15-second limit and all assertions: fresh changed output, one ordinary Run per step,
no upstream repetition, and observed final completion. A genuine setup, preservation or status
failure remains a test failure; no product polling, retry count or assertion deadline changed.

The controlled reproduction delays each of the four source-action requests by 2.5 seconds (ten seconds total). The original test
hits its shared body deadline; the revised fixture finishes setup before the replay body begins.
Scratch reproduction specs/configuration are removed and are not part of CI. The original delayed test failed at its unchanged body deadline; the revised fixture passed.
Twenty final zero-retry WebKit repetitions and all 26 unchanged-baseline Chromium/WebKit replay
checks passed. Independent follow-up review approved the fixture ownership and bounded waits.

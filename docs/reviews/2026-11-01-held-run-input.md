# Held-run browser input verification

V07a's first full matrix passed Rust and UI checks but failed the existing held-session restart
case in WebKit (85 browser cases passed, four skipped). Its trace contained no attempt POST and
the pre-shutdown attempt snapshot was empty. The required label remained empty in snapshots
immediately around fill and Run; native form validation therefore prevented submission. The
same input node persisted. This does not identify the exact WebKit input mechanism.

The test filled the label immediately after setInputFiles, which returns before the asynchronous
import is settled. It now waits for the source region to stop importing and show its preview,
clicks and confirms focus, fills and verifies the label value, then submits. No retries or longer
timeouts. Review approved these observable prerequisites. Repeat and final matrix results are
recorded in the V07a execution checkpoint.

Verification completed: 40 focused repetitions passed (20 Chromium, 20 WebKit), then the full
matrix passed with 86 browser cases and four skips. No production change or timing retry.

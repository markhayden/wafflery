# H1: shared Wafflehouse launcher proposal

Status: concrete proposal for Rascal/Squint owner agreement, 2026-10-31. Not implemented or
published. Wafflehouse owns this interface now; foundation task F05a verifies it, while real
installation remains gated on an actual release artifact. [Source inventory](../integration-sources.md).

## Canonical public surface

- Program name: **`wafflehouse`**. No generic `playground` alias or compatibility shim is proposed.
- Invocation from a tool: executable argv `--tool`, `<tool-id>`; e.g. `wafflehouse --tool rascal`
  or `wafflehouse --tool squint`. Tool selection is a hint to the shared UI, not an executable
  name to run. Unknown IDs show the host's unavailable/setup state; they do not trigger installation.
- Direct invocation: `wafflehouse`. Public `--help` and `--version` are supported. Duplicate or
  malformed selector flags are usage errors. Tools do not append private host-mode flags.
- The tool process waits for this short launcher command and propagates its exit code. The
  launcher exits after host readiness/browser handoff; it does not stay attached to the host.

## Finding and installing the program

Lookup order: explicit absolute **`WAFFLEHOUSE_BIN`**, then PATH `wafflehouse`, then
`~/.local/bin/wafflehouse`. An invalid explicit override fails with an actionable message;
do not fall through to an unrelated executable (exit 69, naming the invalid variable). Check
it is an executable regular file after following symlinks; Homebrew-style symlinks are valid.
This binary location is separate from `~/Library/Application Support/Wafflehouse`, which stores
library/settings/tool workspaces. Installation must not rewrite shell profiles or require sudo.

When absent in an interactive terminal, explain that Wafflehouse is a separate shared local
program and offer installation. Interactive means stdin and stderr are both terminals; the
prompt goes to stderr, defaults to No, and EOF means No. No leaves the system unchanged and
returns 0. Yes invokes only
the owner-agreed versioned installer/release artifact, verifies it, installs to `~/.local/bin`,
then invokes that absolute path. On a noninteractive invocation, return 69 with instructions;
never prompt indefinitely or infer consent. A failed installation does not open a browser.

The release channel, artifact naming, integrity/signature mechanism and first published version
still require the distribution decision. Do not invent a live URL now. Launcher implementations
can complete against an injected fake installer/executable; production auto-install remains
explicitly unavailable until the approved artifact exists. No hidden curl-to-shell fallback.
An accepted offer before that artifact exists returns 69 without installing anything. Inject
fake installers through a test-only implementation seam; no public installer override is
needed for this milestone. The approved installer will take `--prefix <absolute-bin-directory>`;
success requires exit 0 and a valid installed executable, otherwise return 74.

Each tool's `playground` subcommand takes no extra arguments (exit 64 otherwise), and invokes
exactly `--tool <id>`. It does not forward arbitrary flags to the host.

| Exit | Meaning |
|---|---|
| 0 | Host ready and browser handoff succeeded, or user declined a missing-host install offer |
| 64 | Invalid invocation/arguments |
| 69 | Required executable/release unavailable, incompatible active host requiring restart, or noninteractive missing host |
| 70 | Host startup/internal failure |
| 74 | Installation or local I/O failure |
| 130 | User interrupted launcher/installer |
| 143 | Launcher/installer terminated by SIGTERM |

Other signal deaths map to 128 plus the signal number. Host exit codes, including 70 and 74,
pass through unchanged. When no host executable exists, the tool itself returns 69.

Browser-launch failure prints the ready local address and reports a failure rather than claiming
the browser opened. Browser bootstrap material is short-lived and must not enter ordinary logs.
Tool callers do not parse stdout/stderr to discover the running host.

## Private attachment and process lifetime

All attachment logic belongs to the Wafflehouse executable: resolve selected library, check the
same-user control socket/protocol identity, acquire its lifetime lock if no host exists, start a
detached host, await readiness, mint a one-time browser bootstrap and open the browser. Only
after readiness may the launcher report success. Two concurrent launches converge on one host.

Tools must not read `host.json`, own locks, contact the control socket, create bootstrap secrets,
or embed browser authentication. The private handshake can change without coordinated tool
releases. It reports host protocol/version, selected library identity and ready loopback origin;
an incompatible live host requires an explicit idle restart, never a second writer or implicit
shutdown of active runs. Wafflehouse implements/validates this in F05a/F05b and F06a/F06b.

Closing the tool's terminal after launcher return or closing a browser tab does not stop the
host. Quit Wafflehouse owns graceful child shutdown. There is no auto-start login service.

## Separate startup-registration follow-up

H1's launcher surface does not solve tool MCP registration. The host still needs an explicit
executable/argv/config/workspace descriptor to start each server. Foundation smoke descriptors
remain local test data. Specify that registration interface separately with tool owners; do
not hold up agreement on the binary name and `--tool` launcher surface waiting for it.

## Required owner changes and verification

- Rascal: replace `PLAYGROUND`/`playground` lookup with the canonical names above; change the
  missing-host path to the agreed interactive offer. Update its existing task-8.7 tests. This is
  a new corrective change after merged M8, not reopening the historical checkpoint.
- Squint: implement its H1-blocked task 7.9 against the same lookup/argv/exit semantics.
- Both: fake executable captures exact argv, paths with spaces work without a shell, invalid
  overrides fail, no/yes/noninteractive/installer failure are tested, and child exit codes propagate.
- Wafflehouse: concurrent launch, stale discovery, compatible/incompatible attachment, browser
  opening and terminal detachment are process tests. No public download is needed for these.

Owner response needed: accept or amend names/lookup/exit semantics, then provide fixture/test
evidence. Production installer availability is an explicit later subgate, not a reason to leave
the agreed launcher ABI unspecified.

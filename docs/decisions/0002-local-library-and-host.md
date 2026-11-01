# ADR-0002: One local library and one owning host

Status: SQLite embedded in the single host accepted (Q18); detailed engineering mechanisms
remain proposed. Product requirements Q6–Q8 and Q14–Q17 are confirmed.
Date: 2026-10-30

## Context

Rascal-first, Squint-first, and Wafflery-first must share history and examples. Saved work
must survive changes to original files. Browser tabs do not own execution. These are exploratory
runs; durable job scheduling and automatic crash recovery would add unnecessary complexity.

## Proposed decision

The owner confirmed SQLite as a local database opened by the host on startup and closed on
shutdown, with no separate database service. This acceptance does not approve the whole build
plan or settle the tool-facing interfaces below.

Use one local library, defaulting on macOS to
`~/Library/Application Support/Wafflery/Library`. Keep a small settings file in the parent
Wafflery directory identifying the selected library. Only one library is active at a time.
Use SQLite for run/example metadata and ownership references; store large immutable assets as
separate files addressed by their SHA-256 byte digest. No database service is installed.

One Rust host owns the library lock, database writes, tool processes, and report processes.
Launchers attach to a compatible existing host or start a detached host on demand. No login
service or automatic restart is needed. Run one execution per tool at a time, without a queue;
different tools may run concurrently. All tabs share these limits and the same tool sessions.

Tool-native workspaces are separate from immutable asset storage. Tools must declare their
startup and artifact-preservation interfaces; the host never guesses their internal files.
Keep these workspaces outside the relocatable library at a stable location in the Wafflery
application-data directory. A library move covers owned history/assets; tool workspace storage
and its separate disk usage remain visible. Moving native workspaces needs tool support.
Completed evidence remains accessible from host-owned storage. Native replay and reports retain
explicit tool/version dependencies where required.

After a host crash, restore committed records and flag unfinished attempts as interrupted.
Do not retry work automatically. Database/file consistency still matters even though execution
recovery is deliberately limited. Full lifecycle and failure rules are in
[the engineering draft](../../ENGINEERING.md).

## Alternatives and consequences

- Project-local history conflicts with the accepted single-library experience.
- A directory of mutable JSON indexes makes shared asset ownership and deletion harder to keep
  consistent. SQLite supplies transactions for metadata; it does not transact external files.
- Storing every image in the database would tie large file transfers to the metadata store.
  Separate immutable files suit inspection/export but require ordered writes and orphan cleanup.
- Per-tab servers would duplicate sessions and budgets. One host gives them a common owner.
- A persistent job service and automatic retries are unnecessary for the accepted recovery scope.

Owned snapshots cost disk space. Tool workspaces can contain additional copies; deduplication is
promised for host-managed identical assets, not across opaque tool stores. A saved bundle is not
automatically portable merely because its files have been copied.

## Sources

This is a design recommendation grounded in the accepted product choices and
[table-manners v0.1.0](../../../table-manners/CONTRACT.md), sections 4–5 and 10.
SQLite documents [local application storage](https://www.sqlite.org/whentouse.html) and
[atomic metadata transactions](https://www.sqlite.org/atomiccommit.html). Apple identifies
[Application Support](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html)
as a location for application support data. Exact dependency pins and macOS process APIs belong
in the remaining engineering specification.

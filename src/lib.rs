//! Wafflery's local host. Foundation modules are added as their slices land.

pub mod host;
pub mod library;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const HELP: &str = "Wafflery — local visual pipeline workshop

Usage: wafflery [--tool <id>] [--no-open] [--data-dir <path>]
       wafflery start [--data-dir <path>] [--no-open]
       wafflery --register <descriptors.json> [--replace-registration] [--data-dir <path>]

  -h, --help     Show this help
  -V, --version  Show the version
      --tool     Select a tool in the shared workspace
      --no-open  Leave browser opening to the caller; print the local address
      --data-dir Use a separate local data directory for development
      start      Keep the host in this terminal with logs (Ctrl-C to quit)
      --register Register local tools while the host is stopped; do not launch them
      --replace-registration Explicitly replace existing registered IDs while stopped

The default launcher starts or attaches to a detached local host.
WAFFLERY_DATA_DIR selects an absolute library parent for every entry point;
--data-dir takes precedence. Otherwise the personal macOS data directory is used.";

pub mod tools;

pub mod events;

#[doc(hidden)]
pub mod process;

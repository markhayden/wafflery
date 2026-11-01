//! Wafflehouse's local host. Foundation modules are added as their slices land.

pub mod host;
pub mod library;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const HELP: &str = "Wafflehouse — local visual pipeline workshop

Usage: wafflehouse [--tool <id>] [--no-open] [--data-dir <path>]
       wafflehouse --foreground --no-open [--data-dir <path>]
       wafflehouse --register <descriptors.json> [--data-dir <path>]

  -h, --help     Show this help
  -V, --version  Show the version
      --tool     Select a tool in the shared workspace
      --no-open  Leave browser opening to the caller; print the local address
      --data-dir Use a separate local data directory for development
      --foreground  Keep the host in this terminal (Ctrl-C to quit)
      --register Register local tools while the host is stopped; do not launch them

The default launcher starts or attaches to a detached local host.
WAFFLEHOUSE_DATA_DIR selects an absolute library parent for every entry point;
--data-dir takes precedence. Otherwise the personal macOS data directory is used.";

pub mod tools;

pub mod events;

//! Wafflehouse's local host. Foundation modules are added as their slices land.

pub mod host;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const HELP: &str = "Wafflehouse — local visual pipeline workshop

Usage: wafflehouse [--tool <id>] [--no-open] [--data-dir <path>]
       wafflehouse --foreground --no-open [--data-dir <path>]

  -h, --help     Show this help
  -V, --version  Show the version
      --tool     Select a tool in the shared workspace
      --no-open  Leave browser opening to the caller; print the local address
      --data-dir Use a separate local data directory for development
      --foreground  Keep the host in this terminal (Ctrl-C to quit)

The default launcher starts or attaches to a detached local host.";

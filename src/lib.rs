//! Wafflehouse's local host. Foundation modules are added as their slices land.

pub mod host;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const HELP: &str = "Wafflehouse — local visual pipeline workshop

Usage: wafflehouse [--help | --version]

  -h, --help     Show this help
  -V, --version  Show the version

The local host is not available in this foundation scaffold yet.";

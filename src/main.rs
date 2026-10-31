use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--version" || args[0] == "-V") {
        println!("wafflehouse {}", wafflehouse::VERSION);
        ExitCode::SUCCESS
    } else if args.is_empty() || (args.len() == 1 && (args[0] == "--help" || args[0] == "-h")) {
        println!("{}", wafflehouse::HELP);
        ExitCode::SUCCESS
    } else {
        eprintln!("Invalid arguments. Run wafflehouse --help for usage.");
        ExitCode::from(64)
    }
}

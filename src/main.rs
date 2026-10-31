use std::{env, process::ExitCode};
use wafflehouse::host::launch::Options;

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--version" || args[0] == "-V") {
        println!("wafflehouse {}", wafflehouse::VERSION);
        return ExitCode::SUCCESS;
    }
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!("{}", wafflehouse::HELP);
        return ExitCode::SUCCESS;
    }
    let options = match Options::parse(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("Invalid arguments: {error}. Run wafflehouse --help for usage.");
            return ExitCode::from(64);
        }
    };
    if options.internal
        && let Err(error) = nix::unistd::setsid()
    {
        eprintln!("Cannot detach host: {error}");
        return ExitCode::from(70);
    }
    let result = if options.foreground || options.internal {
        match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(wafflehouse::host::serve(&options.root)),
            Err(error) => Err(error),
        }
    } else {
        wafflehouse::host::launch::attach(&options).map(|_| ())
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Cannot open Wafflehouse: {error}");
            ExitCode::from(match error.kind() {
                std::io::ErrorKind::InvalidData | std::io::ErrorKind::WouldBlock => 69,
                std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::NotFound => 74,
                _ => 70,
            })
        }
    }
}

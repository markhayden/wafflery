use std::{env, process::ExitCode};
use wafflery::host::launch::Options;

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--version" || args[0] == "-V") {
        println!("wafflery {}", wafflery::VERSION);
        return ExitCode::SUCCESS;
    }
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!("{}", wafflery::HELP);
        return ExitCode::SUCCESS;
    }
    let options = match Options::parse(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("Invalid arguments: {error}. Run wafflery --help for usage.");
            return ExitCode::from(64);
        }
    };
    if let Some(source) = &options.registration {
        let result = if options.replace_registration {
            wafflery::tools::registration::replace(&options.root, source)
        } else {
            wafflery::tools::registration::register(&options.root, source)
        };
        return match result {
            Ok(changed) => {
                println!(
                    "Saved {changed} tool registration(s). Launch wafflery to open this workspace."
                );
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("Cannot register tools: {error}.");
                if error.kind() == std::io::ErrorKind::WouldBlock {
                    eprintln!("Quit the host before changing its registration.");
                }
                ExitCode::from(if error.kind() == std::io::ErrorKind::WouldBlock {
                    69
                } else {
                    74
                })
            }
        };
    }
    if options.internal
        && let Err(error) = nix::unistd::setsid()
    {
        eprintln!("Cannot detach host: {error}");
        return ExitCode::from(70);
    }
    let result = if options.start || options.internal {
        match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(async {
                if options.start {
                    wafflery::host::start(&options.root, options.no_open).await
                } else {
                    wafflery::host::serve(&options.root).await
                }
            }),
            Err(error) => Err(error),
        }
    } else {
        wafflery::host::launch::attach(&options).map(|_| ())
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Cannot open Wafflery: {error}");
            ExitCode::from(match error.kind() {
                std::io::ErrorKind::InvalidData | std::io::ErrorKind::WouldBlock => 69,
                std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::NotFound => 74,
                _ => 70,
            })
        }
    }
}

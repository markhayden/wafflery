use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--version" || args[0] == "-V") {
        println!("wafflehouse {}", wafflehouse::VERSION);
        ExitCode::SUCCESS
    } else if args.is_empty() || (args.len() == 1 && (args[0] == "--help" || args[0] == "-h")) {
        println!("{}", wafflehouse::HELP);
        ExitCode::SUCCESS
    } else if args.len() == 4
        && args[0] == "--foreground"
        && args[1] == "--no-open"
        && args[2] == "--data-dir"
    {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("host runtime");
        match runtime.block_on(wafflehouse::host::serve(std::path::Path::new(&args[3]))) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Cannot start Wafflehouse: {error}");
                ExitCode::from(69)
            }
        }
    } else {
        eprintln!("Invalid arguments. Run wafflehouse --help for usage.");
        ExitCode::from(64)
    }
}

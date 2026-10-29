//! `codestamp-cli`: runs a collection without the app window, for CI pipelines. It works on its
//! own throwaway database, so the app's data on the machine is neither read nor changed.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "printing is how the command-line runner reports"
)]

mod args;
mod load;
mod report;
mod run;

use std::process::ExitCode;

pub fn main() -> ExitCode {
    match args::parse(std::env::args().skip(1)) {
        Ok(args::Command::Help) => {
            println!("{}", args::USAGE);
            ExitCode::SUCCESS
        }
        Ok(args::Command::Version) => {
            println!("codestamp-cli {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(args::Command::Run(options)) => tauri::async_runtime::block_on(run::run(*options)),
        Err(message) => {
            eprintln!("error: {message}\n\n{}", args::USAGE);
            ExitCode::from(2)
        }
    }
}

use std::path::PathBuf;

use crate::request::validate::{MAX_TIMEOUT_MS, MIN_TIMEOUT_MS};

pub const USAGE: &str = "\
Usage: codestamp-cli run <collection> [options]

Runs the HTTP and GraphQL requests of a collection in order, with their scripts and tests.
WebSocket, MCP and gRPC requests are skipped.

<collection>  A Collaboration folder, or a collection file: Codestamp or Postman
              Collection v2.1, OpenAPI 3 or Swagger 2 (JSON or YAML)

Options:
  -e, --environment <file>  Environment file, exported from Codestamp or Postman
      --var <name>=<value>  Sets an environment variable; repeat for more
      --var <name>          The same, with the value of that process environment variable
                            (keeps secrets off the command line)
  -d, --data <file>         CSV or JSON data file: one iteration per row
  -n, --iterations <count>  Iterations without a data file, 1 to 100 (default 1)
      --bail                Stops after the first failed request
      --delay <ms>          Waits between requests, up to 60000
      --timeout <seconds>   Request timeout, 1 to 300 (default: 30)
      --junit <file>        Writes a JUnit XML report
      --json <file>         Writes a JSON report, like the app's Export report
  -h, --help                Shows this help
  -V, --version             Shows the version

Exit codes: 0 every request passed, 1 something failed, 2 the run could not start.";

const MAX_ITERATIONS: u64 = 100;
const MAX_DELAY_MS: u64 = 60_000;

pub enum Command {
    Help,
    Version,
    Run(Box<RunOptions>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct RunOptions {
    pub collection: PathBuf,
    pub environment: Option<PathBuf>,
    /// Name and value, in the order given; a later one wins.
    pub variables: Vec<(String, String)>,
    pub data: Option<PathBuf>,
    pub iterations: u32,
    pub bail: bool,
    pub delay_ms: u64,
    pub timeout_ms: Option<u64>,
    pub junit: Option<PathBuf>,
    pub json: Option<PathBuf>,
}

pub fn parse(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
    match args.next().as_deref() {
        None | Some("-h" | "--help" | "help") => Ok(Command::Help),
        Some("-V" | "--version") => Ok(Command::Version),
        Some("run") => parse_run(args, |name| std::env::var(name).ok()),
        Some(other) => Err(format!("Unknown command “{other}”")),
    }
}

/// `lookup` reads a process environment variable, for `--var <name>`.
fn parse_run(
    mut args: impl Iterator<Item = String>,
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<Command, String> {
    let mut collection = None;
    let mut options = RunOptions {
        collection: PathBuf::new(),
        environment: None,
        variables: Vec::new(),
        data: None,
        iterations: 1,
        bail: false,
        delay_ms: 0,
        timeout_ms: None,
        junit: None,
        json: None,
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-e" | "--environment" => options.environment = Some(value(&mut args, &arg)?.into()),
            "--var" => options
                .variables
                .push(variable(&value(&mut args, &arg)?, &lookup)?),
            "-d" | "--data" => options.data = Some(value(&mut args, &arg)?.into()),
            "-n" | "--iterations" => {
                let count = number(&arg, &value(&mut args, &arg)?, 1, MAX_ITERATIONS)?;
                options.iterations = u32::try_from(count).map_err(|_| "Too many iterations")?;
            }
            "--bail" => options.bail = true,
            "--delay" => {
                options.delay_ms = number(&arg, &value(&mut args, &arg)?, 0, MAX_DELAY_MS)?;
            }
            "--timeout" => {
                let seconds = number(
                    &arg,
                    &value(&mut args, &arg)?,
                    MIN_TIMEOUT_MS / 1000,
                    MAX_TIMEOUT_MS / 1000,
                )?;
                options.timeout_ms = Some(seconds * 1000);
            }
            "--junit" => options.junit = Some(value(&mut args, &arg)?.into()),
            "--json" => options.json = Some(value(&mut args, &arg)?.into()),
            "-h" | "--help" => return Ok(Command::Help),
            flag if flag.starts_with('-') => return Err(format!("Unknown option {flag}")),
            path if collection.is_none() => collection = Some(PathBuf::from(path)),
            extra => return Err(format!("Unexpected argument “{extra}”")),
        }
    }
    options.collection = collection.ok_or("Say which collection to run")?;
    if options.data.is_some() && options.iterations > 1 {
        return Err("Use --data or --iterations, not both: a data file runs once per row".into());
    }
    Ok(Command::Run(Box::new(options)))
}

fn value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{option} needs a value"))
}

fn number(option: &str, text: &str, min: u64, max: u64) -> Result<u64, String> {
    text.parse::<u64>()
        .ok()
        .filter(|number| (min..=max).contains(number))
        .ok_or_else(|| format!("{option} must be a number from {min} to {max}"))
}

fn variable(
    text: &str,
    lookup: &impl Fn(&str) -> Option<String>,
) -> Result<(String, String), String> {
    let (name, value) = match text.split_once('=') {
        Some((name, value)) => (name.trim(), value.to_owned()),
        None => {
            let name = text.trim();
            let value = lookup(name).ok_or_else(|| {
                format!("--var {name}: there is no process environment variable with that name")
            })?;
            (name, value)
        }
    };
    if name.is_empty() {
        return Err("--var needs a name, such as --var token=abc".to_owned());
    }
    Ok((name.to_owned(), value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> Result<RunOptions, String> {
        let lookup = |name: &str| (name == "API_TOKEN").then(|| "from-env".to_owned());
        match parse_run(args.iter().map(|arg| (*arg).to_owned()), lookup)? {
            Command::Run(options) => Ok(*options),
            _ => Err("not a run".to_owned()),
        }
    }

    #[test]
    fn reads_run_options() {
        let options = run(&[
            "api",
            "-e",
            "staging.json",
            "--var",
            "base=https://x.test",
            "--var",
            "API_TOKEN",
            "--bail",
            "--delay",
            "250",
            "--timeout",
            "10",
            "--junit",
            "out.xml",
        ])
        .expect("options");
        assert_eq!(options.collection, PathBuf::from("api"));
        assert_eq!(
            options.variables,
            [
                ("base".to_owned(), "https://x.test".to_owned()),
                ("API_TOKEN".to_owned(), "from-env".to_owned())
            ]
        );
        assert!(options.bail);
        assert_eq!(options.delay_ms, 250);
        assert_eq!(options.timeout_ms, Some(10_000));
        assert_eq!(options.junit, Some(PathBuf::from("out.xml")));
    }

    #[test]
    fn refuses_bad_options() {
        for args in [
            &[][..],
            &["api", "--iterations", "0"],
            &["api", "--iterations", "101"],
            &["api", "--timeout", "301"],
            &["api", "--var", "MISSING"],
            &["api", "--var", "=x"],
            &["api", "--nope"],
            &["api", "extra"],
            &["api", "--data", "d.csv", "-n", "3"],
            &["api", "--environment"],
        ] {
            assert!(run(args).is_err(), "{args:?}");
        }
    }
}

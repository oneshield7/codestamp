use std::path::PathBuf;

use crate::error::AppError;

const MAX_ARGS: usize = 64;
const MAX_ENV_VARS: usize = 50;
const MAX_ENV_NAME_CHARS: usize = 128;

/// Splits a command line into the program and its arguments. Only quoting is understood
/// ("double" or 'single' quotes keep spaces); there is no shell, so nothing else is special.
pub fn split_command(line: &str) -> Result<Vec<String>, AppError> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut started = false;
    for c in line.chars() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => current.push(c),
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                started = true;
            }
            None if c.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            None => {
                current.push(c);
                started = true;
            }
        }
    }
    if quote.is_some() {
        return Err(AppError::invalid("A quote in the command is not closed"));
    }
    if started {
        words.push(current);
    }
    if words.is_empty() {
        return Err(AppError::invalid(
            "Enter the command that starts the server",
        ));
    }
    if words.len() > MAX_ARGS + 1 {
        return Err(AppError::invalid("The command has too many arguments"));
    }
    if words.iter().any(|word| word.contains('\0')) {
        return Err(AppError::invalid(
            "The command contains a character that cannot be used",
        ));
    }
    Ok(words)
}

/// Environment variable names a program can be given.
pub fn check_env(vars: &[(String, String)]) -> Result<(), AppError> {
    if vars.len() > MAX_ENV_VARS {
        return Err(AppError::invalid(
            "Up to 50 environment variables can be set",
        ));
    }
    let valid = vars.iter().all(|(name, value)| {
        let mut chars = name.chars();
        name.len() <= MAX_ENV_NAME_CHARS
            && chars
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !value.contains('\0')
    });
    if !valid {
        return Err(AppError::invalid(
            "Environment variable names use letters, digits and _, and cannot start with a digit",
        ));
    }
    Ok(())
}

/// Tools such as `npx` are `.cmd` files on Windows, which a program name alone does not find.
pub fn resolve_program(program: &str) -> PathBuf {
    #[cfg(windows)]
    {
        let bare = std::path::Path::new(program).extension().is_none()
            && !program.contains(['/', '\\', ':']);
        if bare && let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                for extension in ["exe", "cmd", "bat", "com"] {
                    let candidate = dir.join(format!("{program}.{extension}"));
                    if candidate.is_file() {
                        return candidate;
                    }
                }
            }
        }
    }
    PathBuf::from(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_words_and_keeps_quoted_spaces() {
        let words =
            split_command(r#"npx -y "@scope/server name" --root 'C:\My Files' x"#).expect("words");
        assert_eq!(
            words,
            [
                "npx",
                "-y",
                "@scope/server name",
                "--root",
                r"C:\My Files",
                "x"
            ]
        );
        assert_eq!(
            split_command(r#"node "" a"#).expect("empty quoted"),
            ["node", "", "a"]
        );
        assert_eq!(
            split_command("node server.js; rm -rf /").expect("no shell"),
            ["node", "server.js;", "rm", "-rf", "/"]
        );
    }

    #[test]
    fn refuses_broken_or_empty_commands() {
        assert!(split_command("   ").is_err());
        assert!(split_command("node \"server.js").is_err());
    }

    #[test]
    fn checks_environment_names() {
        let env = |name: &str| vec![(name.to_owned(), "x".to_owned())];
        assert!(check_env(&env("API_TOKEN")).is_ok());
        assert!(check_env(&env("_PRIVATE1")).is_ok());
        for bad in ["1TOKEN", "API-TOKEN", "A B", ""] {
            assert!(check_env(&env(bad)).is_err(), "{bad}");
        }
    }
}

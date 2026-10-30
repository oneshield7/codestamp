//! Runs the Git installed on this device. Signing in to the host is left to Git (on Windows the
//! Git Credential Manager), so Codestamp never sees or stores a password or token.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use url::Url;

use super::models::Author;
use crate::error::AppError;

const LOCAL_TIMEOUT: Duration = Duration::from_secs(30);
/// Long enough for a first sign-in through the credential manager's browser window.
const NETWORK_TIMEOUT: Duration = Duration::from_secs(180);
const MAX_URL_LENGTH: usize = 2_048;
const MAX_MESSAGE_CHARS: usize = 200;
const MAX_AUTHOR_CHARS: usize = 100;
const MAX_EMAIL_CHARS: usize = 254;
const MAX_USERNAME_CHARS: usize = 100;
const MAX_HOST_CHARS: usize = 253;
/// Applied to every call: no `ext::` transport (it runs commands), LF endings so files compare
/// equal on every machine, plain output, and no signing prompts for Codestamp's own commits.
const BASE_CONFIG: [&str; 12] = [
    "-c",
    "protocol.ext.allow=never",
    "-c",
    "core.autocrlf=false",
    "-c",
    "core.quotepath=off",
    "-c",
    "color.ui=false",
    "-c",
    "commit.gpgsign=false",
    "-c",
    "advice.detachedHead=false",
];
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitError {
    Missing,
    Timeout,
    Offline,
    SignIn,
    NotFound,
    ReadOnly,
    /// The remote changed since the last fetch (a push was refused).
    Rejected,
    Failed(String),
}

impl From<GitError> for AppError {
    fn from(error: GitError) -> Self {
        Self::InvalidInput(match error {
            GitError::Missing => {
                "Git is not installed. Install Git for Windows (git-scm.com), then try again"
                    .to_owned()
            }
            GitError::Timeout => "Git took too long to answer".to_owned(),
            GitError::Offline => "Could not reach the repository. Check the connection".to_owned(),
            GitError::SignIn => {
                "Git could not sign in. Check that your account has access to the repository"
                    .to_owned()
            }
            GitError::NotFound => {
                "The repository was not found, or your account cannot open it".to_owned()
            }
            GitError::ReadOnly => {
                "Your account can read this repository but not change it".to_owned()
            }
            GitError::Rejected => "The repository changed meanwhile; sync again".to_owned(),
            GitError::Failed(message) => format!("Git failed: {message}"),
        })
    }
}

pub struct Git {
    dir: PathBuf,
    /// `credential.username=…` for network calls; `None` leaves the account to Git.
    credential: Option<String>,
}

impl Git {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            credential: None,
        }
    }

    /// Network calls ask the credential manager for this account instead of the host's default.
    /// `username` must come from `check_username`.
    pub fn signed_in_as(dir: PathBuf, username: Option<&str>) -> Self {
        Self {
            dir,
            credential: username.map(|name| format!("credential.username={name}")),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub async fn run(&self, args: &[&str]) -> Result<String, GitError> {
        exec(&self.dir, args, LOCAL_TIMEOUT, None, None).await
    }

    /// Calls that talk to the remote, with the longer timeout.
    pub async fn network(&self, args: &[&str]) -> Result<String, GitError> {
        let mut full: Vec<&str> = Vec::with_capacity(args.len() + 2);
        if let Some(credential) = &self.credential {
            full.extend(["-c", credential]);
        }
        full.extend_from_slice(args);
        exec(&self.dir, &full, NETWORK_TIMEOUT, None, None).await
    }

    /// Commands that record an identity (commit, and merge even with `--no-commit`), so a device
    /// without a global Git identity can still sync.
    pub async fn run_as(&self, args: &[&str], author: &Author) -> Result<String, GitError> {
        exec(&self.dir, args, LOCAL_TIMEOUT, Some(author), None).await
    }

    /// The file at `rev`, or `None` when it does not exist there.
    pub async fn show(&self, rev: &str, path: &str) -> Result<Option<String>, GitError> {
        let spec = format!("{rev}:{path}");
        match self.run(&["cat-file", "-e", &spec]).await {
            Ok(_) => self.run(&["show", &spec]).await.map(Some),
            Err(GitError::Failed(_)) => Ok(None),
            Err(other) => Err(other),
        }
    }
}

/// `input` is written to Git's standard input, for commands that read the credential protocol.
async fn exec(
    dir: &Path,
    args: &[&str],
    timeout: Duration,
    author: Option<&Author>,
    input: Option<&str>,
) -> Result<String, GitError> {
    let mut command = Command::new("git");
    command
        .args(BASE_CONFIG)
        .args(args)
        .current_dir(dir)
        // Never wait for a typed answer; the credential manager shows its own window instead.
        .env("GIT_TERMINAL_PROMPT", "0")
        // English messages, so failures can be told apart.
        .env("LC_ALL", "C")
        .env("LANGUAGE", "C")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    // A clone that lost its .git must not make Git fall back to a repository above it.
    if let Some(parent) = dir.parent() {
        command.env("GIT_CEILING_DIRECTORIES", parent);
    }
    if let Some(author) = author {
        command
            .env("GIT_AUTHOR_NAME", &author.name)
            .env("GIT_AUTHOR_EMAIL", &author.email)
            .env("GIT_COMMITTER_NAME", &author.name)
            .env("GIT_COMMITTER_EMAIL", &author.email);
    }
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            GitError::Missing
        } else {
            GitError::Failed("Git could not be started".to_owned())
        }
    })?;
    if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|_| GitError::Failed("Git could not read its input".to_owned()))?;
    }
    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| GitError::Timeout)?
        .map_err(|_| GitError::Failed("Git stopped unexpectedly".to_owned()))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(classify(&String::from_utf8_lossy(&output.stderr)))
    }
}

/// The installed Git's version, or `None` when Git cannot be run.
pub async fn version(dir: &Path) -> Option<String> {
    exec(dir, &["--version"], LOCAL_TIMEOUT, None, None)
        .await
        .ok()
        .map(|text| text.trim().trim_start_matches("git version ").to_owned())
}

/// The name and email Git already uses on this device, as a starting point for the profile.
pub async fn configured_author(dir: &Path) -> Option<Author> {
    let read = |key: &'static str| async move {
        exec(
            dir,
            &["config", "--global", "--get", key],
            LOCAL_TIMEOUT,
            None,
            None,
        )
        .await
        .ok()
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
    };
    let author = Author {
        name: read("user.name").await?,
        email: read("user.email").await?,
    };
    check_author(&author).ok()
}

/// The GitHub accounts Git's credential manager is signed in to, or `None` when it cannot tell
/// (another credential helper, or a manager without the `github` commands).
pub async fn github_accounts(dir: &Path) -> Option<Vec<String>> {
    let args = ["credential-manager", "github", "list", "--no-ui"];
    let text = exec(dir, &args, LOCAL_TIMEOUT, None, None).await.ok()?;
    Some(
        text.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

/// Opens the credential manager's GitHub sign-in for `username`, a checked account name. The
/// token it gets stays with the credential manager.
pub async fn github_login(dir: &Path, username: &str) -> Result<(), GitError> {
    let args = [
        "credential-manager",
        "github",
        "login",
        "--username",
        username,
    ];
    exec(dir, &args, NETWORK_TIMEOUT, None, None)
        .await
        .map(|_| ())
}

pub async fn github_logout(dir: &Path, username: &str) -> Result<(), GitError> {
    let args = [
        "credential-manager",
        "github",
        "logout",
        username,
        "--no-ui",
    ];
    exec(dir, &args, LOCAL_TIMEOUT, None, None)
        .await
        .map(|_| ())
}

/// Asks Git's credential helpers to forget the sign-in of `username` on `host`; both checked,
/// so neither can add lines to the credential protocol.
pub async fn forget_credential(dir: &Path, host: &str, username: &str) -> Result<(), GitError> {
    let input = format!("protocol=https\nhost={host}\nusername={username}\n\n");
    exec(
        dir,
        &["credential", "reject"],
        LOCAL_TIMEOUT,
        None,
        Some(&input),
    )
    .await
    .map(|_| ())
}

fn classify(stderr: &str) -> GitError {
    let text = stderr.to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|needle| text.contains(needle));
    if has(&[
        "could not resolve host",
        "failed to connect",
        "timed out",
        "network is unreachable",
        "connection refused",
        "connection was reset",
        "could not connect",
    ]) {
        GitError::Offline
    } else if has(&[
        "write access to repository not granted",
        "permission to",
        "protected branch",
    ]) {
        GitError::ReadOnly
    } else if has(&[
        "authentication failed",
        "could not read username",
        "terminal prompts disabled",
        "permission denied",
        "invalid username or password",
        "returned error: 401",
        "returned error: 403",
    ]) {
        GitError::SignIn
    } else if has(&[
        "repository not found",
        "does not appear to be a git repository",
        "returned error: 404",
    ]) {
        GitError::NotFound
    } else if has(&[
        "[rejected]",
        "non-fast-forward",
        "fetch first",
        "failed to push some refs",
    ]) {
        GitError::Rejected
    } else {
        GitError::Failed(first_message(stderr))
    }
}

/// The first useful line of Git's output, without its "fatal:" style prefix and hints.
fn first_message(stderr: &str) -> String {
    let line = stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("hint:"))
        .unwrap_or("unknown error");
    let line = ["fatal: ", "error: "].iter().fold(line, |rest, prefix| {
        rest.strip_prefix(prefix).unwrap_or(rest)
    });
    line.chars().take(MAX_MESSAGE_CHARS).collect()
}

/// Accepts https:// and SSH addresses, or a folder path (a repository on a network share).
/// Everything else is refused, including `ext::` and addresses carrying a password or token.
pub fn check_remote(raw: &str) -> Result<String, AppError> {
    let url = raw.trim();
    if url.is_empty() {
        return Err(AppError::invalid("Enter the repository address"));
    }
    if url.len() > MAX_URL_LENGTH
        || url.starts_with('-')
        || url.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(AppError::invalid("The repository address is not valid"));
    }
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("ssh://") {
        let parsed = Url::parse(url)
            .map_err(|_| AppError::invalid("The repository address is not valid"))?;
        if parsed.password().is_some()
            || (lower.starts_with("https://") && !parsed.username().is_empty())
        {
            return Err(AppError::invalid(
                "Leave the account and token out of the address; Git signs in for you",
            ));
        }
        if parsed.host_str().is_none() {
            return Err(AppError::invalid("The repository address has no host"));
        }
        return Ok(url.to_owned());
    }
    if is_scp_address(url) || Path::new(url).is_absolute() {
        return Ok(url.to_owned());
    }
    Err(AppError::invalid(
        "Use an https:// or SSH address (git@host:team/repo.git), or a folder path on a network share",
    ))
}

/// `git@github.com:team/repo.git`
fn is_scp_address(url: &str) -> bool {
    let Some((user_host, path)) = url.split_once(':') else {
        return false;
    };
    let Some((user, host)) = user_host.split_once('@') else {
        return false;
    };
    !user.is_empty()
        && !host.is_empty()
        && !path.is_empty()
        && !url.contains("://")
        && !url.contains('\\')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
}

/// The account to sign in with on an https remote, or `None` when left empty. SSH and folder
/// remotes do not sign in with a username, so one given for them is refused.
pub fn check_username(username: Option<&str>, remote: &str) -> Result<Option<String>, AppError> {
    let Some(name) = username.map(str::trim).filter(|name| !name.is_empty()) else {
        return Ok(None);
    };
    if host_of(remote).is_none() {
        return Err(AppError::invalid(
            "Only https:// addresses use a Git account. SSH addresses sign in with your SSH key",
        ));
    }
    check_account_name(name).map(Some)
}

/// A username as Git hosts write them: letters, digits and `. _ - @ +`, not starting with `-`.
pub fn check_account_name(raw: &str) -> Result<String, AppError> {
    let name = raw.trim();
    let valid = !name.is_empty()
        && name.chars().count() <= MAX_USERNAME_CHARS
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@' | '+'));
    if valid {
        Ok(name.to_owned())
    } else {
        Err(AppError::invalid(
            "Enter the account's username without spaces (up to 100 characters)",
        ))
    }
}

/// A host name such as `github.com`, in lower case.
pub fn check_host(raw: &str) -> Result<String, AppError> {
    let host = raw.trim().to_ascii_lowercase();
    let valid = !host.is_empty()
        && host.len() <= MAX_HOST_CHARS
        && !host.starts_with(['.', '-'])
        && !host.ends_with(['.', '-'])
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'));
    if valid {
        Ok(host)
    } else {
        Err(AppError::invalid(
            "Enter the host name only, such as github.com",
        ))
    }
}

/// The host of an https remote, in lower case; other remotes have no sign-in account.
pub fn host_of(remote: &str) -> Option<String> {
    if !remote.to_ascii_lowercase().starts_with("https://") {
        return None;
    }
    Url::parse(remote)
        .ok()?
        .host_str()
        .map(str::to_ascii_lowercase)
}

/// The author with surrounding spaces removed, or why it cannot be used.
pub fn check_author(author: &Author) -> Result<Author, AppError> {
    let name = author.name.trim();
    let email = author.email.trim();
    let bad_name = name.is_empty()
        || name.chars().count() > MAX_AUTHOR_CHARS
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, '<' | '>'));
    if bad_name {
        return Err(AppError::invalid("Enter your name (up to 100 characters)"));
    }
    let bad_email = email.len() > MAX_EMAIL_CHARS
        || email
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '<' | '>'))
        || !email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    if bad_email {
        return Err(AppError::invalid("Enter a valid email address"));
    }
    Ok(Author {
        name: name.to_owned(),
        email: email.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https_ssh_and_folder_remotes_only() {
        assert!(check_remote("https://github.com/team/api.git").is_ok());
        assert!(check_remote("ssh://git@github.com/team/api.git").is_ok());
        assert!(check_remote("git@github.com:team/api.git").is_ok());
        assert!(check_remote("\\\\server\\share\\api.git").is_ok());
        assert!(check_remote("C:\\repos\\api.git").is_ok());

        assert!(check_remote("ext::sh -c touch% /tmp/pwned").is_err());
        assert!(check_remote("file:///C:/repos/api.git").is_err());
        assert!(check_remote("http://github.com/team/api.git").is_err());
        assert!(check_remote("--upload-pack=calc.exe").is_err());
        assert!(check_remote("https://user:token@github.com/team/api.git").is_err());
        assert!(check_remote("https://token@github.com/team/api.git").is_err());
        assert!(check_remote("relative/path").is_err());
        assert!(check_remote("").is_err());
    }

    #[test]
    fn tells_failures_apart() {
        assert_eq!(
            classify("fatal: unable to access 'https://x/': Could not resolve host: x"),
            GitError::Offline
        );
        assert_eq!(
            classify("remote: Write access to repository not granted.\nfatal: unable to access"),
            GitError::ReadOnly
        );
        assert_eq!(
            classify("fatal: Authentication failed for 'https://x/'"),
            GitError::SignIn
        );
        assert_eq!(
            classify("remote: Repository not found.\nfatal: repository 'x' not found"),
            GitError::NotFound
        );
        assert_eq!(
            classify(" ! [rejected]        main -> main (fetch first)"),
            GitError::Rejected
        );
        assert_eq!(
            classify("hint: something\nfatal: bad thing happened"),
            GitError::Failed("bad thing happened".to_owned())
        );
    }

    #[test]
    fn checks_the_commit_author() {
        let author = |name: &str, email: &str| Author {
            name: name.to_owned(),
            email: email.to_owned(),
        };
        assert!(check_author(&author("Rina", "rina@example.com")).is_ok());
        assert!(check_author(&author("", "rina@example.com")).is_err());
        assert!(check_author(&author("Rina <x>", "rina@example.com")).is_err());
        assert!(check_author(&author("Rina", "not-an-email")).is_err());
        assert!(check_author(&author("Rina", "rina @example.com")).is_err());
    }

    #[test]
    fn checks_the_sign_in_account() {
        let https = "https://github.com/team/api.git";
        assert_eq!(check_username(None, https).ok(), Some(None));
        assert_eq!(check_username(Some("  "), https).ok(), Some(None));
        assert_eq!(
            check_username(Some(" rina-work "), https).ok(),
            Some(Some("rina-work".to_owned()))
        );
        assert!(check_username(Some("rina@acme.co.id"), https).is_ok());
        assert!(check_username(Some("rina work"), https).is_err());
        assert!(check_username(Some("-c"), https).is_err());
        assert!(check_username(Some("rina\nx"), https).is_err());
        assert!(check_username(Some(&"r".repeat(101)), https).is_err());
        assert!(check_username(Some("rina"), "git@github.com:team/api.git").is_err());
        assert!(check_username(Some("rina"), "C:\\repos\\api.git").is_err());
        assert_eq!(
            check_username(None, "git@github.com:team/api.git").ok(),
            Some(None)
        );
    }

    #[test]
    fn checks_hosts_and_reads_them_from_remotes() {
        assert_eq!(
            check_host(" GitHub.com ").ok().as_deref(),
            Some("github.com")
        );
        assert!(check_host("gitlab.acme.co.id").is_ok());
        assert!(check_host("").is_err());
        assert!(check_host("https://github.com").is_err());
        assert!(check_host("github.com\nusername=x").is_err());
        assert!(check_host("-github.com").is_err());
        assert_eq!(
            host_of("https://GitHub.com/team/api.git").as_deref(),
            Some("github.com")
        );
        assert_eq!(host_of("git@github.com:team/api.git"), None);
        assert_eq!(host_of("C:\\repos\\api.git"), None);
    }

    #[tokio::test]
    async fn network_calls_ask_for_the_chosen_account() {
        let dir = std::env::temp_dir();
        if version(&dir).await.is_none() {
            return;
        }
        let asked = Git::signed_in_as(dir.clone(), Some("rina-work"))
            .network(&["config", "--get", "credential.username"])
            .await
            .expect("setting");
        assert_eq!(asked.trim(), "rina-work");
        let local = Git::signed_in_as(dir, Some("rina-work"))
            .run(&["config", "--get", "credential.username"])
            .await
            .unwrap_or_default();
        assert_ne!(local.trim(), "rina-work");
    }
}

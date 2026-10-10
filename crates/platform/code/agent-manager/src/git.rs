//! Strict git execution boundary (M06-A, M06-E, M06-14).
//!
//! Every git invocation in BAM goes through [`run_in`]. There is no shell, so no
//! argument can be interpreted as a command, and destructive operations are refused
//! before a process starts: `--force`, `--force-with-lease`, `-f`, `--hard`,
//! `--mirror` and `--no-verify` are rejected, and `clean`, `reset` and
//! `filter-branch` are not part of the allowed subcommand set.
use std::{
    collections::BTreeMap,
    path::Path,
    process::{Command, Stdio},
};

/// Subcommands BAM is allowed to run. Destructive history/subcommand pairs are
/// deliberately absent rather than merely filtered.
pub const ALLOWED_SUBCOMMANDS: [&str; 24] = [
    "add",
    "branch",
    "cat-file",
    "checkout",
    "clone",
    "commit",
    "config",
    "diff",
    "fetch",
    "for-each-ref",
    "init",
    "log",
    "ls-remote",
    "merge-base",
    "rebase",
    "remote",
    "rev-list",
    "rev-parse",
    "show-ref",
    "status",
    "switch",
    "symbolic-ref",
    "worktree",
    "push",
];

/// Flags that may never reach a git process, whatever the caller asks for.
pub const FORBIDDEN_FLAGS: [&str; 5] = [
    "-f",
    "--hard",
    "--mirror",
    "--no-verify",
    "--force-with-lease",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitError {
    EmptyCommand,
    UnknownSubcommand(String),
    ForbiddenArgument(String),
    InvalidArgument(String),
    NothingToCommit,
    Failed {
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },
    Io(String),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyCommand => formatter.write_str("empty git command"),
            Self::UnknownSubcommand(subcommand) => {
                write!(formatter, "git subcommand {subcommand:?} is not allowed")
            }
            Self::ForbiddenArgument(argument) => {
                write!(formatter, "git argument {argument:?} is forbidden")
            }
            Self::InvalidArgument(message) => write!(formatter, "invalid git argument: {message}"),
            Self::NothingToCommit => formatter.write_str("nothing to commit"),
            Self::Failed {
                command,
                exit_code,
                stderr,
            } => write!(
                formatter,
                "git {command} failed (exit {exit_code:?}): {}",
                stderr.trim()
            ),
            Self::Io(message) => write!(formatter, "git could not be executed: {message}"),
        }
    }
}

impl std::error::Error for GitError {}

/// Validate an argument vector before any process is spawned.
pub fn validate(args: &[&str]) -> Result<(), GitError> {
    if args.is_empty() {
        return Err(GitError::EmptyCommand);
    }

    // Allow a leading `-c key=value` configuration prefix, then require a known subcommand.
    let mut index = 0;
    while index < args.len() && args[index] == "-c" {
        let setting = args
            .get(index + 1)
            .ok_or_else(|| GitError::InvalidArgument("-c requires key=value".into()))?;
        if !setting.contains('=') {
            return Err(GitError::InvalidArgument(format!(
                "-c setting {setting:?} is not key=value"
            )));
        }
        index += 2;
    }
    let subcommand = args
        .get(index)
        .ok_or_else(|| GitError::InvalidArgument("no git subcommand was given".into()))?;
    if !ALLOWED_SUBCOMMANDS.contains(subcommand) {
        return Err(GitError::UnknownSubcommand((*subcommand).to_owned()));
    }

    for token in args {
        if token.is_empty() || token.contains('\0') || token.contains('\n') || token.contains('\r')
        {
            return Err(GitError::InvalidArgument(
                "git arguments must be non-empty single-line values".into(),
            ));
        }
        let lowered = token.to_ascii_lowercase();
        if FORBIDDEN_FLAGS.contains(&lowered.as_str()) || lowered.starts_with("--force") {
            return Err(GitError::ForbiddenArgument((*token).to_owned()));
        }
    }
    Ok(())
}

/// Run a validated git command inside `directory` with extra environment values.
/// Stdout is captured; stdin is closed.
pub fn run_with_env(
    directory: &Path,
    args: &[&str],
    environment: &BTreeMap<String, String>,
) -> Result<String, GitError> {
    validate(args)?;
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in environment {
        command.env(key, value);
    }
    let output = command
        .output()
        .map_err(|error| GitError::Io(error.to_string()))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if output.status.success() {
        return Ok(stdout);
    }
    if stderr.contains("nothing to commit")
        || stdout.contains("nothing to commit")
        || stderr.contains("no changes added to commit")
    {
        return Err(GitError::NothingToCommit);
    }
    Err(GitError::Failed {
        command: args.join(" "),
        exit_code: output.status.code(),
        stderr,
    })
}

/// Run a validated git command inside `directory`.
pub fn run_in(directory: &Path, args: &[&str]) -> Result<String, GitError> {
    run_with_env(directory, args, &BTreeMap::new())
}

/// Deterministic author/committer identity so commits do not depend on the ambient
/// git configuration of the machine running BAM.
pub fn commit_identity() -> BTreeMap<String, String> {
    let mut environment = BTreeMap::new();
    environment.insert("GIT_AUTHOR_NAME".into(), "BurnCloud Agent Manager".into());
    environment.insert(
        "GIT_AUTHOR_EMAIL".into(),
        "bam@users.noreply.github.com".into(),
    );
    environment.insert(
        "GIT_COMMITTER_NAME".into(),
        "BurnCloud Agent Manager".into(),
    );
    environment.insert(
        "GIT_COMMITTER_EMAIL".into(),
        "bam@users.noreply.github.com".into(),
    );
    environment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destructive_arguments_are_rejected_before_execution() {
        for forbidden in [
            vec!["push", "--force", "origin", "main"],
            vec!["push", "-f", "origin", "main"],
            vec!["push", "--force-with-lease", "origin", "main"],
            vec!["reset", "--hard", "HEAD~1"],
            vec!["clean", "-fdx"],
        ] {
            assert!(
                validate(&forbidden).is_err(),
                "{forbidden:?} must be refused"
            );
        }
    }

    #[test]
    fn only_allowed_subcommands_are_accepted() {
        assert!(validate(&["status", "--porcelain"]).is_ok());
        assert!(validate(&["worktree", "add", "/tmp/x", "main"]).is_ok());
        assert!(validate(&["-c", "user.name=x", "commit", "-m", "msg"]).is_ok());
        assert_eq!(
            validate(&["filter-branch", "--all"]),
            Err(GitError::UnknownSubcommand("filter-branch".into()))
        );
        assert_eq!(validate(&[]), Err(GitError::EmptyCommand));
        assert!(validate(&["-c", "broken", "commit"]).is_err());
        assert!(validate(&["status", "two\nlines"]).is_err());
    }

    #[test]
    fn running_git_outside_a_repository_fails_cleanly() {
        let temp = tempfile::tempdir().unwrap();
        let error = run_in(temp.path(), &["rev-parse", "--git-dir"]).unwrap_err();
        assert!(matches!(error, GitError::Failed { .. }));
    }
}

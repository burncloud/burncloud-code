//! Worktree and branch lifecycle (M06-01..M06-08, M06-11..M06-14).
//!
//! One managed worktree per task, always created from a freshly fetched remote base
//! sha. Pushes are never forced: when the remote branch moved, the push is reported as
//! rejected and the remote head is left untouched.
use crate::git::{self, GitError};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worktree {
    pub task_id: String,
    pub branch: String,
    pub path: PathBuf,
    /// Base commit the worktree was created from.
    pub base_sha: String,
    /// True when an existing remote branch was adopted instead of creating a new one.
    pub adopted: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceStatus {
    pub staged: Vec<String>,
    pub modified: Vec<String>,
    pub untracked: Vec<String>,
}

impl WorkspaceStatus {
    pub fn is_dirty(&self) -> bool {
        !self.staged.is_empty() || !self.modified.is_empty() || !self.untracked.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub insertions: u64,
    pub deletions: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffSummary {
    pub files: Vec<FileChange>,
    pub insertions: u64,
    pub deletions: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushOutcome {
    /// The remote branch now points at `head_sha`.
    Updated { head_sha: String },
    /// The remote branch moved; nothing was overwritten and no force was attempted.
    Rejected {
        reason: String,
        remote_head: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebaseOutcome {
    Clean {
        head_sha: String,
    },
    /// Conflicts were reported and the rebase was aborted, leaving the branch intact.
    Conflicts {
        files: Vec<String>,
    },
}

/// A repository handle. All commands run through the validated git boundary.
#[derive(Debug, Clone)]
pub struct GitRepository {
    root: PathBuf,
}

impl GitRepository {
    /// Open an existing repository, verifying it really is one.
    pub fn open(root: &Path) -> Result<Self, GitError> {
        let repository = Self {
            root: root.to_path_buf(),
        };
        git::run_in(&repository.root, &["rev-parse", "--git-dir"])?;
        Ok(repository)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn fetch(&self, remote: &str, branch: &str) -> Result<String, GitError> {
        validate_name(remote)?;
        validate_name(branch)?;
        git::run_in(&self.root, &["fetch", remote, branch])?;
        let sha = git::run_in(&self.root, &["rev-parse", "FETCH_HEAD"])?;
        Ok(sha.trim().to_owned())
    }

    /// Current head of a remote branch, or `None` when it does not exist yet (M06-11).
    pub fn remote_branch_head(
        &self,
        remote: &str,
        branch: &str,
    ) -> Result<Option<String>, GitError> {
        validate_name(remote)?;
        validate_name(branch)?;
        let output = git::run_in(
            &self.root,
            &["ls-remote", remote, &format!("refs/heads/{branch}")],
        )?;
        Ok(output.split_whitespace().next().map(|sha| sha.to_owned()))
    }

    pub fn resolve_commit(&self, revision: &str) -> Result<String, GitError> {
        validate_name(revision)?;
        let output = git::run_in(&self.root, &["rev-parse", revision])?;
        Ok(output.trim().to_owned())
    }

    pub fn local_branch_head(&self, branch: &str) -> Result<Option<String>, GitError> {
        validate_name(branch)?;
        match git::run_in(&self.root, &["rev-parse", &format!("refs/heads/{branch}")]) {
            Ok(output) => Ok(Some(output.trim().to_owned())),
            Err(GitError::Failed { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn worktree_paths(&self) -> Result<Vec<PathBuf>, GitError> {
        let output = git::run_in(&self.root, &["worktree", "list", "--porcelain"])?;
        Ok(output
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .map(PathBuf::from)
            .collect())
    }

    pub fn status(&self, worktree: &Path) -> Result<WorkspaceStatus, GitError> {
        let output = git::run_in(worktree, &["status", "--porcelain"])?;
        let mut status = WorkspaceStatus::default();
        for line in output.lines() {
            if line.len() < 3 {
                continue;
            }
            let (code, path) = line.split_at(2);
            let path = path.trim().to_owned();
            if code.starts_with("??") {
                status.untracked.push(path);
            } else if code.starts_with(' ') {
                status.modified.push(path);
            } else {
                status.staged.push(path);
            }
        }
        Ok(status)
    }

    pub fn diff_summary(&self, worktree: &Path, base_sha: &str) -> Result<DiffSummary, GitError> {
        validate_name(base_sha)?;
        let output = git::run_in(worktree, &["diff", "--numstat", base_sha])?;
        let mut summary = DiffSummary::default();
        for line in output.lines() {
            let mut parts = line.split('\t');
            let insertions = parts.next().unwrap_or("0");
            let deletions = parts.next().unwrap_or("0");
            let Some(path) = parts.next() else { continue };
            let insertions = insertions.parse::<u64>().unwrap_or(0);
            let deletions = deletions.parse::<u64>().unwrap_or(0);
            summary.files.push(FileChange {
                path: path.to_owned(),
                insertions,
                deletions,
            });
            summary.insertions += insertions;
            summary.deletions += deletions;
        }
        Ok(summary)
    }

    pub fn stage_all(&self, worktree: &Path) -> Result<(), GitError> {
        git::run_in(worktree, &["add", "--all"])?;
        Ok(())
    }

    /// Commit staged changes and return the created sha (M06-07).
    pub fn commit(&self, worktree: &Path, message: &str) -> Result<String, GitError> {
        if message.trim().is_empty() {
            return Err(GitError::InvalidArgument(
                "commit message must not be empty".into(),
            ));
        }
        git::run_with_env(
            worktree,
            &["commit", "-m", message],
            &git::commit_identity(),
        )?;
        let sha = git::run_in(worktree, &["rev-parse", "HEAD"])?;
        Ok(sha.trim().to_owned())
    }

    pub fn head_sha(&self, worktree: &Path) -> Result<String, GitError> {
        let output = git::run_in(worktree, &["rev-parse", "HEAD"])?;
        Ok(output.trim().to_owned())
    }

    /// Push a branch without ever forcing it (M06-08, M06-11, M06-14).
    pub fn push(
        &self,
        worktree: &Path,
        remote: &str,
        branch: &str,
    ) -> Result<PushOutcome, GitError> {
        validate_name(remote)?;
        validate_name(branch)?;
        let head_sha = self.head_sha(worktree)?;
        let before = self.remote_branch_head(remote, branch)?;
        match git::run_in(
            worktree,
            &[
                "push",
                "--set-upstream",
                remote,
                &format!("refs/heads/{branch}"),
            ],
        ) {
            Ok(_) => Ok(PushOutcome::Updated { head_sha }),
            Err(GitError::Failed { stderr, .. }) => {
                let after = self.remote_branch_head(remote, branch)?;
                let reason = if stderr.contains("non-fast-forward")
                    || stderr.contains("rejected")
                    || stderr.contains("fetch first")
                {
                    "remote branch moved; a force push was not attempted".to_owned()
                } else {
                    stderr.trim().to_owned()
                };
                debug_assert_eq!(before, after, "a rejected push must not change the remote");
                Ok(PushOutcome::Rejected {
                    reason,
                    remote_head: after,
                })
            }
            Err(error) => Err(error),
        }
    }

    /// Rebase onto a commit. Conflicts are reported and the rebase is aborted so the
    /// branch keeps its original content (M06-12).
    pub fn rebase_onto(&self, worktree: &Path, onto_sha: &str) -> Result<RebaseOutcome, GitError> {
        validate_name(onto_sha)?;
        match git::run_in(worktree, &["rebase", onto_sha]) {
            Ok(_) => Ok(RebaseOutcome::Clean {
                head_sha: self.head_sha(worktree)?,
            }),
            Err(GitError::Failed { .. }) => {
                let conflicted = git::run_in(worktree, &["diff", "--name-only", "--diff-filter=U"])
                    .unwrap_or_default()
                    .lines()
                    .map(|line| line.trim().to_owned())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>();
                let _ = git::run_in(worktree, &["rebase", "--abort"]);
                Ok(RebaseOutcome::Conflicts { files: conflicted })
            }
            Err(error) => Err(error),
        }
    }

    /// Remove a worktree. Refuses to discard unsaved changes (M06-13).
    pub fn remove_worktree(&self, worktree: &Path) -> Result<(), GitError> {
        let status = self.status(worktree)?;
        if status.is_dirty() {
            return Err(GitError::InvalidArgument(format!(
                "refusing to remove {} with unsaved changes",
                worktree.display()
            )));
        }
        git::run_in(
            &self.root,
            &["worktree", "remove", &worktree.to_string_lossy()],
        )?;
        Ok(())
    }
}

/// Allocates managed worktrees under one root, one per task (M06-01, M06-B).
#[derive(Debug, Clone)]
pub struct WorktreeManager {
    repository: GitRepository,
    managed_root: PathBuf,
}

impl WorktreeManager {
    pub fn new(repository: GitRepository, managed_root: PathBuf) -> Self {
        Self {
            repository,
            managed_root,
        }
    }

    pub fn repository(&self) -> &GitRepository {
        &self.repository
    }

    pub fn worktree_path(&self, task_id: &str) -> Result<PathBuf, GitError> {
        validate_name(task_id)?;
        Ok(self.managed_root.join(task_id))
    }

    /// Create (or reuse) the worktree for a task.
    ///
    /// A branch that already exists on the remote is adopted so that an existing PR is
    /// continued instead of duplicated (M06-03); otherwise the branch is created from
    /// the freshly fetched base sha (M06-02, M06-04).
    pub fn ensure_worktree(
        &self,
        task_id: &str,
        branch: &str,
        remote: &str,
        base_branch: &str,
    ) -> Result<Worktree, GitError> {
        validate_name(task_id)?;
        validate_name(branch)?;
        let path = self.worktree_path(task_id)?;
        let remote_head = self.repository.remote_branch_head(remote, branch)?;

        // Already provisioned for this task: never create a second worktree.
        if path.exists() {
            return Ok(Worktree {
                task_id: task_id.to_owned(),
                branch: branch.to_owned(),
                path,
                base_sha: self
                    .repository
                    .local_branch_head(branch)?
                    .unwrap_or_default(),
                adopted: remote_head.is_some(),
            });
        }

        std::fs::create_dir_all(&self.managed_root)
            .map_err(|error| GitError::Io(error.to_string()))?;

        // A branch that already exists (locally or on the remote) is adopted so that an
        // existing PR is continued instead of duplicated (M06-03).
        if self.repository.local_branch_head(branch)?.is_some() {
            git::run_in(
                self.repository.root(),
                &["worktree", "add", &path.to_string_lossy(), branch],
            )?;
            return Ok(Worktree {
                task_id: task_id.to_owned(),
                branch: branch.to_owned(),
                path,
                base_sha: self
                    .repository
                    .local_branch_head(branch)?
                    .unwrap_or_default(),
                adopted: true,
            });
        }

        if remote_head.is_some() {
            git::run_in(
                self.repository.root(),
                &[
                    "fetch",
                    remote,
                    &format!("refs/heads/{branch}:refs/remotes/{remote}/{branch}"),
                ],
            )?;
            git::run_in(
                self.repository.root(),
                &[
                    "worktree",
                    "add",
                    "--track",
                    "-b",
                    branch,
                    &path.to_string_lossy(),
                    &format!("refs/remotes/{remote}/{branch}"),
                ],
            )?;
            return Ok(Worktree {
                task_id: task_id.to_owned(),
                branch: branch.to_owned(),
                path,
                base_sha: self
                    .repository
                    .local_branch_head(branch)?
                    .unwrap_or_default(),
                adopted: true,
            });
        }

        let base_sha = self.repository.fetch(remote, base_branch)?;
        git::run_in(
            self.repository.root(),
            &[
                "worktree",
                "add",
                "-b",
                branch,
                &path.to_string_lossy(),
                &base_sha,
            ],
        )?;
        Ok(Worktree {
            task_id: task_id.to_owned(),
            branch: branch.to_owned(),
            path,
            base_sha,
            adopted: false,
        })
    }

    /// Verify no other managed worktree already owns this path (M06-01).
    pub fn assert_exclusive(&self, worktree: &Worktree) -> Result<(), GitError> {
        let owners = self
            .repository
            .worktree_paths()?
            .into_iter()
            .filter(|path| path == &worktree.path)
            .count();
        if owners == 1 {
            Ok(())
        } else {
            Err(GitError::InvalidArgument(format!(
                "worktree {} is registered {owners} times",
                worktree.path.display()
            )))
        }
    }

    pub fn cleanup(&self, worktree: &Worktree) -> Result<(), GitError> {
        self.repository.remove_worktree(&worktree.path)
    }
}

/// Path/name segments are restricted so a task id can never escape the managed root.
fn validate_name(value: &str) -> Result<(), GitError> {
    let valid = !value.is_empty()
        && value.len() <= 200
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
        && !value.contains("..")
        && !value.starts_with('/');
    if valid {
        Ok(())
    } else {
        Err(GitError::InvalidArgument(format!(
            "{value:?} is not a safe git name"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    struct Fixture {
        _temp: tempfile::TempDir,
        origin: PathBuf,
        seed: PathBuf,
    }

    /// Deterministic identity and signing configuration for test repositories, so the
    /// suite does not depend on the ambient git configuration of the machine.
    fn harness_env() -> BTreeMap<String, String> {
        let mut environment = git::commit_identity();
        environment.insert("GIT_CONFIG_COUNT".into(), "1".into());
        environment.insert("GIT_CONFIG_KEY_0".into(), "commit.gpgsign".into());
        environment.insert("GIT_CONFIG_VALUE_0".into(), "false".into());
        environment
    }

    fn run(directory: &Path, args: &[&str]) -> String {
        git::run_with_env(directory, args, &harness_env())
            .unwrap_or_else(|error| panic!("git {args:?}: {error}"))
    }

    fn fixture() -> Fixture {
        let temp = tempfile::tempdir().unwrap();
        let origin = temp.path().join("origin.git");
        let seed = temp.path().join("seed");
        std::fs::create_dir_all(&origin).unwrap();
        std::fs::create_dir_all(&seed).unwrap();

        git::run_in(&origin, &["init", "--bare", "--initial-branch=main"]).unwrap();
        run(&seed, &["init", "--initial-branch=main"]);
        run(&seed, &["config", "core.autocrlf", "false"]);
        std::fs::write(seed.join("README.md"), "seed\n").unwrap();
        run(&seed, &["add", "--all"]);
        run(&seed, &["commit", "-m", "seed"]);
        run(
            &seed,
            &["remote", "add", "origin", &origin.to_string_lossy()],
        );
        run(&seed, &["push", "--set-upstream", "origin", "main"]);

        Fixture {
            _temp: temp,
            origin,
            seed,
        }
    }

    #[test]
    fn worktree_is_created_from_the_fetched_base_sha() {
        let fixture = fixture();
        let repository = GitRepository::open(&fixture.seed).unwrap();
        let manager = WorktreeManager::new(repository.clone(), fixture.seed.join("managed"));

        let worktree = manager
            .ensure_worktree("task-1", "bam/task-1", "origin", "main")
            .unwrap();
        assert!(!worktree.adopted);
        assert!(worktree.path.join("README.md").exists());
        assert_eq!(
            worktree.base_sha,
            repository.resolve_commit("HEAD").unwrap()
        );
        assert!(!manager
            .repository()
            .status(&worktree.path)
            .unwrap()
            .is_dirty());

        // The same task never gets a second worktree.
        let again = manager
            .ensure_worktree("task-1", "bam/task-1", "origin", "main")
            .unwrap();
        assert_eq!(again.path, worktree.path);
        assert!(manager.assert_exclusive(&worktree).is_ok());
        assert_eq!(repository.worktree_paths().unwrap().len(), 2);
    }

    #[test]
    fn an_existing_remote_branch_is_adopted_instead_of_duplicated() {
        let fixture = fixture();
        let repository = GitRepository::open(&fixture.seed).unwrap();
        let manager = WorktreeManager::new(repository.clone(), fixture.seed.join("managed"));

        // Publish the branch first, as an existing PR would have done.
        git::run_in(&fixture.seed, &["branch", "bam/existing"]).unwrap();
        run(&fixture.seed, &["push", "origin", "bam/existing"]);

        let worktree = manager
            .ensure_worktree("task-2", "bam/existing", "origin", "main")
            .unwrap();
        assert!(worktree.adopted);
        assert_eq!(
            worktree.base_sha,
            repository
                .remote_branch_head("origin", "bam/existing")
                .unwrap()
                .unwrap()
        );
    }

    #[test]
    fn commit_and_diff_summary_describe_the_change() {
        let fixture = fixture();
        let repository = GitRepository::open(&fixture.seed).unwrap();
        let manager = WorktreeManager::new(repository.clone(), fixture.seed.join("managed"));
        let worktree = manager
            .ensure_worktree("task-3", "bam/task-3", "origin", "main")
            .unwrap();

        std::fs::write(worktree.path.join("feature.rs"), "fn main() {}\n").unwrap();
        assert!(repository.status(&worktree.path).unwrap().is_dirty());
        assert!(repository.commit(&worktree.path, "").is_err());

        repository.stage_all(&worktree.path).unwrap();
        let sha = repository
            .commit(&worktree.path, "feat: add feature")
            .unwrap();
        assert_eq!(sha.len(), 40);
        assert!(!repository.status(&worktree.path).unwrap().is_dirty());

        let summary = repository
            .diff_summary(&worktree.path, &worktree.base_sha)
            .unwrap();
        assert_eq!(summary.files.len(), 1);
        assert_eq!(summary.files[0].path, "feature.rs");
        assert_eq!(summary.insertions, 1);
    }

    #[test]
    fn a_rejected_push_never_overwrites_the_remote() {
        let fixture = fixture();
        let repository = GitRepository::open(&fixture.seed).unwrap();
        let manager = WorktreeManager::new(repository.clone(), fixture.seed.join("managed"));
        let worktree = manager
            .ensure_worktree("task-4", "bam/task-4", "origin", "main")
            .unwrap();

        std::fs::write(worktree.path.join("one.txt"), "one\n").unwrap();
        repository.stage_all(&worktree.path).unwrap();
        let first = repository.commit(&worktree.path, "feat: first").unwrap();
        assert_eq!(
            repository
                .push(&worktree.path, "origin", "bam/task-4")
                .unwrap(),
            PushOutcome::Updated {
                head_sha: first.clone()
            }
        );

        // Another worker advances the same remote branch.
        let other = fixture._temp.path().join("other");
        run(
            fixture._temp.path(),
            &[
                "clone",
                &fixture.origin.to_string_lossy(),
                &other.to_string_lossy(),
            ],
        );
        run(&other, &["fetch", "origin", "bam/task-4"]);
        run(&other, &["switch", "-c", "bam/task-4", "FETCH_HEAD"]);
        std::fs::write(other.join("two.txt"), "two\n").unwrap();
        run(&other, &["add", "--all"]);
        run(&other, &["commit", "-m", "feat: other worker"]);
        run(&other, &["push", "origin", "bam/task-4"]);
        let other_head = run(&other, &["rev-parse", "HEAD"]).trim().to_owned();

        // Our worktree now diverges; the push must be refused, not forced.
        std::fs::write(worktree.path.join("three.txt"), "three\n").unwrap();
        repository.stage_all(&worktree.path).unwrap();
        repository.commit(&worktree.path, "feat: third").unwrap();
        match repository
            .push(&worktree.path, "origin", "bam/task-4")
            .unwrap()
        {
            PushOutcome::Rejected { remote_head, .. } => {
                assert_eq!(remote_head.as_deref(), Some(other_head.as_str()));
            }
            other_outcome => panic!("expected a rejected push, got {other_outcome:?}"),
        }
        assert_eq!(
            repository
                .remote_branch_head("origin", "bam/task-4")
                .unwrap()
                .unwrap(),
            other_head,
            "the other worker's commit must still be the remote head"
        );
    }

    #[test]
    fn cleanup_refuses_to_discard_unsaved_changes() {
        let fixture = fixture();
        let repository = GitRepository::open(&fixture.seed).unwrap();
        let manager = WorktreeManager::new(repository.clone(), fixture.seed.join("managed"));
        let worktree = manager
            .ensure_worktree("task-5", "bam/task-5", "origin", "main")
            .unwrap();

        std::fs::write(worktree.path.join("scratch.txt"), "unsaved\n").unwrap();
        assert!(manager.cleanup(&worktree).is_err());
        assert!(worktree.path.exists());

        std::fs::remove_file(worktree.path.join("scratch.txt")).unwrap();
        manager.cleanup(&worktree).unwrap();
        assert!(!worktree.path.exists());
    }

    #[test]
    fn rebase_reports_conflicts_and_leaves_the_branch_intact() {
        let fixture = fixture();
        let repository = GitRepository::open(&fixture.seed).unwrap();
        let manager = WorktreeManager::new(repository.clone(), fixture.seed.join("managed"));
        let worktree = manager
            .ensure_worktree("task-6", "bam/task-6", "origin", "main")
            .unwrap();

        let before = repository.head_sha(&worktree.path).unwrap();
        // Rebasing an unchanged branch onto its own base is clean.
        match repository
            .rebase_onto(&worktree.path, &worktree.base_sha)
            .unwrap()
        {
            RebaseOutcome::Clean { head_sha } => assert_eq!(head_sha, before),
            outcome => panic!("expected a clean rebase, got {outcome:?}"),
        }

        // A genuinely conflicting change is reported and the branch is left intact.
        std::fs::write(fixture.seed.join("README.md"), "upstream change\n").unwrap();
        run(&fixture.seed, &["add", "--all"]);
        run(&fixture.seed, &["commit", "-m", "chore: upstream"]);
        run(&fixture.seed, &["push", "origin", "main"]);
        let new_base = repository.fetch("origin", "main").unwrap();

        std::fs::write(worktree.path.join("README.md"), "task change\n").unwrap();
        repository.stage_all(&worktree.path).unwrap();
        let task_head = repository
            .commit(&worktree.path, "feat: conflicting task change")
            .unwrap();

        match repository.rebase_onto(&worktree.path, &new_base).unwrap() {
            RebaseOutcome::Conflicts { files } => {
                assert_eq!(files, vec!["README.md".to_owned()]);
            }
            outcome => panic!("expected reported conflicts, got {outcome:?}"),
        }
        assert_eq!(
            repository.head_sha(&worktree.path).unwrap(),
            task_head,
            "an aborted rebase must leave the branch unchanged"
        );
        assert!(!repository.status(&worktree.path).unwrap().is_dirty());
    }

    #[test]
    fn unsafe_task_names_are_rejected() {
        let fixture = fixture();
        let repository = GitRepository::open(&fixture.seed).unwrap();
        let manager = WorktreeManager::new(repository, fixture.seed.join("managed"));
        assert!(manager.worktree_path("../escape").is_err());
        assert!(manager.worktree_path("").is_err());
        assert!(manager
            .ensure_worktree("ok", "bam/x", "origin", "main")
            .is_ok());
    }
}

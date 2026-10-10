//! GitHub read-only adapter using `gh api`. No token is included in command args.
use serde::Deserialize;
use std::{
    collections::hash_map::DefaultHasher,
    env, fmt,
    hash::{Hash, Hasher},
    process::{Command, Stdio},
};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PullRequestMarker {}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RemoteIssue {
    #[serde(default)]
    pub pull_request: Option<PullRequestMarker>,
    pub id: u64,
    pub node_id: String,
    pub number: u64,
    pub title: String,
    pub state: String,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RemotePr {
    pub id: u64,
    pub node_id: String,
    pub number: u64,
    pub state: String,
    pub draft: bool,
    pub head: PrHead,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PrHead {
    pub sha: String,
}

fn validate_segment(s: &str) -> Result<(), String> {
    if !s.is_empty()
        && s.len() <= 100
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        Ok(())
    } else {
        Err("invalid GitHub repository path segment".into())
    }
}
fn validate_number(n: u64) -> Result<(), String> {
    if n > 0 {
        Ok(())
    } else {
        Err("GitHub number must be positive".into())
    }
}
pub fn issue_endpoint(owner: &str, repo: &str, number: u64) -> Result<String, String> {
    validate_segment(owner)?;
    validate_segment(repo)?;
    validate_number(number)?;
    Ok(format!("repos/{owner}/{repo}/issues/{number}"))
}
pub fn pr_endpoint(owner: &str, repo: &str, number: u64) -> Result<String, String> {
    validate_segment(owner)?;
    validate_segment(repo)?;
    validate_number(number)?;
    Ok(format!("repos/{owner}/{repo}/pulls/{number}"))
}
pub fn list_issues_endpoint(owner: &str, repo: &str, page: u32) -> Result<String, String> {
    validate_segment(owner)?;
    validate_segment(repo)?;
    if page == 0 || page > 10_000 {
        return Err("invalid page".into());
    }
    Ok(format!(
        "repos/{owner}/{repo}/issues?state=open&per_page=100&page={page}"
    ))
}
pub fn list_prs_endpoint(owner: &str, repo: &str, page: u32) -> Result<String, String> {
    validate_segment(owner)?;
    validate_segment(repo)?;
    if page == 0 || page > 10_000 {
        return Err("invalid page".into());
    }
    Ok(format!(
        "repos/{owner}/{repo}/pulls?state=open&per_page=100&page={page}"
    ))
}

/// Invokes exactly `gh api GET <validated-endpoint>` with a timeout controlled by
/// GitHub CLI. Not usable for mutations, arbitrary CLI execution, or shell commands.
pub fn gh_get(endpoint: &str) -> Result<String, String> {
    if endpoint.is_empty()
        || endpoint.starts_with('-')
        || !endpoint.starts_with("repos/")
        || endpoint.contains(' ')
        || endpoint.contains('\n')
        || endpoint.contains('\r')
        || endpoint.contains("..")
    {
        return Err("invalid GitHub read-only endpoint".into());
    }
    let output = Command::new("gh")
        .arg("api")
        .arg("--method")
        .arg("GET")
        .arg(endpoint)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cannot execute gh api: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "GitHub request failed (exit: {:?})",
            output.status.code()
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("invalid GitHub UTF-8: {e}"))
}
pub fn get_issue(owner: &str, repo: &str, n: u64) -> Result<RemoteIssue, String> {
    let raw = gh_get(&issue_endpoint(owner, repo, n)?)?;
    serde_json::from_str(&raw).map_err(|e| format!("invalid issue response: {e}"))
}
pub fn get_pr(owner: &str, repo: &str, n: u64) -> Result<RemotePr, String> {
    let raw = gh_get(&pr_endpoint(owner, repo, n)?)?;
    serde_json::from_str(&raw).map_err(|e| format!("invalid PR response: {e}"))
}
pub fn get_open_issues(owner: &str, repo: &str) -> Result<Vec<RemoteIssue>, String> {
    let mut items = Vec::new();
    for page in 1..=100 {
        let raw = gh_get(&list_issues_endpoint(owner, repo, page)?)?;
        let batch: Vec<RemoteIssue> = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let count = batch.len();
        for entry in batch {
            // GitHub issues endpoint includes pull requests; never treat them as issues.
            if entry.pull_request.is_none() {
                items.push(entry);
            }
        }
        if count < 100 {
            return Ok(items);
        }
    }
    Err("pagination limit reached; refusing incomplete issue results".into())
}
pub fn get_open_prs(owner: &str, repo: &str) -> Result<Vec<RemotePr>, String> {
    let mut items = Vec::new();
    for page in 1..=100 {
        let raw = gh_get(&list_prs_endpoint(owner, repo, page)?)?;
        let mut batch: Vec<RemotePr> = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let count = batch.len();
        items.append(&mut batch);
        if count < 100 {
            return Ok(items);
        }
    }
    Err("pagination limit reached; refusing incomplete PR results".into())
}

const ENV_TOKEN_KEYS: [&str; 3] = ["BAM_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"];

#[derive(Clone, PartialEq, Eq)]
pub enum CredentialSource {
    Environment(&'static str),
    GhCli,
}

impl fmt::Debug for CredentialSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Environment(name) => formatter.debug_tuple("Environment").field(name).finish(),
            Self::GhCli => formatter.write_str("GhCli"),
        }
    }
}

#[derive(Clone)]
pub struct Credential {
    secret: String,
    source: CredentialSource,
    generation: u64,
}

impl Credential {
    fn new(secret: String, source: CredentialSource) -> Result<Self, CredentialError> {
        let secret = secret.trim().to_owned();
        if secret.is_empty() {
            return Err(CredentialError::Missing);
        }
        let mut hasher = DefaultHasher::new();
        secret.hash(&mut hasher);
        Ok(Self {
            secret,
            source,
            generation: hasher.finish(),
        })
    }

    pub fn source(&self) -> &CredentialSource {
        &self.source
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn authorization_token(&self) -> &str {
        &self.secret
    }
}

impl fmt::Debug for Credential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Credential")
            .field("source", &self.source)
            .field("secret", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialError {
    Missing,
    GhUnavailable,
    GhRejected,
}

impl fmt::Display for CredentialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => formatter.write_str("GitHub credential is not configured"),
            Self::GhUnavailable => formatter.write_str("GitHub CLI is unavailable"),
            Self::GhRejected => {
                formatter.write_str("GitHub CLI did not return a usable credential")
            }
        }
    }
}

impl std::error::Error for CredentialError {}

pub trait Environment {
    fn read(&self, key: &str) -> Option<String>;
}

pub trait GhCredentialSource {
    fn token(&self) -> Result<String, CredentialError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemEnvironment;

impl Environment for SystemEnvironment {
    fn read(&self, key: &str) -> Option<String> {
        env::var(key).ok()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemGh;

impl GhCredentialSource for SystemGh {
    fn token(&self) -> Result<String, CredentialError> {
        let output = Command::new("gh")
            .args(["auth", "token"])
            .output()
            .map_err(|_| CredentialError::GhUnavailable)?;
        if !output.status.success() {
            return Err(CredentialError::GhRejected);
        }
        let value = String::from_utf8(output.stdout).map_err(|_| CredentialError::GhRejected)?;
        let token = value.trim();
        if token.is_empty() {
            return Err(CredentialError::GhRejected);
        }
        Ok(token.to_owned())
    }
}

#[derive(Debug, Clone)]
pub struct CredentialProvider<E = SystemEnvironment, G = SystemGh> {
    environment: E,
    gh: G,
}

impl Default for CredentialProvider<SystemEnvironment, SystemGh> {
    fn default() -> Self {
        Self {
            environment: SystemEnvironment,
            gh: SystemGh,
        }
    }
}

impl<E, G> CredentialProvider<E, G>
where
    E: Environment,
    G: GhCredentialSource,
{
    pub fn new(environment: E, gh: G) -> Self {
        Self { environment, gh }
    }

    pub fn resolve(&self) -> Result<Credential, CredentialError> {
        for key in ENV_TOKEN_KEYS {
            if let Some(value) = self.environment.read(key) {
                if !value.trim().is_empty() {
                    return Credential::new(value, CredentialSource::Environment(key));
                }
            }
        }

        let token = self.gh.token()?;
        Credential::new(token, CredentialSource::GhCli)
    }
}

pub fn redact_message(message: &str, credentials: &[&Credential]) -> String {
    credentials
        .iter()
        .fold(message.to_owned(), |redacted, credential| {
            redacted.replace(credential.authorization_token(), "***REDACTED***")
        })
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WriteGuard {
    frozen_generation: Option<u64>,
}

impl WriteGuard {
    pub fn can_write(&self, credential: &Credential) -> bool {
        self.frozen_generation != Some(credential.generation())
    }

    pub fn observe_http_status(&mut self, status: u16, credential: &Credential) {
        if status == 401 {
            self.frozen_generation = Some(credential.generation());
        }
    }

    pub fn require_write(&self, credential: &Credential) -> Result<(), WriteGuardError> {
        if self.can_write(credential) {
            Ok(())
        } else {
            Err(WriteGuardError::CredentialFrozen)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteGuardError {
    CredentialFrozen,
}

impl fmt::Display for WriteGuardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("GitHub writes are frozen for the active credential generation")
    }
}

impl std::error::Error for WriteGuardError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialDomain {
    Worker,
    MergeController,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitHubCapability {
    RepositoryRead,
    RepositoryWrite,
    ProjectsRead,
    ProjectsWrite,
    ActionsRead,
    ChecksRead,
    Merge,
}

pub fn capability_allowed(domain: CredentialDomain, capability: GitHubCapability) -> bool {
    match domain {
        CredentialDomain::Worker => capability != GitHubCapability::Merge,
        CredentialDomain::MergeController => true,
    }
}

#[cfg(test)]
mod read_only_tests {
    use super::*;
    #[test]
    fn endpoints_are_strictly_validated() {
        assert_eq!(
            issue_endpoint("burncloud", "burncloud", 734).unwrap(),
            "repos/burncloud/burncloud/issues/734"
        );
        assert!(issue_endpoint("burncloud", "evil/path", 1).is_err());
        assert!(pr_endpoint("burncloud", "repo", 0).is_err());
        assert!(list_issues_endpoint("burncloud", "repo", 0).is_err());
        assert!(list_prs_endpoint("burncloud", "repo", 1).is_ok());
        assert!(gh_get("-X DELETE").is_err());
    }
    #[test]
    fn github_response_shapes_are_typed() {
        let issue: RemoteIssue = serde_json::from_str(
            r#"{"id":123,"node_id":"I_1","number":7,"title":"Do work","state":"open"}"#,
        )
        .unwrap();
        assert_eq!(issue.number, 7);
        let pr:RemotePr=serde_json::from_str(r#"{"id":10,"node_id":"PR_1","number":2,"state":"open","draft":true,"head":{"sha":"abc"}}"#).unwrap();
        assert!(pr.draft);
    }
}


#[cfg(test)]
mod credential_tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct FakeEnvironment {
        values: HashMap<&'static str, String>,
    }

    impl Environment for FakeEnvironment {
        fn read(&self, key: &str) -> Option<String> {
            self.values.get(key).cloned()
        }
    }

    struct FakeGh(Result<String, CredentialError>);

    impl GhCredentialSource for FakeGh {
        fn token(&self) -> Result<String, CredentialError> {
            self.0.clone()
        }
    }

    #[test]
    fn environment_precedes_gh_and_debug_redacts() {
        let mut environment = FakeEnvironment::default();
        environment
            .values
            .insert("BAM_GITHUB_TOKEN", "super-secret".into());
        let provider =
            CredentialProvider::new(environment, FakeGh(Err(CredentialError::GhRejected)));
        let credential = provider.resolve().unwrap();
        assert_eq!(
            credential.source(),
            &CredentialSource::Environment("BAM_GITHUB_TOKEN")
        );
        let debug = format!("{credential:?}");
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("redacted"));
    }

    #[test]
    fn empty_environment_falls_back_to_gh() {
        let mut environment = FakeEnvironment::default();
        environment.values.insert("GH_TOKEN", "   ".into());
        let provider = CredentialProvider::new(environment, FakeGh(Ok("gh-secret\n".into())));
        let credential = provider.resolve().unwrap();
        assert_eq!(credential.source(), &CredentialSource::GhCli);
        assert_eq!(credential.authorization_token(), "gh-secret");
    }

    #[test]
    fn missing_credentials_fail_closed() {
        let provider = CredentialProvider::new(
            FakeEnvironment::default(),
            FakeGh(Err(CredentialError::Missing)),
        );
        assert_eq!(provider.resolve().unwrap_err(), CredentialError::Missing);
    }

    #[test]
    fn redaction_removes_secret_material() {
        let credential = Credential::new("token-value".into(), CredentialSource::GhCli).unwrap();
        let message = redact_message(
            "request Authorization: bearer token-value failed",
            &[&credential],
        );
        assert_eq!(
            message,
            "request Authorization: bearer ***REDACTED*** failed"
        );
    }

    #[test]
    fn unauthorized_response_freezes_only_that_generation() {
        let first = Credential::new("first-token".into(), CredentialSource::GhCli).unwrap();
        let second = Credential::new("second-token".into(), CredentialSource::GhCli).unwrap();
        let mut guard = WriteGuard::default();

        assert!(guard.require_write(&first).is_ok());
        guard.observe_http_status(403, &first);
        assert!(guard.require_write(&first).is_ok());

        guard.observe_http_status(401, &first);
        assert_eq!(
            guard.require_write(&first),
            Err(WriteGuardError::CredentialFrozen)
        );
        assert!(guard.require_write(&second).is_ok());
    }

    #[test]
    fn worker_domain_never_has_merge_capability() {
        assert!(!capability_allowed(
            CredentialDomain::Worker,
            GitHubCapability::Merge
        ));
        assert!(capability_allowed(
            CredentialDomain::Worker,
            GitHubCapability::RepositoryWrite
        ));
        assert!(capability_allowed(
            CredentialDomain::MergeController,
            GitHubCapability::Merge
        ));
    }
}


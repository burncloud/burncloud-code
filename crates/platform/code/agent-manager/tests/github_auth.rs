use bam::github::{
    capability_allowed, CredentialDomain, CredentialProvider, CredentialSource, Environment,
    GhCredentialSource, GitHubCapability, WriteGuard,
};
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

struct FakeGh {
    token: Option<String>,
}

impl GhCredentialSource for FakeGh {
    fn token(&self) -> Result<String, bam::github::CredentialError> {
        self.token
            .clone()
            .ok_or(bam::github::CredentialError::Missing)
    }
}

#[test]
fn provider_uses_explicit_environment_source_without_secret_in_debug() {
    let mut environment = FakeEnvironment::default();
    environment
        .values
        .insert("GH_TOKEN", "integration-secret".into());

    let credential = CredentialProvider::new(environment, FakeGh { token: None })
        .resolve()
        .unwrap();

    assert_eq!(
        credential.source(),
        &CredentialSource::Environment("GH_TOKEN")
    );
    assert!(!format!("{credential:?}").contains("integration-secret"));
}

#[test]
fn auth_failure_blocks_old_credential_but_not_rotated_one() {
    let mut first_environment = FakeEnvironment::default();
    first_environment.values.insert("GH_TOKEN", "first".into());
    let first = CredentialProvider::new(first_environment, FakeGh { token: None })
        .resolve()
        .unwrap();

    let mut second_environment = FakeEnvironment::default();
    second_environment
        .values
        .insert("GH_TOKEN", "second".into());
    let second = CredentialProvider::new(second_environment, FakeGh { token: None })
        .resolve()
        .unwrap();

    let mut guard = WriteGuard::default();
    guard.observe_http_status(401, &first);

    assert!(!guard.can_write(&first));
    assert!(guard.can_write(&second));
}

#[test]
fn worker_credentials_cannot_be_used_for_merge() {
    assert!(!capability_allowed(
        CredentialDomain::Worker,
        GitHubCapability::Merge
    ));
}

//! Merge authorization and final verification guard.
//!
//! Requirements: M12-01 (automatic merge is denied by default), M12-02 (read the user
//! authorization policy), M12-03 (verify the authorization scope before merging),
//! M12-04 (re-verify HEAD), M12-05 (check CI), M12-06 (check upstream dependencies),
//! M12-07 (check review), M12-08 (user can revoke), M12-09 (audit trail), M12-10 (never
//! modify protection rules to obtain merge rights); design M12-A, M12-C, M12-D, M12-E,
//! M12-F.
//!
//! Design M12-B (isolated credential domain) is not claimed here: it belongs with the
//! credential boundary module, which is not on `main` yet.
//!
//! The guard is the only place that may decide a merge. It performs no network call
//! itself: the caller re-reads GitHub and passes in the freshly observed head sha and
//! whether the pull request is already merged.
use crate::evidence::{is_full_sha, EvidenceReport, Verdict};
use crate::storage::{self, MergeAuthorization};
use rusqlite::Connection;

/// Runtime merge policy. `auto_merge_enabled` is false unless the deployment
/// explicitly authorizes autonomous merging; the shipped configuration forbids it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MergePolicy {
    pub auto_merge_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeRefusal {
    AutomaticMergeDisabled,
    ProtectionBypassForbidden,
    HeadChanged,
    HeadNotFullSha,
    EvidenceNotPassing,
    ReviewBlocking,
    DependenciesIncomplete,
    AuthorizationMissing,
    AuthorizationNotYetValid,
    AuthorizationExpired,
    AuthorizationRevoked,
}

impl MergeRefusal {
    pub fn code(&self) -> &'static str {
        match self {
            Self::AutomaticMergeDisabled => "AUTOMATIC_MERGE_DISABLED",
            Self::ProtectionBypassForbidden => "PROTECTION_BYPASS_FORBIDDEN",
            Self::HeadChanged => "HEAD_CHANGED",
            Self::HeadNotFullSha => "HEAD_NOT_FULL_SHA",
            Self::EvidenceNotPassing => "EVIDENCE_NOT_PASSING",
            Self::ReviewBlocking => "REVIEW_BLOCKING",
            Self::DependenciesIncomplete => "DEPENDENCIES_INCOMPLETE",
            Self::AuthorizationMissing => "AUTHORIZATION_MISSING",
            Self::AuthorizationNotYetValid => "AUTHORIZATION_NOT_YET_VALID",
            Self::AuthorizationExpired => "AUTHORIZATION_EXPIRED",
            Self::AuthorizationRevoked => "AUTHORIZATION_REVOKED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeDecision {
    /// Merge exactly this sha; any other head must be refused.
    Authorized {
        authorization_id: String,
        head_sha: String,
    },
    /// GitHub already merged this pull request; never issue a second merge request.
    AlreadyMerged {
        merged_sha: String,
    },
    Refused(MergeRefusal),
}

impl MergeDecision {
    pub fn is_authorized(&self) -> bool {
        matches!(self, Self::Authorized { .. })
    }

    pub fn refusal(&self) -> Option<&MergeRefusal> {
        match self {
            Self::Refused(refusal) => Some(refusal),
            _ => None,
        }
    }
}

/// Everything the guard needs, all of it freshly observed by the caller.
#[derive(Debug, Clone)]
pub struct MergeRequest<'a> {
    pub repo_id: i64,
    pub pr_number: u64,
    /// The sha the authorization was written for and the evidence was evaluated on.
    pub expected_head_sha: &'a str,
    /// The head sha re-read from GitHub immediately before merging (M12-04).
    pub current_head_sha: &'a str,
    /// Set when GitHub already reports the pull request as merged (M12-E).
    pub remote_merged_sha: Option<&'a str>,
    /// Non-blocking dependencies of the task are all DONE (M12-06).
    pub dependencies_done: bool,
    /// The caller asked to bypass branch protection; always refused (M12-10).
    pub bypass_protection: bool,
    pub now: i64,
}

/// Decide whether a merge may proceed, and audit the outcome (M12-D, M12-E, M12-F).
///
/// The decision is recorded as a durable operation intent before any remote write, so a
/// crash between the request and the confirmation is recoverable by re-reading GitHub.
pub fn authorize(
    connection: &mut Connection,
    policy: MergePolicy,
    request: &MergeRequest<'_>,
    evidence: &EvidenceReport,
) -> Result<MergeDecision, String> {
    let decision = decide(connection, policy, request, evidence)?;
    audit(connection, request, &decision)?;
    Ok(decision)
}

fn decide(
    connection: &Connection,
    policy: MergePolicy,
    request: &MergeRequest<'_>,
    evidence: &EvidenceReport,
) -> Result<MergeDecision, String> {
    // M12-10: protection is never bypassed to obtain merge rights.
    if request.bypass_protection {
        return Ok(MergeDecision::Refused(
            MergeRefusal::ProtectionBypassForbidden,
        ));
    }

    // M12-E: merging twice is refused; the remote is the authority.
    if let Some(merged_sha) = request.remote_merged_sha {
        return Ok(MergeDecision::AlreadyMerged {
            merged_sha: merged_sha.to_owned(),
        });
    }

    // M12-01: without an explicit policy the answer is no.
    if !policy.auto_merge_enabled {
        return Ok(MergeDecision::Refused(MergeRefusal::AutomaticMergeDisabled));
    }

    // M12-04 / M12-E: the head must still be the exact sha we verified.
    if !is_full_sha(request.current_head_sha) {
        return Ok(MergeDecision::Refused(MergeRefusal::HeadNotFullSha));
    }
    if request.expected_head_sha != request.current_head_sha {
        return Ok(MergeDecision::Refused(MergeRefusal::HeadChanged));
    }

    // M12-05: CI evidence must be a PASS bound to this very commit.
    if evidence.verdict != Verdict::Pass || evidence.pr.head_sha != request.current_head_sha {
        return Ok(MergeDecision::Refused(MergeRefusal::EvidenceNotPassing));
    }

    // M12-07: no unresolved blocking review.
    if evidence.review.verdict != Verdict::Pass || !evidence.review.blocking_reviews.is_empty() {
        return Ok(MergeDecision::Refused(MergeRefusal::ReviewBlocking));
    }

    // M12-06: upstream dependencies must be complete.
    if !request.dependencies_done {
        return Ok(MergeDecision::Refused(MergeRefusal::DependenciesIncomplete));
    }

    // M12-02 / M12-03: an authorization must exist and cover exactly this scope.
    let grants = storage::merge_authorizations_for(connection, request.repo_id, request.pr_number)?;
    if grants.is_empty() {
        return Ok(MergeDecision::Refused(MergeRefusal::AuthorizationMissing));
    }
    if let Some(active) = grants.iter().find(|grant| {
        grant.is_active_for(
            request.repo_id,
            request.pr_number,
            request.current_head_sha,
            request.now,
        )
    }) {
        return Ok(MergeDecision::Authorized {
            authorization_id: active.authorization_id.clone(),
            head_sha: request.current_head_sha.to_owned(),
        });
    }

    // Explain precisely why no grant applied, newest first.
    let refusal = grants
        .iter()
        .find(|grant| {
            grant.repo_id == request.repo_id
                && grant.pr_number == request.pr_number
                && grant.head_sha == request.current_head_sha
                && grant.revoked_at.is_some()
        })
        .map(|_| MergeRefusal::AuthorizationRevoked)
        .or_else(|| {
            grants
                .iter()
                .find(|grant| {
                    grant.head_sha == request.current_head_sha
                        && grant
                            .expires_at
                            .is_some_and(|deadline| request.now >= deadline)
                })
                .map(|_| MergeRefusal::AuthorizationExpired)
        })
        .unwrap_or(MergeRefusal::AuthorizationNotYetValid);
    Ok(MergeDecision::Refused(refusal))
}

/// Record the decision where a crash cannot lose it (M12-09, M12-F).
///
/// An authorized decision becomes a PENDING outbox intent that must be confirmed against
/// GitHub after the merge. A refused decision is recorded as BLOCKED, so it can never be
/// replayed as a pending merge.
fn audit(
    connection: &mut Connection,
    request: &MergeRequest<'_>,
    decision: &MergeDecision,
) -> Result<(), String> {
    let (operation_key, blocked) = match decision {
        MergeDecision::Authorized {
            authorization_id, ..
        } => (
            format!("merge:{}:{authorization_id}", request.pr_number),
            false,
        ),
        MergeDecision::AlreadyMerged { .. } => return Ok(()),
        MergeDecision::Refused(refusal) => (
            format!("merge:{}:refused:{}", request.pr_number, refusal.code()),
            true,
        ),
    };
    storage::enqueue_operation(
        connection,
        &storage::OperationIntent {
            operation_key: operation_key.clone(),
            task_id: None,
            operation_type: "MERGE".to_owned(),
            target: format!("repos/{}/pulls/{}", request.repo_id, request.pr_number),
            expected_head_sha: Some(request.current_head_sha.to_owned()),
        },
    )?;
    if blocked {
        connection
            .execute(
                "UPDATE operation_outbox SET state='BLOCKED' WHERE operation_key=?1",
                rusqlite::params![operation_key],
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Read the effective policy from configuration. Autonomous merging stays disabled
/// unless the configuration explicitly enables it.
pub fn policy_from_config(config: &crate::config::Config) -> MergePolicy {
    MergePolicy {
        auto_merge_enabled: config.merge.auto_merge,
    }
}

/// Confirm a performed merge with the sha GitHub actually reports (M12-D).
pub fn confirm(
    connection: &Connection,
    operation_key: &str,
    merged_sha: &str,
) -> Result<bool, String> {
    if !is_full_sha(merged_sha) {
        return Err("merge confirmation requires the full merged sha".into());
    }
    storage::confirm_operation(connection, operation_key, Some(merged_sha))
}

/// The grants currently usable for a pull request.
pub fn active_authorizations(
    connection: &Connection,
    repo_id: i64,
    pr_number: u64,
    head_sha: &str,
    now: i64,
) -> Result<Vec<MergeAuthorization>, String> {
    Ok(
        storage::merge_authorizations_for(connection, repo_id, pr_number)?
            .into_iter()
            .filter(|grant| grant.is_active_for(repo_id, pr_number, head_sha, now))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DEFAULT_CONFIG};
    use crate::evidence::{
        CheckConclusion, CheckEvidence, CheckStatus, Criterion, IssueRef, Mergeability,
        PullRequestRef, RequiredPolicy, ReviewState, SCHEMA_VERSION,
    };

    const HEAD: &str = "0bb0783aa176922343ec1897313b4a4cb75c09dd";

    fn authorization(id: &str, expires_at: Option<i64>) -> MergeAuthorization {
        MergeAuthorization {
            authorization_id: id.to_owned(),
            repo_id: 1,
            pr_number: 33,
            head_sha: HEAD.to_owned(),
            actor: "rustburn".to_owned(),
            expires_at,
            revoked_at: None,
        }
    }

    fn passing_report() -> EvidenceReport {
        EvidenceReport {
            schema_version: SCHEMA_VERSION.to_owned(),
            task_id: "task-32".to_owned(),
            issue: IssueRef {
                repository_id: 1,
                number: 32,
                node_id: "I_kwDOA".to_owned(),
            },
            pr: PullRequestRef {
                repository_id: 1,
                number: 33,
                node_id: "PR_kwDOA".to_owned(),
                head_sha: HEAD.to_owned(),
            },
            base_sha: "0f81ecb3c1d2e3f405162738495a6b7c8d9e0f11".to_owned(),
            merge_ref_sha: None,
            required_policy: RequiredPolicy {
                source: "github_rulesets".to_owned(),
                verified: true,
                version: Some("ruleset:24846723".to_owned()),
            },
            checks: vec![CheckEvidence {
                context: "BAM / Actual Fmt Clippy Test".to_owned(),
                app: Some("github-actions".to_owned()),
                run_id: Some(38078403556),
                job_id: Some(114289999999),
                check_run_id: None,
                sha: HEAD.to_owned(),
                status: CheckStatus::Completed,
                conclusion: CheckConclusion::Success,
                actually_executed: true,
                url: Some("https://example.invalid/run".to_owned()),
            }],
            criteria: vec![Criterion {
                id: "M12-05".to_owned(),
                test_ids: vec!["merge_guard::tests::authorized_merge".to_owned()],
                verdict: Verdict::Pass,
                evidence_ids: vec!["run:38078403556".to_owned()],
            }],
            review: ReviewState {
                verdict: Verdict::Pass,
                blocking_reviews: Vec::new(),
            },
            mergeability: Mergeability::Clean,
            local_compiles: 0,
            local_tests: 0,
            verdict: Verdict::Pass,
            reason_codes: Vec::new(),
            evaluated_at: "2026-10-11T03:20:00Z".to_owned(),
        }
    }

    fn request<'a>() -> MergeRequest<'a> {
        MergeRequest {
            repo_id: 1,
            pr_number: 33,
            expected_head_sha: HEAD,
            current_head_sha: HEAD,
            remote_merged_sha: None,
            dependencies_done: true,
            bypass_protection: false,
            now: 1_000,
        }
    }

    fn database() -> (tempfile::TempDir, Connection) {
        let temp = tempfile::tempdir().unwrap();
        let connection = storage::open(&temp.path().join("state.db")).unwrap();
        (temp, connection)
    }

    #[test]
    fn an_authorized_merge_proceeds_and_is_audited() {
        let (_temp, mut connection) = database();
        storage::register_merge_authorization(&connection, &authorization("auth-1", Some(2_000)))
            .unwrap();
        let report = passing_report();
        let decision = authorize(
            &mut connection,
            MergePolicy {
                auto_merge_enabled: true,
            },
            &request(),
            &report,
        )
        .unwrap();
        assert_eq!(
            decision,
            MergeDecision::Authorized {
                authorization_id: "auth-1".to_owned(),
                head_sha: HEAD.to_owned(),
            }
        );
        assert!(decision.is_authorized());

        // The intent is durable and pending confirmation.
        let pending = storage::pending_operations(&connection).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].operation_type, "MERGE");
        assert_eq!(pending[0].expected_head_sha.as_deref(), Some(HEAD));

        // Confirmation requires the real merged sha.
        assert!(confirm(&connection, &pending[0].operation_key, "short").is_err());
        assert!(confirm(
            &connection,
            &pending[0].operation_key,
            "0f81ecb3c1d2e3f405162738495a6b7c8d9e0f11"
        )
        .unwrap());
        assert!(storage::pending_operations(&connection).unwrap().is_empty());
    }

    #[test]
    fn automatic_merge_is_denied_by_default() {
        let (_temp, mut connection) = database();
        storage::register_merge_authorization(&connection, &authorization("auth-1", None)).unwrap();
        let report = passing_report();
        let decision =
            authorize(&mut connection, MergePolicy::default(), &request(), &report).unwrap();
        assert_eq!(
            decision,
            MergeDecision::Refused(MergeRefusal::AutomaticMergeDisabled)
        );
        // A refusal is audited as blocked, not left silent.
        assert_eq!(storage::pending_operations(&connection).unwrap().len(), 0);

        let config = Config::parse(DEFAULT_CONFIG).unwrap();
        assert!(!policy_from_config(&config).auto_merge_enabled);
    }

    #[test]
    fn a_changed_head_invalidates_the_authorization_and_the_evidence() {
        let (_temp, mut connection) = database();
        storage::register_merge_authorization(&connection, &authorization("auth-1", None)).unwrap();
        let report = passing_report();
        let policy = MergePolicy {
            auto_merge_enabled: true,
        };

        let mut moved = request();
        moved.current_head_sha = "c5820a2c719a4c4c257cf94362de46c428a59bd4";
        assert_eq!(
            authorize(&mut connection, policy, &moved, &report)
                .unwrap()
                .refusal(),
            Some(&MergeRefusal::HeadChanged)
        );

        let mut abbreviated = request();
        abbreviated.current_head_sha = "0bb0783";
        assert_eq!(
            authorize(&mut connection, policy, &abbreviated, &report)
                .unwrap()
                .refusal(),
            Some(&MergeRefusal::HeadNotFullSha)
        );
    }

    #[test]
    fn evidence_review_and_dependencies_are_hard_gates() {
        let (_temp, mut connection) = database();
        storage::register_merge_authorization(&connection, &authorization("auth-1", None)).unwrap();
        let policy = MergePolicy {
            auto_merge_enabled: true,
        };

        let mut failing = passing_report();
        failing.verdict = Verdict::Fail;
        assert_eq!(
            authorize(&mut connection, policy, &request(), &failing)
                .unwrap()
                .refusal(),
            Some(&MergeRefusal::EvidenceNotPassing)
        );

        let mut stale = passing_report();
        stale.pr.head_sha = "c5820a2c719a4c4c257cf94362de46c428a59bd4".to_owned();
        assert_eq!(
            authorize(&mut connection, policy, &request(), &stale)
                .unwrap()
                .refusal(),
            Some(&MergeRefusal::EvidenceNotPassing)
        );

        let mut blocked_review = passing_report();
        blocked_review.review.blocking_reviews = vec!["changes requested".to_owned()];
        assert_eq!(
            authorize(&mut connection, policy, &request(), &blocked_review)
                .unwrap()
                .refusal(),
            Some(&MergeRefusal::ReviewBlocking)
        );

        let mut unfinished = request();
        unfinished.dependencies_done = false;
        let report = passing_report();
        assert_eq!(
            authorize(&mut connection, policy, &unfinished, &report)
                .unwrap()
                .refusal(),
            Some(&MergeRefusal::DependenciesIncomplete)
        );
    }

    #[test]
    fn authorizations_are_scoped_expiring_and_revocable() {
        let (_temp, mut connection) = database();
        let policy = MergePolicy {
            auto_merge_enabled: true,
        };
        let report = passing_report();

        // No grant at all.
        assert_eq!(
            authorize(&mut connection, policy, &request(), &report)
                .unwrap()
                .refusal(),
            Some(&MergeRefusal::AuthorizationMissing)
        );

        // Grant for another commit.
        let mut other_head = authorization("auth-other", None);
        other_head.head_sha = "c5820a2c719a4c4c257cf94362de46c428a59bd4".to_owned();
        storage::register_merge_authorization(&connection, &other_head).unwrap();
        assert_eq!(
            authorize(&mut connection, policy, &request(), &report)
                .unwrap()
                .refusal(),
            Some(&MergeRefusal::AuthorizationNotYetValid)
        );

        // Expired grant (now is past the deadline).
        storage::register_merge_authorization(&connection, &authorization("auth-old", Some(900)))
            .unwrap();
        assert_eq!(
            authorize(&mut connection, policy, &request(), &report)
                .unwrap()
                .refusal(),
            Some(&MergeRefusal::AuthorizationExpired)
        );

        // Live grant.
        storage::register_merge_authorization(
            &connection,
            &authorization("auth-live", Some(2_000)),
        )
        .unwrap();
        assert!(authorize(&mut connection, policy, &request(), &report)
            .unwrap()
            .is_authorized());

        // Revocation takes effect immediately.
        assert!(storage::revoke_merge_authorization(&connection, "auth-live", 1_100).unwrap());
        assert_eq!(
            authorize(&mut connection, policy, &request(), &report)
                .unwrap()
                .refusal(),
            Some(&MergeRefusal::AuthorizationRevoked)
        );
        assert!(active_authorizations(&connection, 1, 33, HEAD, 1_100)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn protection_bypass_and_double_merge_are_refused() {
        let (_temp, mut connection) = database();
        storage::register_merge_authorization(&connection, &authorization("auth-1", None)).unwrap();
        let policy = MergePolicy {
            auto_merge_enabled: true,
        };
        let report = passing_report();

        let mut bypass = request();
        bypass.bypass_protection = true;
        assert_eq!(
            authorize(&mut connection, policy, &bypass, &report)
                .unwrap()
                .refusal(),
            Some(&MergeRefusal::ProtectionBypassForbidden)
        );

        let mut merged = request();
        merged.remote_merged_sha = Some("0f81ecb3c1d2e3f405162738495a6b7c8d9e0f11");
        let decision = authorize(&mut connection, policy, &merged, &report).unwrap();
        assert_eq!(
            decision,
            MergeDecision::AlreadyMerged {
                merged_sha: "0f81ecb3c1d2e3f405162738495a6b7c8d9e0f11".to_owned()
            }
        );
        assert!(!decision.is_authorized());
    }

    #[test]
    fn refusals_are_recorded_as_blocked_outbox_intents() {
        let (_temp, mut connection) = database();
        let policy = MergePolicy {
            auto_merge_enabled: true,
        };
        let report = passing_report();
        let decision = authorize(&mut connection, policy, &request(), &report).unwrap();
        assert_eq!(
            decision.refusal(),
            Some(&MergeRefusal::AuthorizationMissing)
        );
        let blocked: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM operation_outbox WHERE state='BLOCKED' AND operation_type='MERGE'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(blocked, 1);
        assert!(storage::pending_operations(&connection).unwrap().is_empty());
    }

    #[test]
    fn refusal_codes_are_stable() {
        assert_eq!(
            MergeRefusal::AutomaticMergeDisabled.code(),
            "AUTOMATIC_MERGE_DISABLED"
        );
        assert_eq!(
            MergeRefusal::ProtectionBypassForbidden.code(),
            "PROTECTION_BYPASS_FORBIDDEN"
        );
        assert_eq!(MergeRefusal::HeadChanged.code(), "HEAD_CHANGED");
        assert_eq!(
            MergeRefusal::AuthorizationRevoked.code(),
            "AUTHORIZATION_REVOKED"
        );
    }
}

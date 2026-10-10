//! CI evidence contract and verdict evaluation.
//!
//! Requirements: M08-B (SHA binding), M08-C (status normalisation), M08-D (actual
//! execution evidence), M08-12 (SKIPPED), M08-13 (CANCELLED), M08-14 (TIMED_OUT),
//! M08-15 (stale HEAD), M08-16 (missing check), M08-17 (path filtering / applicability).
//!
//! This module implements the JSON contract and the PASS rules of
//! `docs/ci/evidence-schema.md` plus the non-pass states listed in `AGENTS.md` R4:
//! a green string alone is never a pass, and no verdict of PASS is possible without
//! proof that the check really executed against the evaluated head sha.
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "1.0";
pub const FULL_SHA_LENGTH: usize = 40;

/// A 40 character hexadecimal commit id.
pub fn is_full_sha(value: &str) -> bool {
    value.len() == FULL_SHA_LENGTH && value.chars().all(|c| c.is_ascii_hexdigit())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    Pass,
    Fail,
    Blocked,
    Unknown,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Blocked => "BLOCKED",
            Self::Unknown => "UNKNOWN",
        }
    }

    /// Ranking used when several findings disagree: a credible failure outranks a
    /// blocked gate, which outranks missing information.
    fn severity(self) -> u8 {
        match self {
            Self::Pass => 0,
            Self::Unknown => 1,
            Self::Blocked => 2,
            Self::Fail => 3,
        }
    }

    /// Only PASS may advance a pull request to Ready for Review.
    pub fn allows_ready(self) -> bool {
        self == Self::Pass
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Queued,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckConclusion {
    Success,
    Failure,
    Cancelled,
    TimedOut,
    ActionRequired,
    Neutral,
    Skipped,
    Stale,
    Missing,
    Unknown,
}

impl CheckConclusion {
    /// Normalise a raw GitHub conclusion. Unknown strings stay `Unknown`; they are
    /// never silently promoted to a success.
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "success" => Self::Success,
            "failure" | "failed" | "error" => Self::Failure,
            "cancelled" | "canceled" => Self::Cancelled,
            "timed_out" | "timedout" | "timeout" => Self::TimedOut,
            "action_required" => Self::ActionRequired,
            "neutral" => Self::Neutral,
            "skipped" => Self::Skipped,
            "stale" => Self::Stale,
            "missing" => Self::Missing,
            _ => Self::Unknown,
        }
    }

    /// The auditable rejection list: none of these may be treated as a pass.
    pub fn is_unusable_as_pass(self) -> bool {
        matches!(
            self,
            Self::Failure
                | Self::Cancelled
                | Self::TimedOut
                | Self::ActionRequired
                | Self::Neutral
                | Self::Skipped
                | Self::Stale
                | Self::Missing
                | Self::Unknown
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Mergeability {
    Clean,
    Dirty,
    Blocked,
    Behind,
    Draft,
    Unstable,
    Unknown,
}

impl Mergeability {
    /// Normalise GitHub's `mergeable_state`.
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "clean" => Self::Clean,
            "dirty" => Self::Dirty,
            "blocked" => Self::Blocked,
            "behind" => Self::Behind,
            "draft" => Self::Draft,
            "unstable" => Self::Unstable,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasonCode {
    StaleHead,
    RequiredPolicyUnavailable,
    RequiredPolicyUnverified,
    RequiredCheckMissing,
    RequiredCheckPending,
    RequiredCheckFailed,
    RequiredCheckSkipped,
    RequiredCheckCancelled,
    RequiredCheckTimedOut,
    RequiredCheckActionRequired,
    RequiredCheckNeutral,
    RequiredCheckUnknown,
    RequiredCheckNotExecuted,
    EvidenceWithoutRunIdentity,
    CriterionFailed,
    MissingRequiredEvidence,
    ReviewBlocking,
    ReviewUnknown,
    MergeConflict,
    MergeBlocked,
    MergeDraft,
    MergeBehind,
    MergeUnstable,
    MergeabilityUnknown,
    LocalExecutionViolation,
}

impl ReasonCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StaleHead => "STALE_HEAD",
            Self::RequiredPolicyUnavailable => "REQUIRED_POLICY_UNAVAILABLE",
            Self::RequiredPolicyUnverified => "REQUIRED_POLICY_UNVERIFIED",
            Self::RequiredCheckMissing => "REQUIRED_CHECK_MISSING",
            Self::RequiredCheckPending => "REQUIRED_CHECK_PENDING",
            Self::RequiredCheckFailed => "REQUIRED_CHECK_FAILED",
            Self::RequiredCheckSkipped => "REQUIRED_CHECK_SKIPPED",
            Self::RequiredCheckCancelled => "REQUIRED_CHECK_CANCELLED",
            Self::RequiredCheckTimedOut => "REQUIRED_CHECK_TIMED_OUT",
            Self::RequiredCheckActionRequired => "REQUIRED_CHECK_ACTION_REQUIRED",
            Self::RequiredCheckNeutral => "REQUIRED_CHECK_NEUTRAL",
            Self::RequiredCheckUnknown => "REQUIRED_CHECK_UNKNOWN",
            Self::RequiredCheckNotExecuted => "REQUIRED_CHECK_NOT_EXECUTED",
            Self::EvidenceWithoutRunIdentity => "EVIDENCE_WITHOUT_RUN_IDENTITY",
            Self::CriterionFailed => "CRITERION_FAILED",
            Self::MissingRequiredEvidence => "MISSING_REQUIRED_EVIDENCE",
            Self::ReviewBlocking => "REVIEW_BLOCKING",
            Self::ReviewUnknown => "REVIEW_UNKNOWN",
            Self::MergeConflict => "MERGE_CONFLICT",
            Self::MergeBlocked => "MERGE_BLOCKED",
            Self::MergeDraft => "MERGE_DRAFT",
            Self::MergeBehind => "MERGE_BEHIND",
            Self::MergeUnstable => "MERGE_UNSTABLE",
            Self::MergeabilityUnknown => "MERGEABILITY_UNKNOWN",
            Self::LocalExecutionViolation => "LOCAL_EXECUTION_VIOLATION",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueRef {
    pub repository_id: u64,
    pub number: u64,
    pub node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestRef {
    pub repository_id: u64,
    pub number: u64,
    pub node_id: String,
    pub head_sha: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequiredPolicy {
    pub source: String,
    pub verified: bool,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckEvidence {
    pub context: String,
    #[serde(default)]
    pub app: Option<String>,
    #[serde(default)]
    pub run_id: Option<u64>,
    #[serde(default)]
    pub job_id: Option<u64>,
    #[serde(default)]
    pub check_run_id: Option<u64>,
    pub sha: String,
    pub status: CheckStatus,
    pub conclusion: CheckConclusion,
    #[serde(default)]
    pub actually_executed: bool,
    #[serde(default)]
    pub url: Option<String>,
}

impl CheckEvidence {
    /// True only for a completed, actually executed success bound to `expected_sha`
    /// and carrying a run identity that can be re-audited.
    pub fn is_pass_for(&self, expected_sha: &str) -> bool {
        self.classify(expected_sha).is_none()
    }

    /// Why this check cannot be accepted, or `None` when it is a valid pass.
    pub fn classify(&self, expected_sha: &str) -> Option<(Verdict, ReasonCode)> {
        if !is_full_sha(&self.sha) || self.sha != expected_sha {
            return Some((Verdict::Unknown, ReasonCode::StaleHead));
        }
        match self.status {
            CheckStatus::Queued | CheckStatus::InProgress => {
                return Some((Verdict::Unknown, ReasonCode::RequiredCheckPending))
            }
            CheckStatus::Completed => {}
        }
        match self.conclusion {
            CheckConclusion::Success => {
                if !self.actually_executed {
                    return Some((Verdict::Fail, ReasonCode::RequiredCheckNotExecuted));
                }
                if self.run_id.is_none() && self.job_id.is_none() && self.check_run_id.is_none() {
                    return Some((Verdict::Fail, ReasonCode::EvidenceWithoutRunIdentity));
                }
                None
            }
            CheckConclusion::Failure => Some((Verdict::Fail, ReasonCode::RequiredCheckFailed)),
            CheckConclusion::Skipped => Some((Verdict::Fail, ReasonCode::RequiredCheckSkipped)),
            CheckConclusion::Cancelled => Some((Verdict::Fail, ReasonCode::RequiredCheckCancelled)),
            CheckConclusion::TimedOut => Some((Verdict::Fail, ReasonCode::RequiredCheckTimedOut)),
            CheckConclusion::ActionRequired => {
                Some((Verdict::Fail, ReasonCode::RequiredCheckActionRequired))
            }
            CheckConclusion::Neutral => Some((Verdict::Fail, ReasonCode::RequiredCheckNeutral)),
            CheckConclusion::Missing => Some((Verdict::Fail, ReasonCode::RequiredCheckMissing)),
            CheckConclusion::Stale => Some((Verdict::Unknown, ReasonCode::StaleHead)),
            CheckConclusion::Unknown => Some((Verdict::Unknown, ReasonCode::RequiredCheckUnknown)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Criterion {
    pub id: String,
    pub test_ids: Vec<String>,
    pub verdict: Verdict,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewState {
    pub verdict: Verdict,
    pub blocking_reviews: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceReport {
    pub schema_version: String,
    pub task_id: String,
    pub issue: IssueRef,
    pub pr: PullRequestRef,
    pub base_sha: String,
    #[serde(default)]
    pub merge_ref_sha: Option<String>,
    pub required_policy: RequiredPolicy,
    pub checks: Vec<CheckEvidence>,
    pub criteria: Vec<Criterion>,
    pub review: ReviewState,
    pub mergeability: Mergeability,
    pub local_compiles: u64,
    pub local_tests: u64,
    pub verdict: Verdict,
    pub reason_codes: Vec<String>,
    pub evaluated_at: String,
}

impl EvidenceReport {
    /// Evaluate the report against the expectations that apply to this pull request.
    /// Writes `verdict` and `reason_codes`, and returns the verdict.
    pub fn evaluate(&mut self, context: &EvaluationContext<'_>) -> Verdict {
        let mut findings: Vec<(Verdict, ReasonCode)> = Vec::new();

        // The product forbids local compilation and local test execution.
        if self.local_compiles > 0 || self.local_tests > 0 {
            findings.push((Verdict::Fail, ReasonCode::LocalExecutionViolation));
        }

        // Without an authoritative required policy we cannot claim a gate passed.
        if context.policy_unavailable {
            findings.push((Verdict::Blocked, ReasonCode::RequiredPolicyUnavailable));
        } else if !self.required_policy.verified {
            findings.push((Verdict::Unknown, ReasonCode::RequiredPolicyUnverified));
        }

        if !is_full_sha(&self.pr.head_sha) || self.pr.head_sha != context.expected_head_sha {
            findings.push((Verdict::Unknown, ReasonCode::StaleHead));
        }

        for required in context.required_contexts {
            match self.checks.iter().find(|check| check.context == *required) {
                None => findings.push((Verdict::Fail, ReasonCode::RequiredCheckMissing)),
                Some(check) => {
                    if let Some(finding) = check.classify(context.expected_head_sha) {
                        findings.push(finding);
                    }
                }
            }
        }

        for criterion in &self.criteria {
            match criterion.verdict {
                Verdict::Pass => {}
                Verdict::Fail => findings.push((Verdict::Fail, ReasonCode::CriterionFailed)),
                Verdict::Blocked => findings.push((Verdict::Blocked, ReasonCode::CriterionFailed)),
                Verdict::Unknown => {
                    findings.push((Verdict::Unknown, ReasonCode::MissingRequiredEvidence))
                }
            }
        }

        if !self.review.blocking_reviews.is_empty() || self.review.verdict == Verdict::Fail {
            findings.push((Verdict::Fail, ReasonCode::ReviewBlocking));
        } else if self.review.verdict != Verdict::Pass {
            findings.push((Verdict::Unknown, ReasonCode::ReviewUnknown));
        }

        match self.mergeability {
            Mergeability::Clean => {}
            Mergeability::Dirty => findings.push((Verdict::Fail, ReasonCode::MergeConflict)),
            Mergeability::Blocked => findings.push((Verdict::Fail, ReasonCode::MergeBlocked)),
            Mergeability::Draft => findings.push((Verdict::Fail, ReasonCode::MergeDraft)),
            Mergeability::Behind => findings.push((Verdict::Unknown, ReasonCode::MergeBehind)),
            Mergeability::Unstable => findings.push((Verdict::Unknown, ReasonCode::MergeUnstable)),
            Mergeability::Unknown => {
                findings.push((Verdict::Unknown, ReasonCode::MergeabilityUnknown))
            }
        }

        let mut codes: Vec<String> = findings
            .iter()
            .map(|(_, code)| code.as_str().to_owned())
            .collect();
        codes.sort();
        codes.dedup();
        self.reason_codes = codes;

        let verdict = if findings.is_empty() {
            Verdict::Pass
        } else {
            let mut worst = Verdict::Unknown;
            for (finding, _) in &findings {
                if finding.severity() > worst.severity() {
                    worst = *finding;
                }
            }
            worst
        };
        self.verdict = verdict;
        verdict
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }

    pub fn from_json(raw: &str) -> Result<Self, String> {
        serde_json::from_str(raw).map_err(|error| error.to_string())
    }
}

/// The expectations a report is evaluated against. They come from the authoritative
/// required policy for the pull request, never from the agent's own judgement.
#[derive(Debug, Clone)]
pub struct EvaluationContext<'a> {
    /// The head sha the PR currently has; evidence must match it exactly.
    pub expected_head_sha: &'a str,
    /// Check contexts that must pass for this pull request.
    pub required_contexts: &'a [String],
    /// True when the required policy could not be read at all.
    pub policy_unavailable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "2101ed8050c7d442dfc86df8817574dec0d4b811";

    fn passing_report() -> EvidenceReport {
        EvidenceReport {
            schema_version: SCHEMA_VERSION.to_owned(),
            task_id: "task-1".to_owned(),
            issue: IssueRef {
                repository_id: 1,
                number: 24,
                node_id: "I_kwDOA".to_owned(),
            },
            pr: PullRequestRef {
                repository_id: 1,
                number: 25,
                node_id: "PR_kwDOA".to_owned(),
                head_sha: HEAD.to_owned(),
            },
            base_sha: "671e0e867f0f6bf031c48e55822d8cca8eb98200".to_owned(),
            merge_ref_sha: None,
            required_policy: RequiredPolicy {
                source: "github_rulesets".to_owned(),
                verified: true,
                version: Some("ruleset:24846723".to_owned()),
            },
            checks: vec![CheckEvidence {
                context: "BAM / Actual Fmt Clippy Test".to_owned(),
                app: Some("github-actions".to_owned()),
                run_id: Some(38077367154),
                job_id: Some(114284999999),
                check_run_id: None,
                sha: HEAD.to_owned(),
                status: CheckStatus::Completed,
                conclusion: CheckConclusion::Success,
                actually_executed: true,
                url: Some(
                    "https://github.com/burncloud/burncloud-code/actions/runs/38077367154"
                        .to_owned(),
                ),
            }],
            criteria: vec![Criterion {
                id: "M08-B".to_owned(),
                test_ids: vec!["evidence::tests::head_binding".to_owned()],
                verdict: Verdict::Pass,
                evidence_ids: vec!["run:38077367154".to_owned()],
            }],
            review: ReviewState {
                verdict: Verdict::Pass,
                blocking_reviews: Vec::new(),
            },
            mergeability: Mergeability::Clean,
            local_compiles: 0,
            local_tests: 0,
            verdict: Verdict::Unknown,
            reason_codes: Vec::new(),
            evaluated_at: "2026-10-11T02:50:00Z".to_owned(),
        }
    }

    fn context(required: &[String]) -> EvaluationContext<'_> {
        EvaluationContext {
            expected_head_sha: HEAD,
            required_contexts: required,
            policy_unavailable: false,
        }
    }

    #[test]
    fn documented_contract_round_trips() {
        let documented = r#"{
          "schema_version":"1.0",
          "task_id":"uuid",
          "issue":{"repository_id":0,"number":0,"node_id":"I_..."},
          "pr":{"repository_id":0,"number":0,"node_id":"PR_...","head_sha":"40-char-full-sha"},
          "base_sha":"40-char-full-sha",
          "merge_ref_sha":null,
          "required_policy":{"source":"github_rulesets","verified":false,"version":null},
          "checks":[{"context":"Cargo Test","app":null,"run_id":null,"job_id":null,"check_run_id":null,"sha":"40-char-full-sha","status":"completed","conclusion":"unknown","actually_executed":false,"url":null}],
          "criteria":[{"id":"M08-01","test_ids":["T-008"],"verdict":"UNKNOWN","evidence_ids":[]}],
          "review":{"verdict":"UNKNOWN","blocking_reviews":[]},
          "mergeability":"UNKNOWN",
          "local_compiles":0,
          "local_tests":0,
          "verdict":"UNKNOWN",
          "reason_codes":["MISSING_REQUIRED_EVIDENCE"],
          "evaluated_at":"RFC3339"
        }"#;
        let report = EvidenceReport::from_json(documented).expect("documented contract must parse");
        assert_eq!(report.schema_version, SCHEMA_VERSION);
        assert_eq!(report.verdict, Verdict::Unknown);
        assert_eq!(report.mergeability, Mergeability::Unknown);
        assert_eq!(report.checks[0].status, CheckStatus::Completed);
        assert_eq!(report.checks[0].conclusion, CheckConclusion::Unknown);
        assert!(report.checks[0].conclusion.is_unusable_as_pass());
        assert_eq!(
            report.reason_codes,
            vec!["MISSING_REQUIRED_EVIDENCE".to_owned()]
        );

        // Serialisation keeps the contract's uppercase verdict vocabulary.
        let json = report.to_json().unwrap();
        assert!(json.contains("\"verdict\": \"UNKNOWN\""));
        assert!(json.contains("\"mergeability\": \"UNKNOWN\""));
        assert!(json.contains("\"conclusion\": \"unknown\""));
        let reparsed = EvidenceReport::from_json(&json).unwrap();
        assert_eq!(reparsed, report);
    }

    #[test]
    fn a_complete_report_passes() {
        let mut report = passing_report();
        let required = vec!["BAM / Actual Fmt Clippy Test".to_owned()];
        assert_eq!(report.evaluate(&context(&required)), Verdict::Pass);
        assert!(report.reason_codes.is_empty());
        assert!(report.verdict.allows_ready());
    }

    #[test]
    fn stale_head_invalidates_green_evidence() {
        let mut report = passing_report();
        report.pr.head_sha = "c81e81374ee4218adafcc3ec8299be319bea23ad".to_owned();
        let required = vec!["BAM / Actual Fmt Clippy Test".to_owned()];
        assert_eq!(report.evaluate(&context(&required)), Verdict::Unknown);
        assert!(report.reason_codes.contains(&"STALE_HEAD".to_owned()));
        assert!(!report.verdict.allows_ready());
    }

    #[test]
    fn skipped_required_check_never_passes() {
        let mut report = passing_report();
        report.checks[0].conclusion = CheckConclusion::Skipped;
        report.checks[0].actually_executed = false;
        let required = vec!["BAM / Actual Fmt Clippy Test".to_owned()];
        assert_eq!(report.evaluate(&context(&required)), Verdict::Fail);
        assert!(report
            .reason_codes
            .contains(&"REQUIRED_CHECK_SKIPPED".to_owned()));
    }

    #[test]
    fn success_without_execution_or_run_identity_is_not_a_pass() {
        let required = vec!["BAM / Actual Fmt Clippy Test".to_owned()];

        let mut not_executed = passing_report();
        not_executed.checks[0].actually_executed = false;
        assert_eq!(not_executed.evaluate(&context(&required)), Verdict::Fail);
        assert!(not_executed
            .reason_codes
            .contains(&"REQUIRED_CHECK_NOT_EXECUTED".to_owned()));

        let mut no_identity = passing_report();
        no_identity.checks[0].run_id = None;
        no_identity.checks[0].job_id = None;
        no_identity.checks[0].check_run_id = None;
        assert_eq!(no_identity.evaluate(&context(&required)), Verdict::Fail);
        assert!(no_identity
            .reason_codes
            .contains(&"EVIDENCE_WITHOUT_RUN_IDENTITY".to_owned()));
    }

    #[test]
    fn rejected_states_are_all_refused() {
        for (raw, expected) in [
            ("failure", CheckConclusion::Failure),
            ("cancelled", CheckConclusion::Cancelled),
            ("timed_out", CheckConclusion::TimedOut),
            ("action_required", CheckConclusion::ActionRequired),
            ("neutral", CheckConclusion::Neutral),
            ("skipped", CheckConclusion::Skipped),
            ("stale", CheckConclusion::Stale),
            ("something-new", CheckConclusion::Unknown),
        ] {
            let conclusion = CheckConclusion::parse(raw);
            assert_eq!(conclusion, expected, "{raw} must normalise to {expected:?}");
            assert!(
                conclusion.is_unusable_as_pass(),
                "{raw} must never be treated as a pass"
            );
        }
        assert_eq!(CheckConclusion::parse("success"), CheckConclusion::Success);
        assert!(!CheckConclusion::parse("success").is_unusable_as_pass());
        assert_eq!(
            CheckConclusion::parse(" CANCELLED "),
            CheckConclusion::Cancelled
        );
    }

    #[test]
    fn pending_run_is_information_shortage_not_a_failure() {
        let mut report = passing_report();
        report.checks[0].status = CheckStatus::InProgress;
        report.checks[0].conclusion = CheckConclusion::Unknown;
        let required = vec!["BAM / Actual Fmt Clippy Test".to_owned()];
        assert_eq!(report.evaluate(&context(&required)), Verdict::Unknown);
        assert!(report
            .reason_codes
            .contains(&"REQUIRED_CHECK_PENDING".to_owned()));
    }

    #[test]
    fn a_missing_required_context_blocks_the_merge_gate() {
        let mut report = passing_report();
        let required = vec!["check / Cargo Test".to_owned()];
        assert_eq!(report.evaluate(&context(&required)), Verdict::Fail);
        assert!(report
            .reason_codes
            .contains(&"REQUIRED_CHECK_MISSING".to_owned()));
    }

    #[test]
    fn unverified_policy_never_yields_pass() {
        let mut report = passing_report();
        report.required_policy.verified = false;
        let required = vec!["BAM / Actual Fmt Clippy Test".to_owned()];
        assert_eq!(report.evaluate(&context(&required)), Verdict::Unknown);
        assert!(report
            .reason_codes
            .contains(&"REQUIRED_POLICY_UNVERIFIED".to_owned()));

        let mut unavailable = passing_report();
        let mut blocked_context = context(&required);
        blocked_context.policy_unavailable = true;
        assert_eq!(unavailable.evaluate(&blocked_context), Verdict::Blocked);
        assert!(unavailable
            .reason_codes
            .contains(&"REQUIRED_POLICY_UNAVAILABLE".to_owned()));
    }

    #[test]
    fn blocking_review_conflict_and_local_execution_are_failures() {
        let required = vec!["BAM / Actual Fmt Clippy Test".to_owned()];

        let mut reviewed = passing_report();
        reviewed.review.blocking_reviews = vec!["changes requested by rustburn".to_owned()];
        assert_eq!(reviewed.evaluate(&context(&required)), Verdict::Fail);
        assert!(reviewed
            .reason_codes
            .contains(&"REVIEW_BLOCKING".to_owned()));

        let mut conflicted = passing_report();
        conflicted.mergeability = Mergeability::Dirty;
        assert_eq!(conflicted.evaluate(&context(&required)), Verdict::Fail);
        assert!(conflicted
            .reason_codes
            .contains(&"MERGE_CONFLICT".to_owned()));

        let mut local = passing_report();
        local.local_tests = 1;
        assert_eq!(local.evaluate(&context(&required)), Verdict::Fail);
        assert!(local
            .reason_codes
            .contains(&"LOCAL_EXECUTION_VIOLATION".to_owned()));
    }

    #[test]
    fn criteria_and_mergeability_normalisation() {
        let required = vec!["BAM / Actual Fmt Clippy Test".to_owned()];

        let mut unevidenced = passing_report();
        unevidenced.criteria[0].verdict = Verdict::Unknown;
        assert_eq!(unevidenced.evaluate(&context(&required)), Verdict::Unknown);
        assert!(unevidenced
            .reason_codes
            .contains(&"MISSING_REQUIRED_EVIDENCE".to_owned()));

        let mut failing = passing_report();
        failing.criteria[0].verdict = Verdict::Fail;
        assert_eq!(failing.evaluate(&context(&required)), Verdict::Fail);
        assert!(failing
            .reason_codes
            .contains(&"CRITERION_FAILED".to_owned()));

        assert_eq!(Mergeability::parse("clean"), Mergeability::Clean);
        assert_eq!(Mergeability::parse("DIRTY"), Mergeability::Dirty);
        assert_eq!(Mergeability::parse("has_hooks"), Mergeability::Unknown);
        assert_eq!(Mergeability::parse("behind"), Mergeability::Behind);
    }

    #[test]
    fn head_binding_requires_a_full_sha() {
        assert!(is_full_sha(HEAD));
        assert!(!is_full_sha("2101ed8"));
        assert!(!is_full_sha("40-char-full-sha"));

        let mut abbreviated = passing_report();
        abbreviated.pr.head_sha = "2101ed8".to_owned();
        abbreviated.checks[0].sha = "2101ed8".to_owned();
        let required = vec!["BAM / Actual Fmt Clippy Test".to_owned()];
        assert_eq!(abbreviated.evaluate(&context(&required)), Verdict::Unknown);
        assert!(abbreviated.reason_codes.contains(&"STALE_HEAD".to_owned()));
    }

    #[test]
    fn verdicts_rank_failure_above_blocked_above_unknown() {
        assert!(Verdict::Fail.severity() > Verdict::Blocked.severity());
        assert!(Verdict::Blocked.severity() > Verdict::Unknown.severity());
        assert!(Verdict::Unknown.severity() > Verdict::Pass.severity());
        assert!(!Verdict::Fail.allows_ready());
        assert!(!Verdict::Blocked.allows_ready());
        assert!(!Verdict::Unknown.allows_ready());
    }
}

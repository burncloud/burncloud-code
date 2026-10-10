//! Acceptance controller.
//!
//! Requirements: M11-01 (read issue acceptance), M11-02 (change scope),
//! M11-03 (required caller migration), M11-04 (old code removal), M11-05 (required
//! checks), M11-06 (tests actually executed), M11-07 (real PostgreSQL evidence),
//! M11-08 (migration invariants), M11-09 (PR description), M11-10 (structured report),
//! M11-11 (no automatic Ready without evidence), M11-12 (a judgement call can never
//! replace a hard gate); design M11-A .. M11-F.
//!
//! The controller only consumes evidence produced elsewhere. It never invents a pass:
//! when the underlying evidence verdict is not PASS, acceptance fails regardless of how
//! convincing any prose or model judgement is.
use crate::evidence::{CheckEvidence, EvidenceReport, Verdict};
use serde::{Deserialize, Serialize};

/// Machine-checkable findings, each carrying the concrete subject it refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptanceFinding {
    MissingAcceptanceCriteria,
    PathOutOfScope(String),
    ForbiddenPathTouched(String),
    MissingPrSection(String),
    MissingRequiredCiJob(String),
    RequiredJobNotPassing(String),
    RequiredJobNotExecuted(String),
    PostgresEvidenceMissing,
    InvariantViolated(String),
    EvidenceVerdictNotPass,
    StaleEvidence,
}

impl AcceptanceFinding {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingAcceptanceCriteria => "MISSING_ACCEPTANCE_CRITERIA",
            Self::PathOutOfScope(_) => "PATH_OUT_OF_SCOPE",
            Self::ForbiddenPathTouched(_) => "FORBIDDEN_PATH_TOUCHED",
            Self::MissingPrSection(_) => "MISSING_PR_SECTION",
            Self::MissingRequiredCiJob(_) => "MISSING_REQUIRED_CI_JOB",
            Self::RequiredJobNotPassing(_) => "REQUIRED_JOB_NOT_PASSING",
            Self::RequiredJobNotExecuted(_) => "REQUIRED_JOB_NOT_EXECUTED",
            Self::PostgresEvidenceMissing => "POSTGRES_EVIDENCE_MISSING",
            Self::InvariantViolated(_) => "INVARIANT_VIOLATED",
            Self::EvidenceVerdictNotPass => "EVIDENCE_VERDICT_NOT_PASS",
            Self::StaleEvidence => "STALE_EVIDENCE",
        }
    }

    pub fn subject(&self) -> Option<&str> {
        match self {
            Self::PathOutOfScope(subject)
            | Self::ForbiddenPathTouched(subject)
            | Self::MissingPrSection(subject)
            | Self::MissingRequiredCiJob(subject)
            | Self::RequiredJobNotPassing(subject)
            | Self::RequiredJobNotExecuted(subject)
            | Self::InvariantViolated(subject) => Some(subject),
            _ => None,
        }
    }

    /// Stable, greppable rendering used in the report.
    pub fn render(&self) -> String {
        match self.subject() {
            Some(subject) => format!("{}:{subject}", self.code()),
            None => self.code().to_owned(),
        }
    }
}

/// Acceptance section of the issue template (`docs/requirements/task-breakdown.md`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceCriteria {
    pub ac_ids: Vec<String>,
    pub negative_tests: Vec<String>,
    pub required_ci_jobs: Vec<String>,
    pub evidence_uri: Option<String>,
}

impl AcceptanceCriteria {
    /// An issue without written acceptance criteria may not be accepted.
    pub fn is_specified(&self) -> bool {
        !self.ac_ids.is_empty()
    }
}

/// Read the acceptance section out of an issue body (M11-01).
pub fn parse_acceptance(body: &str) -> AcceptanceCriteria {
    let mut criteria = AcceptanceCriteria::default();
    for line in body.lines() {
        let trimmed = line
            .trim()
            .trim_start_matches(|c| c == '-' || c == '*' || c == ' ');
        let Some((label, value)) = trimmed.split_once(':') else {
            continue;
        };
        let label = label.trim().to_ascii_lowercase();
        let value = value.trim();
        match label.as_str() {
            "negative tests" | "negative test" => criteria.negative_tests = split_list(value),
            "required ci jobs" | "required ci job" => criteria.required_ci_jobs = split_list(value),
            "evidence uri" | "evidence" => {
                if !value.is_empty() {
                    criteria.evidence_uri = Some(value.to_owned());
                }
            }
            _ => {
                let upper = label.to_ascii_uppercase();
                if upper.starts_with("AC-") && !value.is_empty() {
                    criteria.ac_ids.push(upper);
                }
            }
        }
    }
    criteria.ac_ids.sort();
    criteria.ac_ids.dedup();
    criteria
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(|c: char| c == ',' || c == ';')
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Declared change scope of a task (M11-B).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeScope {
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
}

impl ChangeScope {
    pub fn declares(&self, path: &str) -> bool {
        self.allowed_paths
            .iter()
            .any(|pattern| path_matches(pattern, path))
    }

    pub fn forbids(&self, path: &str) -> bool {
        self.forbidden_paths
            .iter()
            .any(|pattern| path_matches(pattern, path))
    }
}

/// `dir/**` and `dir/*` style prefixes, or an exact path.
fn path_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return false;
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.starts_with(prefix);
    }
    pattern == path
}

/// A claim computed by the caller (a review, an analysis tool or a migration check)
/// that acceptance treats as a hard gate. The controller does not perform semantic
/// code analysis itself, and says so instead of pretending otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invariant {
    pub id: String,
    pub satisfied: bool,
}

/// PR description sections required by `docs/README.md` change management: the change
/// list must name requirement ids, test ids, an executable CI reference and the HEAD
/// sha that was verified.
pub const REQUIRED_PR_SECTIONS: [&str; 3] = ["Requirement IDs", "Test IDs", "HEAD SHA"];
pub const REQUIRED_PR_CI_MARKERS: [&str; 2] = ["CI Job", "Run ID"];

#[derive(Debug, Clone)]
pub struct AcceptanceInput<'a> {
    pub report: &'a EvidenceReport,
    /// HEAD sha the pull request currently has.
    pub expected_head_sha: &'a str,
    pub criteria: &'a AcceptanceCriteria,
    pub scope: &'a ChangeScope,
    pub changed_paths: &'a [String],
    pub pr_body: &'a str,
    pub invariants: &'a [Invariant],
    /// True when the acceptance criteria require real PostgreSQL evidence.
    pub requires_postgres: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceReport {
    pub verdict: Verdict,
    pub findings: Vec<String>,
    pub acceptance_ids: Vec<String>,
    pub required_ci_jobs: Vec<String>,
    pub checked_sha: String,
    pub evidence_uri: Option<String>,
}

impl AcceptanceReport {
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }

    pub fn from_json(raw: &str) -> Result<Self, String> {
        serde_json::from_str(raw).map_err(|error| error.to_string())
    }
}

/// Evaluate acceptance. The result is PASS only when every hard gate holds; a missing
/// gate is a failure, never an assumption.
pub fn evaluate(input: &AcceptanceInput<'_>) -> AcceptanceReport {
    let mut findings: Vec<AcceptanceFinding> = Vec::new();

    // M11-11 / M11-12: acceptance may not overrule the evidence verdict.
    if input.report.verdict != Verdict::Pass {
        findings.push(AcceptanceFinding::EvidenceVerdictNotPass);
    }
    if input.report.pr.head_sha != input.expected_head_sha {
        findings.push(AcceptanceFinding::StaleEvidence);
    }

    if !input.criteria.is_specified() {
        findings.push(AcceptanceFinding::MissingAcceptanceCriteria);
    }

    // M11-02: every changed path must be inside the declared scope.
    for path in input.changed_paths {
        if input.scope.forbids(path) {
            findings.push(AcceptanceFinding::ForbiddenPathTouched(path.clone()));
            continue;
        }
        if !input.scope.allowed_paths.is_empty() && !input.scope.declares(path) {
            findings.push(AcceptanceFinding::PathOutOfScope(path.clone()));
        }
    }

    // M11-09: the description must carry the documented change-management sections.
    for section in REQUIRED_PR_SECTIONS {
        if !input.pr_body.contains(section) {
            findings.push(AcceptanceFinding::MissingPrSection(section.to_owned()));
        }
    }
    if !REQUIRED_PR_CI_MARKERS
        .iter()
        .any(|marker| input.pr_body.contains(marker))
    {
        findings.push(AcceptanceFinding::MissingPrSection(
            "CI Job / Run ID".to_owned(),
        ));
    }

    // M11-05 / M11-06: required jobs must exist, pass, and really have executed.
    for required in &input.criteria.required_ci_jobs {
        match find_check(input.report, required) {
            None => findings.push(AcceptanceFinding::MissingRequiredCiJob(required.clone())),
            Some(check) => {
                if !check.actually_executed {
                    findings.push(AcceptanceFinding::RequiredJobNotExecuted(required.clone()));
                }
                if !check.is_pass_for(input.expected_head_sha) {
                    findings.push(AcceptanceFinding::RequiredJobNotPassing(required.clone()));
                }
            }
        }
    }

    // M11-07: a database-touching change needs real PostgreSQL evidence.
    if input.requires_postgres && !has_passing_postgres_check(input.report, input.expected_head_sha)
    {
        findings.push(AcceptanceFinding::PostgresEvidenceMissing);
    }

    // M11-03 / M11-04 / M11-08: caller-computed invariants are hard gates.
    for invariant in input.invariants {
        if !invariant.satisfied {
            findings.push(AcceptanceFinding::InvariantViolated(invariant.id.clone()));
        }
    }

    let mut rendered: Vec<String> = findings.iter().map(AcceptanceFinding::render).collect();
    rendered.sort();
    rendered.dedup();

    AcceptanceReport {
        verdict: if rendered.is_empty() {
            Verdict::Pass
        } else {
            Verdict::Fail
        },
        findings: rendered,
        acceptance_ids: input.criteria.ac_ids.clone(),
        required_ci_jobs: input.criteria.required_ci_jobs.clone(),
        checked_sha: input.expected_head_sha.to_owned(),
        evidence_uri: input.criteria.evidence_uri.clone(),
    }
}

/// Context names are matched exactly or as a suffix-free substring, because the
/// repository's required policy names jobs ("Cargo Test") while the observed check
/// context is prefixed ("check / Cargo Test").
fn find_check<'a>(report: &'a EvidenceReport, required: &str) -> Option<&'a CheckEvidence> {
    report
        .checks
        .iter()
        .find(|check| check.context == required || check.context.contains(required))
}

fn has_passing_postgres_check(report: &EvidenceReport, expected_sha: &str) -> bool {
    report.checks.iter().any(|check| {
        let context = check.context.to_ascii_lowercase();
        (context.contains("postgres") || context.contains("postgresql"))
            && check.is_pass_for(expected_sha)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::{
        CheckConclusion, CheckStatus, Criterion, IssueRef, Mergeability, PullRequestRef,
        RequiredPolicy, ReviewState, SCHEMA_VERSION,
    };

    const HEAD: &str = "f0a3ee987d5547ce487444de5132e4475907eec3";

    const ISSUE_BODY: &str = "\
## 验收
AC-1: 工作区互不干扰
AC-2: 清理不删除未保存变更
Negative tests: dirty worktree, force push attempt
Required CI jobs: BAM / Actual Fmt Clippy Test
Evidence URI: https://github.com/burncloud/burncloud-code/actions/runs/38078040063
";

    fn check(context: &str, conclusion: CheckConclusion, executed: bool) -> CheckEvidence {
        CheckEvidence {
            context: context.to_owned(),
            app: Some("github-actions".to_owned()),
            run_id: Some(38078040063),
            job_id: Some(114284999999),
            check_run_id: None,
            sha: HEAD.to_owned(),
            status: CheckStatus::Completed,
            conclusion,
            actually_executed: executed,
            url: Some("https://example.invalid/run".to_owned()),
        }
    }

    fn passing_report() -> EvidenceReport {
        EvidenceReport {
            schema_version: SCHEMA_VERSION.to_owned(),
            task_id: "task-30".to_owned(),
            issue: IssueRef {
                repository_id: 1,
                number: 30,
                node_id: "I_kwDOA".to_owned(),
            },
            pr: PullRequestRef {
                repository_id: 1,
                number: 31,
                node_id: "PR_kwDOA".to_owned(),
                head_sha: HEAD.to_owned(),
            },
            base_sha: "5357198a1b2c3d4e5f60718293a4b5c6d7e8f901".to_owned(),
            merge_ref_sha: None,
            required_policy: RequiredPolicy {
                source: "github_rulesets".to_owned(),
                verified: true,
                version: Some("ruleset:24846723".to_owned()),
            },
            checks: vec![check(
                "BAM / Actual Fmt Clippy Test",
                CheckConclusion::Success,
                true,
            )],
            criteria: vec![Criterion {
                id: "M11-05".to_owned(),
                test_ids: vec!["acceptance::tests::required_jobs".to_owned()],
                verdict: Verdict::Pass,
                evidence_ids: vec!["run:38078040063".to_owned()],
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
            evaluated_at: "2026-10-11T03:05:00Z".to_owned(),
        }
    }

    fn scope() -> ChangeScope {
        ChangeScope {
            allowed_paths: vec![
                "crates/platform/code/agent-manager/src/**".to_owned(),
                "crates/platform/code/agent-manager/tests/**".to_owned(),
            ],
            forbidden_paths: vec![".github/**".to_owned()],
        }
    }

    const PR_BODY: &str = "\
Requirement IDs: M11-01
Test IDs: acceptance::tests::happy_path
CI Job / Run ID: 38078040063
Current HEAD SHA: f0a3ee987d5547ce487444de5132e4475907eec3
";

    fn input<'a>(
        report: &'a EvidenceReport,
        criteria: &'a AcceptanceCriteria,
        scope: &'a ChangeScope,
        paths: &'a [String],
        invariants: &'a [Invariant],
    ) -> AcceptanceInput<'a> {
        AcceptanceInput {
            report,
            expected_head_sha: HEAD,
            criteria,
            scope,
            changed_paths: paths,
            pr_body: PR_BODY,
            invariants,
            requires_postgres: false,
        }
    }

    #[test]
    fn acceptance_criteria_are_read_from_the_issue_template() {
        let criteria = parse_acceptance(ISSUE_BODY);
        assert_eq!(criteria.ac_ids, vec!["AC-1".to_owned(), "AC-2".to_owned()]);
        assert_eq!(
            criteria.negative_tests,
            vec!["dirty worktree".to_owned(), "force push attempt".to_owned()]
        );
        assert_eq!(
            criteria.required_ci_jobs,
            vec!["BAM / Actual Fmt Clippy Test".to_owned()]
        );
        assert!(criteria
            .evidence_uri
            .as_deref()
            .unwrap()
            .starts_with("https://github.com/"));
        assert!(criteria.is_specified());

        let empty = parse_acceptance("## 实施\n- [ ] do work\n");
        assert!(!empty.is_specified());
        assert!(empty.required_ci_jobs.is_empty());
    }

    #[test]
    fn a_fully_evidenced_change_is_accepted() {
        let report = passing_report();
        let criteria = parse_acceptance(ISSUE_BODY);
        let scope = scope();
        let paths = vec![
            "crates/platform/code/agent-manager/src/acceptance.rs".to_owned(),
            "crates/platform/code/agent-manager/tests/github_auth.rs".to_owned(),
        ];
        let report_out = evaluate(&input(&report, &criteria, &scope, &paths, &[]));
        assert_eq!(report_out.verdict, Verdict::Pass);
        assert!(report_out.findings.is_empty());
        assert_eq!(report_out.checked_sha, HEAD);
        assert_eq!(
            report_out.required_ci_jobs,
            vec!["BAM / Actual Fmt Clippy Test".to_owned()]
        );

        let json = report_out.to_json().unwrap();
        assert!(json.contains("\"verdict\": \"PASS\""));
        assert_eq!(AcceptanceReport::from_json(&json).unwrap(), report_out);
    }

    #[test]
    fn change_scope_is_enforced_against_allowed_and_forbidden_paths() {
        let report = passing_report();
        let criteria = parse_acceptance(ISSUE_BODY);
        let scope = scope();
        let paths = vec![
            "crates/platform/code/agent-manager/src/lib.rs".to_owned(),
            "docs/readme.md".to_owned(),
            ".github/workflows/bam.yml".to_owned(),
        ];
        let outcome = evaluate(&input(&report, &criteria, &scope, &paths, &[]));
        assert_eq!(outcome.verdict, Verdict::Fail);
        assert!(outcome
            .findings
            .contains(&"PATH_OUT_OF_SCOPE:docs/readme.md".to_owned()));
        assert!(outcome
            .findings
            .contains(&"FORBIDDEN_PATH_TOUCHED:.github/workflows/bam.yml".to_owned()));
    }

    #[test]
    fn a_description_without_the_required_sections_is_rejected() {
        let report = passing_report();
        let criteria = parse_acceptance(ISSUE_BODY);
        let scope = scope();
        let mut broken = input(&report, &criteria, &scope, &[], &[]);
        broken.pr_body = "Implemented the thing.";
        let outcome = evaluate(&broken);
        assert_eq!(outcome.verdict, Verdict::Fail);
        assert!(outcome
            .findings
            .contains(&"MISSING_PR_SECTION:Requirement IDs".to_owned()));
        assert!(outcome
            .findings
            .contains(&"MISSING_PR_SECTION:Test IDs".to_owned()));
        assert!(outcome
            .findings
            .contains(&"MISSING_PR_SECTION:HEAD SHA".to_owned()));
        assert!(outcome
            .findings
            .contains(&"MISSING_PR_SECTION:CI Job / Run ID".to_owned()));
    }

    #[test]
    fn an_issue_without_acceptance_criteria_cannot_be_accepted() {
        let report = passing_report();
        let criteria = parse_acceptance("no acceptance here");
        let scope = scope();
        let outcome = evaluate(&input(&report, &criteria, &scope, &[], &[]));
        assert_eq!(outcome.verdict, Verdict::Fail);
        assert!(outcome
            .findings
            .contains(&"MISSING_ACCEPTANCE_CRITERIA".to_owned()));
    }

    #[test]
    fn required_jobs_must_exist_pass_and_have_executed() {
        let criteria = parse_acceptance(ISSUE_BODY);
        let scope = scope();

        // Missing entirely.
        let mut missing = passing_report();
        missing.checks.clear();
        let outcome = evaluate(&input(&missing, &criteria, &scope, &[], &[]));
        assert_eq!(outcome.verdict, Verdict::Fail);
        assert!(outcome
            .findings
            .contains(&"MISSING_REQUIRED_CI_JOB:BAM / Actual Fmt Clippy Test".to_owned()));

        // Present but skipped.
        let mut skipped = passing_report();
        skipped.checks = vec![check(
            "BAM / Actual Fmt Clippy Test",
            CheckConclusion::Skipped,
            false,
        )];
        let outcome = evaluate(&input(&skipped, &criteria, &scope, &[], &[]));
        assert_eq!(outcome.verdict, Verdict::Fail);
        assert!(outcome
            .findings
            .contains(&"REQUIRED_JOB_NOT_EXECUTED:BAM / Actual Fmt Clippy Test".to_owned()));
        assert!(outcome
            .findings
            .contains(&"REQUIRED_JOB_NOT_PASSING:BAM / Actual Fmt Clippy Test".to_owned()));

        // Green but never executed.
        let mut not_executed = passing_report();
        not_executed.checks = vec![check(
            "BAM / Actual Fmt Clippy Test",
            CheckConclusion::Success,
            false,
        )];
        let outcome = evaluate(&input(&not_executed, &criteria, &scope, &[], &[]));
        assert_eq!(outcome.verdict, Verdict::Fail);
        assert!(outcome
            .findings
            .contains(&"REQUIRED_JOB_NOT_EXECUTED:BAM / Actual Fmt Clippy Test".to_owned()));
    }

    #[test]
    fn required_checks_may_be_named_with_the_repository_prefix() {
        let mut report = passing_report();
        let mut criteria = parse_acceptance(ISSUE_BODY);
        criteria.required_ci_jobs = vec!["Cargo Test".to_owned()];
        report.checks = vec![check("check / Cargo Test", CheckConclusion::Success, true)];
        let scope = scope();
        let outcome = evaluate(&input(&report, &criteria, &scope, &[], &[]));
        assert_eq!(outcome.verdict, Verdict::Pass);
    }

    #[test]
    fn postgres_acceptance_requires_real_postgres_evidence() {
        let report = passing_report();
        let mut criteria = parse_acceptance(ISSUE_BODY);
        criteria.required_ci_jobs.clear();
        let scope = scope();
        let mut strict = input(&report, &criteria, &scope, &[], &[]);
        strict.requires_postgres = true;
        let outcome = evaluate(&strict);
        assert_eq!(outcome.verdict, Verdict::Fail);
        assert!(outcome
            .findings
            .contains(&"POSTGRES_EVIDENCE_MISSING".to_owned()));

        let mut with_postgres = passing_report();
        with_postgres.checks = vec![check(
            "Test / PostgreSQL integration",
            CheckConclusion::Success,
            true,
        )];
        let outcome = evaluate(&AcceptanceInput {
            report: &with_postgres,
            expected_head_sha: HEAD,
            criteria: &criteria,
            scope: &scope,
            changed_paths: &[],
            pr_body: PR_BODY,
            invariants: &[],
            requires_postgres: true,
        });
        assert_eq!(outcome.verdict, Verdict::Pass);
    }

    #[test]
    fn unsatisfied_invariants_block_acceptance() {
        let report = passing_report();
        let mut criteria = parse_acceptance(ISSUE_BODY);
        criteria.required_ci_jobs.clear();
        let scope = scope();
        let invariants = vec![
            Invariant {
                id: "no remaining callers of removed symbol".to_owned(),
                satisfied: false,
            },
            Invariant {
                id: "legacy module deleted".to_owned(),
                satisfied: true,
            },
        ];
        let outcome = evaluate(&input(&report, &criteria, &scope, &[], &invariants));
        assert_eq!(outcome.verdict, Verdict::Fail);
        assert!(outcome
            .findings
            .contains(&"INVARIANT_VIOLATED:no remaining callers of removed symbol".to_owned()));
        assert!(!outcome
            .findings
            .contains(&"INVARIANT_VIOLATED:legacy module deleted".to_owned()));
    }

    #[test]
    fn acceptance_never_overrules_the_evidence_verdict() {
        let mut report = passing_report();
        report.verdict = Verdict::Unknown;
        report.reason_codes = vec!["STALE_HEAD".to_owned()];
        let mut criteria = parse_acceptance(ISSUE_BODY);
        criteria.required_ci_jobs.clear();
        let scope = scope();
        let outcome = evaluate(&input(&report, &criteria, &scope, &[], &[]));
        assert_eq!(outcome.verdict, Verdict::Fail);
        assert!(outcome
            .findings
            .contains(&"EVIDENCE_VERDICT_NOT_PASS".to_owned()));
    }

    #[test]
    fn acceptance_from_a_stale_head_is_refused() {
        let report = passing_report();
        let mut criteria = parse_acceptance(ISSUE_BODY);
        criteria.required_ci_jobs.clear();
        let scope = scope();
        let outcome = evaluate(&AcceptanceInput {
            report: &report,
            expected_head_sha: "5357198a1b2c3d4e5f60718293a4b5c6d7e8f901",
            criteria: &criteria,
            scope: &scope,
            changed_paths: &[],
            pr_body: PR_BODY,
            invariants: &[],
            requires_postgres: false,
        });
        assert_eq!(outcome.verdict, Verdict::Fail);
        assert!(outcome.findings.contains(&"STALE_EVIDENCE".to_owned()));
    }

    #[test]
    fn scope_patterns_are_matched_precisely() {
        let scope = ChangeScope {
            allowed_paths: vec!["crates/platform/code/agent-manager/src/**".to_owned()],
            forbidden_paths: vec![".github/**".to_owned()],
        };
        assert!(scope.declares("crates/platform/code/agent-manager/src/lib.rs"));
        assert!(scope.declares("crates/platform/code/agent-manager/src"));
        assert!(!scope.declares("crates/platform/code/agent-manager/srcx/lib.rs"));
        assert!(scope.forbids(".github/workflows/x.yml"));
        assert!(!scope.forbids(".githubx/y.yml"));

        let exact = ChangeScope {
            allowed_paths: vec!["AGENTS.md".to_owned()],
            forbidden_paths: Vec::new(),
        };
        assert!(exact.declares("AGENTS.md"));
        assert!(!exact.declares("AGENTS.md.bak"));
    }
}

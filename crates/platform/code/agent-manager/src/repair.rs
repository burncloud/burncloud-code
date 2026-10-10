//! CI failure classification, fingerprinting and the repair loop budget.
//!
//! Requirements: M09-01 (parse failure logs), M09-02 (classify), M09-03 (hand the
//! error back to the owning agent), M09-04 (change the relevant code), M09-05 (push and
//! re-check CI), M09-06 (do not repair while CI is pending), M09-07 (keep repairing new
//! errors), M09-08 (back off when the same error keeps failing), M09-09 (never change
//! business code for an environment error), M09-10 (record every repair), M09-11 (stop
//! at the retry ceiling), M09-12 (never fix CI by weakening the tests); design M09-A..
//! M09-G.
//!
//! The loop only decides and records. It performs no local execution and no network
//! call: the caller pushes the change and re-observes GitHub.
use crate::{config::Config, event_sync::Backoff, storage};
use rusqlite::Connection;

/// Failure classes (M09-A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureClass {
    RunnerInfrastructure,
    Environment,
    Format,
    Lint,
    Compile,
    Test,
    DependencyPolicy,
    Database,
    Unknown,
}

impl FailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RunnerInfrastructure => "RUNNER_INFRASTRUCTURE",
            Self::Environment => "ENVIRONMENT",
            Self::Format => "FORMAT",
            Self::Lint => "LINT",
            Self::Compile => "COMPILE",
            Self::Test => "TEST",
            Self::DependencyPolicy => "DEPENDENCY_POLICY",
            Self::Database => "DATABASE",
            Self::Unknown => "UNKNOWN",
        }
    }

    /// M09-09: an infrastructure or environment failure is not a defect in the code
    /// under review, so business code must not be modified for it.
    pub fn is_repairable_in_code(self) -> bool {
        !matches!(self, Self::RunnerInfrastructure | Self::Environment)
    }
}

/// Ordered markers; the first class whose marker appears wins, so an infrastructure
/// failure is never mistaken for a code defect (M09-A, M09-09).
const CLASS_MARKERS: [(&str, FailureClass); 17] = [
    (
        "no space left on device",
        FailureClass::RunnerInfrastructure,
    ),
    (
        "the runner has received a shutdown signal",
        FailureClass::RunnerInfrastructure,
    ),
    (
        "lost communication with the server",
        FailureClass::RunnerInfrastructure,
    ),
    ("could not resolve host", FailureClass::Environment),
    (
        "temporary failure in name resolution",
        FailureClass::Environment,
    ),
    ("connection refused", FailureClass::Environment),
    ("error[e", FailureClass::Compile),
    ("could not compile", FailureClass::Compile),
    ("clippy::", FailureClass::Lint),
    ("-d warnings", FailureClass::Lint),
    ("rustfmt", FailureClass::Format),
    ("diff in", FailureClass::Format),
    ("cargo fmt", FailureClass::Format),
    ("cargo-deny", FailureClass::DependencyPolicy),
    ("denied by", FailureClass::DependencyPolicy),
    ("test failed", FailureClass::Test),
    ("panicked at", FailureClass::Test),
];

/// Parse a CI log and classify it (M09-01, M09-02).
pub fn classify_log(log: &str) -> FailureClass {
    let lowered = log.to_ascii_lowercase();
    for (marker, class) in CLASS_MARKERS {
        if lowered.contains(marker) {
            return class;
        }
    }
    if lowered.contains("psql:") || lowered.contains("postgres") {
        return FailureClass::Database;
    }
    if lowered.contains("assertion") || lowered.contains("failed") {
        return FailureClass::Test;
    }
    FailureClass::Unknown
}

/// FNV-1a: stable across processes and toolchain upgrades, unlike `DefaultHasher`.
fn stable_hash(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Collapse volatile parts (line numbers, paths, shas, counts, durations) so a rerun of
/// the same defect fingerprints identically (M09-B).
fn normalise(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut inside_digits = false;
    for ch in line.trim().chars() {
        if ch.is_ascii_digit() {
            if !inside_digits {
                out.push('#');
            }
            inside_digits = true;
        } else {
            inside_digits = false;
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_signal_line(line: &str) -> bool {
    let lowered = line.to_ascii_lowercase();
    lowered.contains("error") || lowered.contains("failed") || lowered.contains("panicked")
}

/// Fingerprint the first meaningful failure in a log (M09-B).
pub fn fingerprint(log: &str) -> String {
    let signal = log
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && is_signal_line(line))
        .or_else(|| log.lines().map(str::trim).find(|line| !line.is_empty()))
        .unwrap_or("EMPTY_LOG");
    format!("{:016x}", stable_hash(&normalise(signal)))
}

/// Reasons a proposed repair is refused (M09-12, and the repository's own P0 rules).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairRefusal {
    ReducesTestStrictness(String),
    TouchesCiConfiguration(String),
    RequiresLocalExecution(String),
}

impl RepairRefusal {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ReducesTestStrictness(_) => "REPAIR_WEAKENS_TESTS",
            Self::TouchesCiConfiguration(_) => "REPAIR_TOUCHES_CI",
            Self::RequiresLocalExecution(_) => "REPAIR_REQUIRES_LOCAL_EXECUTION",
        }
    }

    pub fn marker(&self) -> &str {
        match self {
            Self::ReducesTestStrictness(marker)
            | Self::TouchesCiConfiguration(marker)
            | Self::RequiresLocalExecution(marker) => marker,
        }
    }
}

const TEST_WEAKENING_MARKERS: [&str; 9] = [
    "#[ignore]",
    "--no-verify",
    "--no-fail-fast",
    "-- --skip",
    "continue-on-error",
    "|| true",
    "allow(clippy::all)",
    "allow(dead_code)",
    "#[allow(unused)]",
];
const CI_MARKERS: [&str; 2] = [".github/workflows/", "ci-github-hosted"];
const LOCAL_EXECUTION_MARKERS: [&str; 3] = ["cargo test", "cargo build", "cargo clippy"];

/// Review the *text* of a proposed repair.
///
/// This is a heuristic text review, not a proof: it refuses repairs that visibly weaken
/// the gate, touch CI, or instruct a local build/test. The acceptance controller remains
/// the authoritative gate.
pub fn review_repair_text(patch: &str) -> Result<(), RepairRefusal> {
    let lowered = patch.to_ascii_lowercase();
    for marker in TEST_WEAKENING_MARKERS {
        if lowered.contains(marker.to_ascii_lowercase().as_str()) {
            return Err(RepairRefusal::ReducesTestStrictness(marker.to_owned()));
        }
    }
    for marker in CI_MARKERS {
        if lowered.contains(marker.to_ascii_lowercase().as_str()) {
            return Err(RepairRefusal::TouchesCiConfiguration(marker.to_owned()));
        }
    }
    for marker in LOCAL_EXECUTION_MARKERS {
        if lowered.contains(marker.to_ascii_lowercase().as_str()) {
            return Err(RepairRefusal::RequiresLocalExecution(marker.to_owned()));
        }
    }
    Ok(())
}

/// Retry budget taken from configuration (M09-11, M09-G).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairBudget {
    pub max_attempts_per_error: u32,
    pub max_total_attempts: u32,
}

impl RepairBudget {
    pub fn new(max_attempts_per_error: u32, max_total_attempts: u32) -> Result<Self, String> {
        if max_attempts_per_error == 0 {
            return Err("per-error budget must be positive".into());
        }
        if max_total_attempts < max_attempts_per_error {
            return Err("total budget must cover at least one error budget".into());
        }
        Ok(Self {
            max_attempts_per_error,
            max_total_attempts,
        })
    }

    pub fn from_config(config: &Config) -> Result<Self, String> {
        Self::new(
            config.manager.max_repair_attempts_per_error,
            config.manager.max_total_attempts_per_task,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiState {
    Pending,
    Failed,
    Passed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairOutcome {
    /// The push fixed this fingerprint.
    Fixed,
    /// Still failing with the same fingerprint.
    StillFailing,
    /// A different fingerprint appeared (M09-07).
    NewError,
}

/// What the caller must do next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairDecision {
    /// Resume the owning agent session with the failing context (M09-03, M09-04).
    Repair {
        fingerprint: String,
        class: FailureClass,
        attempt_number: u32,
        delay_seconds: i64,
        agent_session: Option<String>,
        ci_run_id: Option<u64>,
    },
    /// CI has not finished; starting another repair now would duplicate work (M09-06).
    WaitCi,
    NoRepairNeeded,
    /// Environment/infrastructure failure: back off and re-run, never touch code (M09-09).
    EnvironmentBackoff {
        class: FailureClass,
        delay_seconds: i64,
    },
    /// The retry ceiling was reached (M09-11, M09-G).
    BudgetExhausted {
        fingerprint: String,
        attempts: u32,
        total_attempts: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairRecord {
    pub task_id: String,
    pub fingerprint: String,
    pub class: FailureClass,
    pub attempt_number: u32,
    pub outcome: RepairOutcome,
    pub ci_run_id: Option<u64>,
    pub at: i64,
}

impl RepairRecord {
    pub fn attempt_id(&self) -> String {
        format!(
            "{}:{}:{}",
            self.task_id, self.fingerprint, self.attempt_number
        )
    }
}

/// The failure being repaired, with the owning session and the CI run it came from.
#[derive(Debug, Clone)]
pub struct CiFailure<'a> {
    pub task_id: &'a str,
    pub agent_session: Option<&'a str>,
    pub ci_run_id: Option<u64>,
    pub log: &'a str,
}

/// Repair loop state (M09-F re-tests by re-observing CI, M09-10 keeps history).
#[derive(Debug)]
pub struct RepairLoop {
    budget: RepairBudget,
    per_fingerprint: std::collections::BTreeMap<String, u32>,
    total_attempts: u32,
    backoff: Backoff,
    history: Vec<RepairRecord>,
}

impl RepairLoop {
    pub fn new(
        budget: RepairBudget,
        base_backoff_seconds: i64,
        max_backoff_seconds: i64,
    ) -> Result<Self, String> {
        Ok(Self {
            budget,
            per_fingerprint: std::collections::BTreeMap::new(),
            total_attempts: 0,
            backoff: Backoff::new(base_backoff_seconds, max_backoff_seconds)?,
            history: Vec::new(),
        })
    }

    pub fn budget(&self) -> RepairBudget {
        self.budget
    }

    pub fn total_attempts(&self) -> u32 {
        self.total_attempts
    }

    pub fn history(&self) -> &[RepairRecord] {
        &self.history
    }

    /// Decide from the observed CI state alone (M09-06).
    pub fn on_ci_state(&self, state: CiState) -> Option<RepairDecision> {
        match state {
            CiState::Pending => Some(RepairDecision::WaitCi),
            CiState::Passed => Some(RepairDecision::NoRepairNeeded),
            CiState::Failed => None,
        }
    }

    /// Plan the next repair for a failing run.
    pub fn plan(&mut self, failure: &CiFailure<'_>, at: i64) -> Result<RepairDecision, String> {
        if failure.task_id.trim().is_empty() {
            return Err("repair planning requires a task id".into());
        }
        let class = classify_log(failure.log);
        let key = fingerprint(failure.log);

        // M09-09: an environment failure is not repaired in code.
        if !class.is_repairable_in_code() {
            let delay = self.backoff.record_failure();
            self.history.push(RepairRecord {
                task_id: failure.task_id.to_owned(),
                fingerprint: key,
                class,
                attempt_number: 0,
                outcome: RepairOutcome::StillFailing,
                ci_run_id: failure.ci_run_id,
                at,
            });
            return Ok(RepairDecision::EnvironmentBackoff {
                class,
                delay_seconds: delay,
            });
        }

        let attempts = self.per_fingerprint.get(&key).copied().unwrap_or(0);
        if attempts >= self.budget.max_attempts_per_error
            || self.total_attempts >= self.budget.max_total_attempts
        {
            return Ok(RepairDecision::BudgetExhausted {
                fingerprint: key,
                attempts,
                total_attempts: self.total_attempts,
            });
        }

        // M09-08: the same error repeatedly means back off instead of hammering CI.
        let delay = if attempts == 0 {
            self.backoff.record_success();
            0
        } else {
            self.backoff.record_failure()
        };
        let attempt_number = attempts + 1;
        self.per_fingerprint.insert(key.clone(), attempt_number);
        self.total_attempts += 1;
        self.history.push(RepairRecord {
            task_id: failure.task_id.to_owned(),
            fingerprint: key.clone(),
            class,
            attempt_number,
            outcome: RepairOutcome::StillFailing,
            ci_run_id: failure.ci_run_id,
            at,
        });

        Ok(RepairDecision::Repair {
            fingerprint: key,
            class,
            attempt_number,
            delay_seconds: delay,
            agent_session: failure.agent_session.map(str::to_owned),
            ci_run_id: failure.ci_run_id,
        })
    }

    /// Record what happened after the push (M09-05, M09-07, M09-10).
    pub fn record_outcome(&mut self, key: &str, outcome: RepairOutcome) {
        match outcome {
            RepairOutcome::Fixed => {
                self.per_fingerprint.remove(key);
                self.backoff.record_success();
            }
            // A new error starts its own budget; the previous one stops consuming it.
            RepairOutcome::NewError => {
                self.per_fingerprint.remove(key);
            }
            RepairOutcome::StillFailing => {}
        }
        if let Some(last) = self
            .history
            .iter_mut()
            .rev()
            .find(|record| record.fingerprint == key)
        {
            last.outcome = outcome;
        }
    }

    /// Persist one history entry into `repair_attempts` (M09-10).
    pub fn persist(&self, connection: &Connection, record: &RepairRecord) -> Result<(), String> {
        storage::record_repair_attempt(
            connection,
            &record.attempt_id(),
            &record.task_id,
            &record.fingerprint,
            record.attempt_number.max(1),
            match record.outcome {
                RepairOutcome::Fixed => "FIXED",
                RepairOutcome::StillFailing => "STILL_FAILING",
                RepairOutcome::NewError => "NEW_ERROR",
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUSTC_LOG: &str = "\
   Compiling bam v0.1.0
error[E0308]: mismatched types
  --> src/lib.rs:12:5
   |
12 |     let x: u8 = 300;
   |            --   ^^^ expected `u8`, found integer
error: could not compile `bam` (lib) due to 1 previous error
";

    fn budget() -> RepairBudget {
        RepairBudget::new(3, 12).unwrap()
    }

    fn loop_with(per_error: u32, total: u32) -> RepairLoop {
        RepairLoop::new(RepairBudget::new(per_error, total).unwrap(), 5, 60).unwrap()
    }

    fn ci_failure<'a>(task: &'a str, log: &'a str) -> CiFailure<'a> {
        CiFailure {
            task_id: task,
            agent_session: Some("session-1"),
            ci_run_id: Some(38078659490),
            log,
        }
    }

    #[test]
    fn logs_are_classified_and_infrastructure_wins() {
        assert_eq!(classify_log(RUSTC_LOG), FailureClass::Compile);
        assert_eq!(
            classify_log("error: this file is not formatted\nDiff in src/lib.rs"),
            FailureClass::Format
        );
        assert_eq!(
            classify_log("error: unnecessary clone\nclippy::redundant_clone"),
            FailureClass::Lint
        );
        assert_eq!(
            classify_log("test storage::tests::x ... FAILED\nassertion `left == right` failed"),
            FailureClass::Test
        );
        assert_eq!(
            classify_log("error: advisory RUSTSEC-2024-0001 denied by policy"),
            FailureClass::DependencyPolicy
        );
        assert_eq!(
            classify_log("psql: error: connection to server failed"),
            FailureClass::Database
        );
        assert_eq!(
            classify_log("No space left on device"),
            FailureClass::RunnerInfrastructure
        );
        assert_eq!(
            classify_log("error[E0308] mismatched types\nNo space left on device"),
            FailureClass::RunnerInfrastructure,
            "an infrastructure failure must not be mistaken for a code defect"
        );
        assert_eq!(classify_log("everything is fine"), FailureClass::Unknown);

        assert!(!FailureClass::RunnerInfrastructure.is_repairable_in_code());
        assert!(!FailureClass::Environment.is_repairable_in_code());
        assert!(FailureClass::Test.is_repairable_in_code());
    }

    #[test]
    fn fingerprints_are_stable_across_line_numbers_and_reruns() {
        let first = fingerprint("error[E0308]: mismatched types --> src/lib.rs:12:5");
        let second = fingerprint("error[E0308]: mismatched types --> src/lib.rs:99:1");
        assert_eq!(
            first, second,
            "volatile line numbers must not change the identity"
        );

        let other = fingerprint("error[E0599]: no method named `foo` found for struct `Bar`");
        assert_ne!(first, other);
        assert_eq!(first.len(), 16);
        assert_eq!(fingerprint("  \n \n"), fingerprint(""));
        assert_eq!(fingerprint(RUSTC_LOG), fingerprint(RUSTC_LOG));
    }

    #[test]
    fn repairs_that_weaken_the_gate_are_refused() {
        for (patch, expected) in [
            ("+#[ignore]\n fn flaky() {}", "REPAIR_WEAKENS_TESTS"),
            ("+    allow(dead_code)", "REPAIR_WEAKENS_TESTS"),
            ("+continue-on-error: true", "REPAIR_WEAKENS_TESTS"),
            ("+    allow(clippy::all)", "REPAIR_WEAKENS_TESTS"),
            ("+  .github/workflows/ci.yml", "REPAIR_TOUCHES_CI"),
            (
                "+ run cargo test locally",
                "REPAIR_REQUIRES_LOCAL_EXECUTION",
            ),
        ] {
            let code = match review_repair_text(patch) {
                Err(refusal) => refusal.code(),
                Ok(()) => panic!("{patch} must be refused"),
            };
            assert_eq!(code, expected, "{patch}");
        }
        assert!(review_repair_text("- let x: u8 = 300;\n+ let x: u16 = 300;").is_ok());
    }

    #[test]
    fn pending_ci_never_starts_a_second_repair() {
        let loop_ = loop_with(3, 12);
        assert_eq!(
            loop_.on_ci_state(CiState::Pending),
            Some(RepairDecision::WaitCi)
        );
        assert_eq!(
            loop_.on_ci_state(CiState::Passed),
            Some(RepairDecision::NoRepairNeeded)
        );
        assert_eq!(loop_.on_ci_state(CiState::Failed), None);
    }

    #[test]
    fn repeated_identical_errors_back_off_and_then_exhaust_the_budget() {
        let mut loop_ = loop_with(3, 12);
        let failure = ci_failure("task-1", RUSTC_LOG);

        let first = loop_.plan(&failure, 100).unwrap();
        match &first {
            RepairDecision::Repair {
                attempt_number,
                delay_seconds,
                class,
                agent_session,
                ..
            } => {
                assert_eq!(*attempt_number, 1);
                assert_eq!(*delay_seconds, 0);
                assert_eq!(*class, FailureClass::Compile);
                assert_eq!(agent_session.as_deref(), Some("session-1"));
            }
            other => panic!("expected a repair, got {other:?}"),
        }

        let second = loop_.plan(&failure, 200).unwrap();
        match second {
            RepairDecision::Repair {
                attempt_number,
                delay_seconds,
                ..
            } => {
                assert_eq!(attempt_number, 2);
                assert_eq!(delay_seconds, 5);
            }
            other => panic!("expected a repair, got {other:?}"),
        }

        let third = loop_.plan(&failure, 300).unwrap();
        match third {
            RepairDecision::Repair {
                attempt_number,
                delay_seconds,
                ..
            } => {
                assert_eq!(attempt_number, 3);
                assert_eq!(delay_seconds, 10);
            }
            other => panic!("expected a repair, got {other:?}"),
        }

        let exhausted = loop_.plan(&failure, 400).unwrap();
        match exhausted {
            RepairDecision::BudgetExhausted {
                attempts,
                total_attempts,
                ..
            } => {
                assert_eq!(attempts, 3);
                assert_eq!(total_attempts, 3);
            }
            other => panic!("expected an exhausted budget, got {other:?}"),
        }
        assert_eq!(loop_.total_attempts(), 3);
    }

    #[test]
    fn a_fixed_error_frees_its_budget_and_a_new_error_gets_its_own() {
        let mut loop_ = loop_with(2, 4);
        let failure = ci_failure("task-2", RUSTC_LOG);
        let key = fingerprint(RUSTC_LOG);

        // Attempt 1 consumes one unit of the total budget.
        assert!(matches!(
            loop_.plan(&failure, 1).unwrap(),
            RepairDecision::Repair {
                attempt_number: 1,
                ..
            }
        ));
        // Fixing it releases the per-error budget, so the same defect starts at 1 again.
        loop_.record_outcome(&key, RepairOutcome::Fixed);
        assert!(matches!(
            loop_.plan(&failure, 2).unwrap(),
            RepairDecision::Repair {
                attempt_number: 1,
                ..
            }
        ));

        // A different error is tracked under its own fingerprint (M09-07).
        let other = ci_failure("task-2", "test x ... FAILED\nassertion failed");
        assert!(matches!(
            loop_.plan(&other, 3).unwrap(),
            RepairDecision::Repair {
                attempt_number: 1,
                ..
            }
        ));

        // The same defect again: second and last permitted attempt for that fingerprint.
        assert!(matches!(
            loop_.plan(&failure, 4).unwrap(),
            RepairDecision::Repair {
                attempt_number: 2,
                ..
            }
        ));
        assert_eq!(loop_.total_attempts(), 4);

        // Both the per-error and the total ceiling now refuse further work (M09-11).
        match loop_.plan(&failure, 5).unwrap() {
            RepairDecision::BudgetExhausted {
                attempts,
                total_attempts,
                ..
            } => {
                assert_eq!(attempts, 2);
                assert_eq!(total_attempts, 4);
            }
            other => panic!("expected an exhausted budget, got {other:?}"),
        }
    }

    #[test]
    fn environment_failures_back_off_without_touching_code() {
        let mut loop_ = loop_with(3, 12);
        let failure = ci_failure("task-3", "No space left on device");
        let decision = loop_.plan(&failure, 10).unwrap();
        match decision {
            RepairDecision::EnvironmentBackoff {
                class,
                delay_seconds,
            } => {
                assert_eq!(class, FailureClass::RunnerInfrastructure);
                assert_eq!(delay_seconds, 5);
            }
            other => panic!("expected an environment backoff, got {other:?}"),
        }
        // No code-repair budget is consumed by an infrastructure failure.
        assert_eq!(loop_.total_attempts(), 0);

        match loop_.plan(&failure, 20).unwrap() {
            RepairDecision::EnvironmentBackoff { delay_seconds, .. } => {
                assert_eq!(delay_seconds, 10)
            }
            other => panic!("expected an environment backoff, got {other:?}"),
        }
    }

    #[test]
    fn budget_comes_from_configuration_and_is_validated() {
        let config = Config::parse(crate::config::DEFAULT_CONFIG).unwrap();
        let budget = RepairBudget::from_config(&config).unwrap();
        assert_eq!(budget.max_attempts_per_error, 3);
        assert_eq!(budget.max_total_attempts, 12);

        assert!(RepairBudget::new(0, 5).is_err());
        assert!(RepairBudget::new(5, 3).is_err());
        assert!(RepairLoop::new(budget, 0, 10).is_err());
        assert!(RepairLoop::new(budget, 10, 5).is_err());
    }

    #[test]
    fn every_repair_is_recorded_and_can_be_persisted() {
        let temp = tempfile::tempdir().unwrap();
        let connection = storage::open(&temp.path().join("state.db")).unwrap();
        storage::create_task(&connection, "task-4", 1, "I_4", 4).unwrap();

        let mut loop_ = loop_with(3, 12);
        let failure = ci_failure("task-4", RUSTC_LOG);
        assert!(matches!(
            loop_.plan(&failure, 50).unwrap(),
            RepairDecision::Repair { .. }
        ));
        assert_eq!(loop_.history().len(), 1);
        assert_eq!(loop_.history()[0].attempt_number, 1);
        assert_eq!(loop_.history()[0].ci_run_id, Some(38078659490));

        loop_
            .persist(&connection, &loop_.history()[0].clone())
            .unwrap();
        let stored: (String, i64) = connection
            .query_row(
                "SELECT fingerprint, attempt_number FROM repair_attempts WHERE task_id='task-4'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(stored.0, fingerprint(RUSTC_LOG));
        assert_eq!(stored.1, 1);

        // A task-less failure cannot be planned.
        assert!(loop_.plan(&ci_failure("  ", RUSTC_LOG), 60).is_err());
    }
}

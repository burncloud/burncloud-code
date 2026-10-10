#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Backlog,
    Ready,
    Running,
    WaitCi,
    FixCi,
    Review,
    ReadyForReview,
    AwaitingMerge,
    Merged,
    VerifyMain,
    Done,
    Blocked,
    Paused,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiState {
    Queued,
    InProgress,
    Success,
    Failure,
    Cancelled,
    TimedOut,
    Skipped,
    Missing,
}

impl CiState {
    pub const fn as_name(self) -> &'static str {
        match self {
            Self::Queued => "QUEUED",
            Self::InProgress => "IN_PROGRESS",
            Self::Success => "SUCCESS",
            Self::Failure => "FAILURE",
            Self::Cancelled => "CANCELLED",
            Self::TimedOut => "TIMED_OUT",
            Self::Skipped => "SKIPPED",
            Self::Missing => "MISSING",
        }
    }

    pub fn parse_name(name: &str) -> Option<Self> {
        match name {
            "QUEUED" => Some(Self::Queued),
            "IN_PROGRESS" => Some(Self::InProgress),
            "SUCCESS" => Some(Self::Success),
            "FAILURE" => Some(Self::Failure),
            "CANCELLED" => Some(Self::Cancelled),
            "TIMED_OUT" => Some(Self::TimedOut),
            "SKIPPED" => Some(Self::Skipped),
            "MISSING" => Some(Self::Missing),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    None,
    ReadyCriteria,
    LeaseGranted,
    RemotePushConfirmed,
    CurrentHeadCiFailure,
    CurrentHeadCiPass,
    AcceptancePass,
    ReviewPass,
    GitHubMergeConfirmed,
    MainVerificationPass,
    ExplicitResume,
    ExplicitPause,
    ExplicitCancel,
    ExternalBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionReason {
    Allowed,
    TerminalState,
    ReadyCriteriaRequired,
    LeaseRequired,
    RemotePushRequired,
    CurrentHeadCiRequired,
    AcceptanceRequired,
    ReviewRequired,
    MergeConfirmationRequired,
    MainVerificationRequired,
    ResumeRequired,
    PauseEvidenceRequired,
    BlockEvidenceRequired,
    CancelEvidenceRequired,
    InvalidTransition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionDecision {
    pub allowed: bool,
    pub reason: TransitionReason,
}

impl TransitionDecision {
    const fn allowed() -> Self {
        Self {
            allowed: true,
            reason: TransitionReason::Allowed,
        }
    }

    const fn denied(reason: TransitionReason) -> Self {
        Self {
            allowed: false,
            reason,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionError {
    pub reason: TransitionReason,
}

impl TaskState {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled)
    }

    pub fn decide_transition(self, next: Self, evidence: Evidence) -> TransitionDecision {
        use Evidence::*;
        use TaskState::*;

        if self.is_terminal() {
            return TransitionDecision::denied(TransitionReason::TerminalState);
        }

        let allowed = matches!(
            (self, next, evidence),
            (Backlog, Ready, ReadyCriteria)
                | (Ready, Running, LeaseGranted)
                | (Running, WaitCi, RemotePushConfirmed)
                | (WaitCi, FixCi, CurrentHeadCiFailure)
                | (FixCi, Running, LeaseGranted)
                | (WaitCi, Review, CurrentHeadCiPass)
                | (Review, ReadyForReview, AcceptancePass)
                | (ReadyForReview, AwaitingMerge, ReviewPass)
                | (AwaitingMerge, Merged, GitHubMergeConfirmed)
                | (Merged, VerifyMain, GitHubMergeConfirmed)
                | (VerifyMain, Done, MainVerificationPass)
                | (Blocked, Ready, ExplicitResume)
                | (Paused, Ready, ExplicitResume)
        ) || (next == Paused && evidence == ExplicitPause)
            || (next == Blocked && evidence == ExternalBlock)
            || (next == Cancelled && evidence == ExplicitCancel);

        if allowed {
            return TransitionDecision::allowed();
        }

        let reason = match (self, next) {
            (Backlog, Ready) => TransitionReason::ReadyCriteriaRequired,
            (Ready, Running) | (FixCi, Running) => TransitionReason::LeaseRequired,
            (Running, WaitCi) => TransitionReason::RemotePushRequired,
            (WaitCi, FixCi) | (WaitCi, Review) => TransitionReason::CurrentHeadCiRequired,
            (Review, ReadyForReview) => TransitionReason::AcceptanceRequired,
            (ReadyForReview, AwaitingMerge) => TransitionReason::ReviewRequired,
            (AwaitingMerge, Merged) | (Merged, VerifyMain) => {
                TransitionReason::MergeConfirmationRequired
            }
            (VerifyMain, Done) => TransitionReason::MainVerificationRequired,
            (Blocked, Ready) | (Paused, Ready) => TransitionReason::ResumeRequired,
            (_, Paused) => TransitionReason::PauseEvidenceRequired,
            (_, Blocked) => TransitionReason::BlockEvidenceRequired,
            (_, Cancelled) => TransitionReason::CancelEvidenceRequired,
            _ => TransitionReason::InvalidTransition,
        };
        TransitionDecision::denied(reason)
    }

    pub fn transition(self, next: Self, evidence: Evidence) -> Result<Self, TransitionError> {
        let decision = self.decide_transition(next, evidence);
        if decision.allowed {
            Ok(next)
        } else {
            Err(TransitionError {
                reason: decision.reason,
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspensionKind {
    Paused,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suspension {
    pub previous_state: TaskState,
    pub suspended_state: TaskState,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspensionError {
    TerminalState,
    EmptyReason,
}

impl TaskState {
    pub fn suspend(
        self,
        kind: SuspensionKind,
        reason: impl Into<String>,
    ) -> Result<Suspension, SuspensionError> {
        if self.is_terminal() {
            return Err(SuspensionError::TerminalState);
        }
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(SuspensionError::EmptyReason);
        }
        let suspended_state = match kind {
            SuspensionKind::Paused => Self::Paused,
            SuspensionKind::Blocked => Self::Blocked,
        };
        Ok(Suspension {
            previous_state: self,
            suspended_state,
            reason,
        })
    }
}

impl Suspension {
    pub const fn resume_target(&self) -> TaskState {
        TaskState::Ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_reaches_done_only_with_evidence() {
        use Evidence::*;
        use TaskState::*;
        let sequence = [
            (Ready, ReadyCriteria),
            (Running, LeaseGranted),
            (WaitCi, RemotePushConfirmed),
            (Review, CurrentHeadCiPass),
            (ReadyForReview, AcceptancePass),
            (AwaitingMerge, ReviewPass),
            (Merged, GitHubMergeConfirmed),
            (VerifyMain, GitHubMergeConfirmed),
            (Done, MainVerificationPass),
        ];
        let mut state = Backlog;
        for (next, evidence) in sequence {
            state = state.transition(next, evidence).unwrap();
        }
        assert_eq!(state, Done);
    }

    #[test]
    fn cannot_merge_without_authoritative_remote_evidence() {
        let decision =
            TaskState::AwaitingMerge.decide_transition(TaskState::Merged, Evidence::None);
        assert_eq!(decision.reason, TransitionReason::MergeConfirmationRequired);
        assert!(!decision.allowed);
    }

    #[test]
    fn cannot_claim_ci_success_without_evidence() {
        let decision = TaskState::WaitCi.decide_transition(TaskState::Review, Evidence::None);
        assert_eq!(decision.reason, TransitionReason::CurrentHeadCiRequired);
        assert!(!decision.allowed);
    }

    #[test]
    fn completed_tasks_are_terminal() {
        let decision =
            TaskState::Done.decide_transition(TaskState::Ready, Evidence::ExplicitResume);
        assert_eq!(decision.reason, TransitionReason::TerminalState);
        assert!(!decision.allowed);
    }

    #[test]
    fn suspension_preserves_previous_state_but_revalidates_on_resume() {
        let suspension = TaskState::WaitCi
            .suspend(SuspensionKind::Paused, "operator pause")
            .unwrap();
        assert_eq!(suspension.previous_state, TaskState::WaitCi);
        assert_eq!(suspension.suspended_state, TaskState::Paused);
        assert_eq!(suspension.resume_target(), TaskState::Ready);
    }
}

impl TaskState {
    pub const fn as_name(self) -> &'static str {
        match self {
            Self::Backlog => "BACKLOG",
            Self::Ready => "READY",
            Self::Running => "RUNNING",
            Self::WaitCi => "WAIT_CI",
            Self::FixCi => "FIX_CI",
            Self::Review => "REVIEW",
            Self::ReadyForReview => "READY_FOR_REVIEW",
            Self::AwaitingMerge => "AWAITING_MERGE",
            Self::Merged => "MERGED",
            Self::VerifyMain => "VERIFY_MAIN",
            Self::Done => "DONE",
            Self::Blocked => "BLOCKED",
            Self::Paused => "PAUSED",
            Self::Cancelled => "CANCELLED",
        }
    }

    pub fn parse_name(name: &str) -> Option<Self> {
        match name {
            "BACKLOG" => Some(Self::Backlog),
            "READY" => Some(Self::Ready),
            "RUNNING" => Some(Self::Running),
            "WAIT_CI" => Some(Self::WaitCi),
            "FIX_CI" => Some(Self::FixCi),
            "REVIEW" => Some(Self::Review),
            "READY_FOR_REVIEW" => Some(Self::ReadyForReview),
            "AWAITING_MERGE" => Some(Self::AwaitingMerge),
            "MERGED" => Some(Self::Merged),
            "VERIFY_MAIN" => Some(Self::VerifyMain),
            "DONE" => Some(Self::Done),
            "BLOCKED" => Some(Self::Blocked),
            "PAUSED" => Some(Self::Paused),
            "CANCELLED" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[cfg(test)]
mod persisted_state_tests {
    use super::*;

    #[test]
    fn all_state_names_round_trip() {
        for state in [
            TaskState::Backlog,
            TaskState::Ready,
            TaskState::Running,
            TaskState::WaitCi,
            TaskState::FixCi,
            TaskState::Review,
            TaskState::ReadyForReview,
            TaskState::AwaitingMerge,
            TaskState::Merged,
            TaskState::VerifyMain,
            TaskState::Done,
            TaskState::Blocked,
            TaskState::Paused,
            TaskState::Cancelled,
        ] {
            assert_eq!(TaskState::parse_name(state.as_name()), Some(state));
        }
        assert_eq!(TaskState::parse_name("unknown"), None);
    }

    #[test]
    fn all_ci_state_names_round_trip_without_cross_parsing() {
        for state in [
            CiState::Queued,
            CiState::InProgress,
            CiState::Success,
            CiState::Failure,
            CiState::Cancelled,
            CiState::TimedOut,
            CiState::Skipped,
            CiState::Missing,
        ] {
            assert_eq!(CiState::parse_name(state.as_name()), Some(state));
        }
        assert_eq!(CiState::parse_name("UNKNOWN"), None);
        assert_eq!(TaskState::parse_name(CiState::Success.as_name()), None);
    }
}

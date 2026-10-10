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
pub struct TransitionError;

impl TaskState {
    pub fn transition(self, next: Self, evidence: Evidence) -> Result<Self, TransitionError> {
        use Evidence::*;
        use TaskState::*;
        let valid = matches!(
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
        ) || (next == Paused
            && evidence == ExplicitPause
            && !matches!(self, Done | Cancelled))
            || (next == Blocked && evidence == ExternalBlock && !matches!(self, Done | Cancelled))
            || (next == Cancelled
                && evidence == ExplicitCancel
                && !matches!(self, Done | Cancelled));
        valid.then_some(next).ok_or(TransitionError)
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
        assert!(TaskState::AwaitingMerge
            .transition(TaskState::Merged, Evidence::None)
            .is_err());
    }
    #[test]
    fn cannot_claim_ci_success_without_evidence() {
        assert!(TaskState::WaitCi
            .transition(TaskState::Review, Evidence::None)
            .is_err());
    }
    #[test]
    fn completed_tasks_are_terminal() {
        assert!(TaskState::Done
            .transition(TaskState::Ready, Evidence::ExplicitResume)
            .is_err());
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
}

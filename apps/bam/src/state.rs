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
        assert!(TaskState::WaitCi.transition(TaskState::Review, Evidence::None).is_err());
    }
    #[test]
    fn completed_tasks_are_terminal() {
        assert!(TaskState::Done.transition(TaskState::Ready, Evidence::ExplicitResume).is_err());
    }
}

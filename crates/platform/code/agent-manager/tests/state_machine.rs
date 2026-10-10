use bam::state::{CiState, Evidence, SuspensionKind, TaskState, TransitionReason};

#[test]
fn ci_state_is_independent_from_task_state() {
    assert_eq!(CiState::parse_name("SUCCESS"), Some(CiState::Success));
    assert_eq!(TaskState::parse_name("SUCCESS"), None);
    assert_eq!(CiState::parse_name("WAIT_CI"), None);
}

#[test]
fn evidence_guards_match_the_documented_lifecycle() {
    assert_eq!(
        TaskState::WaitCi
            .decide_transition(TaskState::Review, Evidence::None)
            .reason,
        TransitionReason::CurrentHeadCiRequired
    );
    assert_eq!(
        TaskState::AwaitingMerge
            .decide_transition(TaskState::Merged, Evidence::None)
            .reason,
        TransitionReason::MergeConfirmationRequired
    );
    assert_eq!(
        TaskState::VerifyMain
            .decide_transition(TaskState::Done, Evidence::None)
            .reason,
        TransitionReason::MainVerificationRequired
    );
}

#[test]
fn terminal_state_replay_cannot_revive_task() {
    for state in [TaskState::Done, TaskState::Cancelled] {
        let decision = state.decide_transition(TaskState::Ready, Evidence::ExplicitResume);
        assert!(!decision.allowed);
        assert_eq!(decision.reason, TransitionReason::TerminalState);
    }
}

#[test]
fn paused_work_preserves_origin_and_resumes_through_ready() {
    let suspension = TaskState::Running
        .suspend(SuspensionKind::Paused, "manual maintenance")
        .unwrap();
    assert_eq!(suspension.previous_state, TaskState::Running);
    assert_eq!(suspension.suspended_state, TaskState::Paused);
    assert_eq!(suspension.resume_target(), TaskState::Ready);
}

use bam::{
    scheduler::{
        schedule, Priority, ScheduleBudget, ScheduleError, TaskNode, WorkKind, MAX_AGENTS,
    },
    state::TaskState,
};

fn task(id: u64, state: TaskState) -> TaskNode {
    TaskNode {
        id,
        dependencies: Vec::new(),
        state,
        priority: Priority::P2,
        kind: WorkKind::NewIssue,
        wait_cycles: 0,
    }
}

#[test]
fn missing_dependency_fails_closed() {
    let mut candidate = task(10, TaskState::Ready);
    candidate.dependencies.push(999);
    assert_eq!(
        schedule(vec![candidate], ScheduleBudget::default()),
        Err(ScheduleError::MissingDependency {
            task: 10,
            missing: 999
        })
    );
}

#[test]
fn budget_is_bounded_to_one_through_eight() {
    assert!(schedule(
        vec![task(1, TaskState::Ready)],
        ScheduleBudget {
            max_agents: 0,
            active_agents: 0,
        },
    )
    .is_err());
    assert!(schedule(
        vec![task(1, TaskState::Ready)],
        ScheduleBudget {
            max_agents: MAX_AGENTS + 1,
            active_agents: 0,
        },
    )
    .is_err());
}

#[test]
fn blocked_task_does_not_stop_independent_ready_work() {
    let blocked = task(1, TaskState::Blocked);
    let ready = task(2, TaskState::Ready);
    let decision = schedule(vec![blocked, ready], ScheduleBudget::default()).unwrap();
    assert_eq!(decision.selected, vec![2]);
}

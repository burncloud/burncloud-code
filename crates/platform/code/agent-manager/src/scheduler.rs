use crate::state::TaskState;
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
};

pub const DEFAULT_MAX_AGENTS: u8 = 2;
pub const MAX_AGENTS: u8 = 8;
pub const STARVATION_AGE: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
}

impl Priority {
    const fn rank(self) -> u8 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkKind {
    CiRepair,
    ConflictResolution,
    ExistingPullRequest,
    NewIssue,
}

impl WorkKind {
    const fn rank(self) -> u8 {
        match self {
            Self::CiRepair => 0,
            Self::ConflictResolution => 1,
            Self::ExistingPullRequest => 2,
            Self::NewIssue => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskNode {
    pub id: u64,
    pub dependencies: Vec<u64>,
    pub state: TaskState,
    pub priority: Priority,
    pub kind: WorkKind,
    pub wait_cycles: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleBudget {
    pub max_agents: u8,
    pub active_agents: u8,
}

impl Default for ScheduleBudget {
    fn default() -> Self {
        Self {
            max_agents: DEFAULT_MAX_AGENTS,
            active_agents: 0,
        }
    }
}

impl ScheduleBudget {
    pub fn validate(self) -> Result<Self, ScheduleError> {
        if !(1..=MAX_AGENTS).contains(&self.max_agents) || self.active_agents > self.max_agents {
            return Err(ScheduleError::InvalidBudget {
                max_agents: self.max_agents,
                active_agents: self.active_agents,
            });
        }
        Ok(self)
    }

    pub const fn free_slots(self) -> u8 {
        self.max_agents - self.active_agents
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleDecision {
    pub selected: Vec<u64>,
    pub free_slots: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    DuplicateTask(u64),
    MissingDependency { task: u64, missing: u64 },
    Cycle { path: Vec<u64> },
    InvalidBudget { max_agents: u8, active_agents: u8 },
}

#[derive(Debug, Clone)]
pub struct TaskGraph {
    tasks: HashMap<u64, TaskNode>,
}

impl TaskGraph {
    pub fn new(tasks: Vec<TaskNode>) -> Result<Self, ScheduleError> {
        let mut by_id = HashMap::with_capacity(tasks.len());
        for task in tasks {
            let id = task.id;
            if by_id.insert(id, task).is_some() {
                return Err(ScheduleError::DuplicateTask(id));
            }
        }

        let graph = Self { tasks: by_id };
        graph.validate_dependencies()?;
        if let Some(path) = graph.cycle_path() {
            return Err(ScheduleError::Cycle { path });
        }
        Ok(graph)
    }

    fn validate_dependencies(&self) -> Result<(), ScheduleError> {
        for task in self.tasks.values() {
            for dependency in &task.dependencies {
                if !self.tasks.contains_key(dependency) {
                    return Err(ScheduleError::MissingDependency {
                        task: task.id,
                        missing: *dependency,
                    });
                }
            }
        }
        Ok(())
    }

    pub fn dependencies_done(&self, task_id: u64) -> bool {
        let Some(task) = self.tasks.get(&task_id) else {
            return false;
        };
        task.dependencies.iter().all(|dependency| {
            self.tasks
                .get(dependency)
                .is_some_and(|task| task.state == TaskState::Done)
        })
    }

    pub fn cycle_path(&self) -> Option<Vec<u64>> {
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        let mut ids: Vec<_> = self.tasks.keys().copied().collect();
        ids.sort_unstable();

        for id in ids {
            if let Some(path) = self.visit(id, &mut visiting, &mut visited, &mut stack) {
                return Some(path);
            }
        }
        None
    }

    fn visit(
        &self,
        id: u64,
        visiting: &mut HashSet<u64>,
        visited: &mut HashSet<u64>,
        stack: &mut Vec<u64>,
    ) -> Option<Vec<u64>> {
        if visited.contains(&id) {
            return None;
        }
        if visiting.contains(&id) {
            let start = stack.iter().position(|candidate| *candidate == id)?;
            let mut path = stack[start..].to_vec();
            path.push(id);
            return Some(path);
        }

        visiting.insert(id);
        stack.push(id);

        let dependencies = self
            .tasks
            .get(&id)
            .map(|task| task.dependencies.clone())
            .unwrap_or_default();
        for dependency in dependencies {
            if let Some(path) = self.visit(dependency, visiting, visited, stack) {
                return Some(path);
            }
        }

        stack.pop();
        visiting.remove(&id);
        visited.insert(id);
        None
    }

    fn candidates(&self) -> Vec<&TaskNode> {
        self.tasks
            .values()
            .filter(|task| matches!(task.state, TaskState::Ready | TaskState::FixCi))
            .filter(|task| self.dependencies_done(task.id))
            .collect()
    }
}

fn scheduling_key(task: &TaskNode) -> (u8, u8, Reverse<u32>, u64) {
    if task.wait_cycles >= STARVATION_AGE {
        return (0, 0, Reverse(task.wait_cycles), task.id);
    }

    (
        task.kind.rank(),
        task.priority.rank(),
        Reverse(task.wait_cycles),
        task.id,
    )
}

pub fn schedule(
    tasks: Vec<TaskNode>,
    budget: ScheduleBudget,
) -> Result<ScheduleDecision, ScheduleError> {
    let budget = budget.validate()?;
    let graph = TaskGraph::new(tasks)?;
    let free_slots = budget.free_slots();

    if free_slots == 0 {
        return Ok(ScheduleDecision {
            selected: Vec::new(),
            free_slots,
        });
    }

    let mut candidates = graph.candidates();
    candidates.sort_by_key(|task| scheduling_key(task));

    Ok(ScheduleDecision {
        selected: candidates
            .into_iter()
            .take(usize::from(free_slots))
            .map(|task| task.id)
            .collect(),
        free_slots,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: u64, state: TaskState, kind: WorkKind) -> TaskNode {
        TaskNode {
            id,
            dependencies: Vec::new(),
            state,
            priority: Priority::P1,
            kind,
            wait_cycles: 0,
        }
    }

    #[test]
    fn cycle_reports_concrete_path() {
        let mut one = task(1, TaskState::Ready, WorkKind::NewIssue);
        one.dependencies = vec![2];
        let mut two = task(2, TaskState::Ready, WorkKind::NewIssue);
        two.dependencies = vec![3];
        let mut three = task(3, TaskState::Ready, WorkKind::NewIssue);
        three.dependencies = vec![1];

        let error = TaskGraph::new(vec![one, two, three]).unwrap_err();
        assert_eq!(
            error,
            ScheduleError::Cycle {
                path: vec![1, 2, 3, 1]
            }
        );
    }

    #[test]
    fn only_done_dependencies_unlock_work() {
        let dependency = task(1, TaskState::WaitCi, WorkKind::ExistingPullRequest);
        let mut dependent = task(2, TaskState::Ready, WorkKind::NewIssue);
        dependent.dependencies = vec![1];

        let decision = schedule(vec![dependency, dependent], ScheduleBudget::default()).unwrap();
        assert!(decision.selected.is_empty());

        let dependency = task(1, TaskState::Done, WorkKind::ExistingPullRequest);
        let mut dependent = task(2, TaskState::Ready, WorkKind::NewIssue);
        dependent.dependencies = vec![1];
        let decision = schedule(vec![dependency, dependent], ScheduleBudget::default()).unwrap();
        assert_eq!(decision.selected, vec![2]);
    }

    #[test]
    fn repair_priority_and_wait_ci_slot_release_are_deterministic() {
        let wait = task(1, TaskState::WaitCi, WorkKind::ExistingPullRequest);
        let new_issue = task(2, TaskState::Ready, WorkKind::NewIssue);
        let repair = task(3, TaskState::FixCi, WorkKind::CiRepair);

        let decision = schedule(
            vec![wait, new_issue, repair],
            ScheduleBudget {
                max_agents: 2,
                active_agents: 0,
            },
        )
        .unwrap();

        assert_eq!(decision.selected, vec![3, 2]);
    }

    #[test]
    fn aging_prevents_permanent_starvation() {
        let repair = task(1, TaskState::FixCi, WorkKind::CiRepair);
        let mut old_low_priority = task(2, TaskState::Ready, WorkKind::NewIssue);
        old_low_priority.priority = Priority::P3;
        old_low_priority.wait_cycles = STARVATION_AGE + 5;

        let decision = schedule(
            vec![repair, old_low_priority],
            ScheduleBudget {
                max_agents: 1,
                active_agents: 0,
            },
        )
        .unwrap();

        assert_eq!(decision.selected, vec![2]);
    }
}

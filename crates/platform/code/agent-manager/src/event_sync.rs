//! Event synchronisation: polling schedule, normalisation, ordering, de-duplication
//! and safe backoff.
//!
//! Requirements: M15-01 (scheduled polling), M15-03..M15-07 (issue / pull request /
//! check / review / project item monitoring), M15-08 (duplicate de-duplication),
//! M15-09 (periodic full reconciliation), M15-10 (safe backoff when GitHub is
//! unavailable), M15-11 (API call rate control); design M15-A, M15-C, M15-D, M15-E,
//! M15-F.
//!
//! Webhook intake (M15-02 / M15-B) is deliberately not implemented here: verifying a
//! webhook signature requires an approved cryptographic dependency, which is a manifest
//! and dependency-policy change of its own.
use crate::{config::Config, storage};
use rusqlite::Connection;
use std::collections::BTreeSet;

/// Default number of polls between two full reconciliations.
pub const DEFAULT_FULL_RECONCILE_EVERY: u32 = 10;

/// Highest exponent used when computing the exponential backoff delay.
const MAX_BACKOFF_EXPONENT: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSource {
    Issue,
    PullRequest,
    Check,
    Review,
    ProjectItem,
}

impl EventSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Issue => "issue",
            Self::PullRequest => "pull_request",
            Self::Check => "check",
            Self::Review => "review",
            Self::ProjectItem => "project_item",
        }
    }
}

/// Provider-independent event kind. Only kinds that are explicitly mapped exist; an
/// unrecognised action is reported as unsupported instead of being guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NormalisedKind {
    Opened,
    Closed,
    Reopened,
    Edited,
    Labeled,
    Unlabeled,
    Assigned,
    Unassigned,
    Synchronized,
    ReadyForReview,
    ConvertedToDraft,
    ReviewRequested,
    ReviewSubmitted,
    CheckCompleted,
    FieldChanged,
}

impl NormalisedKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Opened => "opened",
            Self::Closed => "closed",
            Self::Reopened => "reopened",
            Self::Edited => "edited",
            Self::Labeled => "labeled",
            Self::Unlabeled => "unlabeled",
            Self::Assigned => "assigned",
            Self::Unassigned => "unassigned",
            Self::Synchronized => "synchronized",
            Self::ReadyForReview => "ready_for_review",
            Self::ConvertedToDraft => "converted_to_draft",
            Self::ReviewRequested => "review_requested",
            Self::ReviewSubmitted => "review_submitted",
            Self::CheckCompleted => "check_completed",
            Self::FieldChanged => "field_changed",
        }
    }
}

/// Normalise a provider action into the shared vocabulary (M15-C).
///
/// A combination that is not explicitly known returns `None`, so an unexpected or
/// hostile action can never be silently mapped onto a state change we do act upon.
pub fn normalise(source: EventSource, raw_action: &str) -> Option<NormalisedKind> {
    let action = raw_action.trim().to_ascii_lowercase();
    let kind = match (source, action.as_str()) {
        (EventSource::Issue, "opened") | (EventSource::PullRequest, "opened") => {
            NormalisedKind::Opened
        }
        (EventSource::Issue, "closed") | (EventSource::PullRequest, "closed") => {
            NormalisedKind::Closed
        }
        (EventSource::Issue, "reopened") | (EventSource::PullRequest, "reopened") => {
            NormalisedKind::Reopened
        }
        (EventSource::Issue, "edited")
        | (EventSource::PullRequest, "edited")
        | (EventSource::Review, "edited") => NormalisedKind::Edited,
        (EventSource::Issue, "labeled") => NormalisedKind::Labeled,
        (EventSource::Issue, "unlabeled") => NormalisedKind::Unlabeled,
        (EventSource::Issue, "assigned") => NormalisedKind::Assigned,
        (EventSource::Issue, "unassigned") => NormalisedKind::Unassigned,
        (EventSource::PullRequest, "synchronize") | (EventSource::PullRequest, "synchronized") => {
            NormalisedKind::Synchronized
        }
        (EventSource::PullRequest, "ready_for_review") => NormalisedKind::ReadyForReview,
        (EventSource::PullRequest, "converted_to_draft") => NormalisedKind::ConvertedToDraft,
        (EventSource::PullRequest, "review_requested") => NormalisedKind::ReviewRequested,
        (EventSource::Review, "submitted") => NormalisedKind::ReviewSubmitted,
        (EventSource::Check, "completed") => NormalisedKind::CheckCompleted,
        (EventSource::ProjectItem, "field_changed") | (EventSource::ProjectItem, "edited") => {
            NormalisedKind::FieldChanged
        }
        _ => return None,
    };
    Some(kind)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncEvent {
    pub source: EventSource,
    pub kind: NormalisedKind,
    pub resource_key: String,
    pub occurred_at: i64,
    pub sequence: u64,
}

impl SyncEvent {
    /// De-duplication identity: the same provider change delivered twice must collapse.
    pub fn dedup_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.source.as_str(),
            self.resource_key,
            self.kind.as_str()
        )
    }

    /// Durable inbox key, so de-duplication survives a restart.
    pub fn inbox_key(&self) -> String {
        self.dedup_key()
    }
}

/// In-memory de-duplicating, order-preserving queue (M15-D, M15-08).
///
/// Ordering is by occurrence time and then by arrival, so replay is deterministic even
/// when events arrive out of order.
#[derive(Debug, Default)]
pub struct EventQueue {
    pending: Vec<SyncEvent>,
    seen: BTreeSet<String>,
    next_sequence: u64,
}

impl EventQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns false when the event was already queued (duplicate delivery).
    pub fn push(
        &mut self,
        source: EventSource,
        kind: NormalisedKind,
        resource_key: &str,
        occurred_at: i64,
    ) -> bool {
        let event = SyncEvent {
            source,
            kind,
            resource_key: resource_key.to_owned(),
            occurred_at,
            sequence: self.next_sequence,
        };
        if !self.seen.insert(event.dedup_key()) {
            return false;
        }
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.pending.push(event);
        true
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn seen(&self) -> usize {
        self.seen.len()
    }

    /// Take up to `limit` events in deterministic order.
    pub fn drain_ready(&mut self, limit: usize) -> Vec<SyncEvent> {
        self.pending.sort_by(|left, right| {
            (left.occurred_at, left.sequence).cmp(&(right.occurred_at, right.sequence))
        });
        let take = limit.min(self.pending.len());
        self.pending.drain(..take).collect()
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub accepted: Vec<SyncEvent>,
    pub duplicates: u64,
}

/// Persist pending events through the durable inbox, so a delivery seen twice - including
/// across a restart - is only applied once (M15-08).
pub fn apply_pending(
    connection: &Connection,
    queue: &mut EventQueue,
    limit: usize,
) -> Result<Applied, String> {
    let mut applied = Applied::default();
    for event in queue.drain_ready(limit) {
        if storage::record_inbox_event(connection, &event.inbox_key())? {
            applied.accepted.push(event);
        } else {
            applied.duplicates += 1;
        }
    }
    Ok(applied)
}

/// Polling schedule (M15-A, M15-01, M15-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PollSchedule {
    interval_seconds: u64,
    full_reconcile_every: u32,
}

impl PollSchedule {
    pub fn new(interval_seconds: u64, full_reconcile_every: u32) -> Result<Self, String> {
        if interval_seconds == 0 {
            return Err("poll interval must be positive".into());
        }
        if full_reconcile_every == 0 {
            return Err("full reconciliation cadence must be positive".into());
        }
        Ok(Self {
            interval_seconds,
            full_reconcile_every,
        })
    }

    pub fn from_config(config: &Config) -> Result<Self, String> {
        Self::new(
            config.manager.poll_interval_seconds,
            DEFAULT_FULL_RECONCILE_EVERY,
        )
    }

    pub fn interval_seconds(&self) -> u64 {
        self.interval_seconds
    }

    pub fn full_reconcile_every(&self) -> u32 {
        self.full_reconcile_every
    }

    pub fn next_poll_at(&self, last_poll_at: i64) -> i64 {
        let interval = i64::try_from(self.interval_seconds).unwrap_or(i64::MAX);
        last_poll_at.saturating_add(interval)
    }

    pub fn is_due(&self, last_poll_at: i64, now: i64) -> bool {
        now >= self.next_poll_at(last_poll_at)
    }

    /// A full reconciliation replaces incremental polling every N completed polls.
    pub fn requires_full_reconcile(&self, completed_polls: u64) -> bool {
        completed_polls > 0 && completed_polls.is_multiple_of(u64::from(self.full_reconcile_every))
    }
}

/// Exponential backoff with a hard ceiling (M15-F, M15-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backoff {
    base_seconds: i64,
    max_seconds: i64,
    failures: u32,
}

impl Backoff {
    pub fn new(base_seconds: i64, max_seconds: i64) -> Result<Self, String> {
        if base_seconds <= 0 || max_seconds < base_seconds {
            return Err("backoff requires 0 < base <= max".into());
        }
        Ok(Self {
            base_seconds,
            max_seconds,
            failures: 0,
        })
    }

    pub fn failures(&self) -> u32 {
        self.failures
    }

    pub fn delay_seconds(&self) -> i64 {
        if self.failures == 0 {
            return 0;
        }
        let exponent = self.failures.saturating_sub(1).min(MAX_BACKOFF_EXPONENT);
        let multiplier = 1_i64.checked_shl(exponent).unwrap_or(i64::MAX);
        self.base_seconds
            .saturating_mul(multiplier)
            .min(self.max_seconds)
    }

    /// Record an unavailable source and return the delay to wait before retrying.
    pub fn record_failure(&mut self) -> i64 {
        self.failures = self.failures.saturating_add(1);
        self.delay_seconds()
    }

    /// A successful call clears the accumulated penalty.
    pub fn record_success(&mut self) {
        self.failures = 0;
    }
}

/// Minimum spacing between GitHub calls (M15-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimit {
    min_interval_seconds: i64,
    last_call_at: Option<i64>,
}

impl RateLimit {
    pub fn new(min_interval_seconds: i64) -> Result<Self, String> {
        if min_interval_seconds <= 0 {
            return Err("rate limit interval must be positive".into());
        }
        Ok(Self {
            min_interval_seconds,
            last_call_at: None,
        })
    }

    pub fn min_interval_seconds(&self) -> i64 {
        self.min_interval_seconds
    }

    pub fn wait_seconds(&self, now: i64) -> i64 {
        match self.last_call_at {
            None => 0,
            Some(last) => last
                .saturating_add(self.min_interval_seconds)
                .saturating_sub(now)
                .max(0),
        }
    }

    pub fn allows(&self, now: i64) -> bool {
        self.wait_seconds(now) == 0
    }

    pub fn record_call(&mut self, now: i64) {
        self.last_call_at = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_actions_are_normalised_and_unknown_ones_are_rejected() {
        assert_eq!(
            normalise(EventSource::Issue, "opened"),
            Some(NormalisedKind::Opened)
        );
        assert_eq!(
            normalise(EventSource::Issue, " LABELED "),
            Some(NormalisedKind::Labeled)
        );
        assert_eq!(
            normalise(EventSource::PullRequest, "synchronize"),
            Some(NormalisedKind::Synchronized)
        );
        assert_eq!(
            normalise(EventSource::Review, "submitted"),
            Some(NormalisedKind::ReviewSubmitted)
        );
        assert_eq!(
            normalise(EventSource::Check, "completed"),
            Some(NormalisedKind::CheckCompleted)
        );
        assert_eq!(
            normalise(EventSource::ProjectItem, "field_changed"),
            Some(NormalisedKind::FieldChanged)
        );

        // Unknown action, and a real action applied to the wrong source, are refused.
        assert_eq!(
            normalise(EventSource::Issue, "definitely_not_an_action"),
            None
        );
        assert_eq!(normalise(EventSource::Issue, "synchronize"), None);
        assert_eq!(normalise(EventSource::Check, "opened"), None);
        assert_eq!(normalise(EventSource::Review, "labeled"), None);
    }

    #[test]
    fn duplicate_deliveries_collapse_and_order_is_deterministic() {
        let mut queue = EventQueue::new();
        assert!(queue.push(EventSource::Issue, NormalisedKind::Opened, "issue:24", 300));
        assert!(!queue.push(EventSource::Issue, NormalisedKind::Opened, "issue:24", 300));
        assert!(queue.push(
            EventSource::Check,
            NormalisedKind::CheckCompleted,
            "check:cargo-test",
            100
        ));
        assert!(queue.push(
            EventSource::PullRequest,
            NormalisedKind::Synchronized,
            "pr:29",
            200
        ));
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.seen(), 3);
        assert!(!queue.is_empty());

        let drained = queue.drain_ready(10);
        assert_eq!(
            drained
                .iter()
                .map(|event| event.occurred_at)
                .collect::<Vec<_>>(),
            vec![100, 200, 300]
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn draining_respects_the_batch_limit() {
        let mut queue = EventQueue::new();
        for index in 0..5 {
            assert!(queue.push(
                EventSource::Issue,
                NormalisedKind::Edited,
                &format!("issue:{index}"),
                i64::from(index)
            ));
        }
        assert_eq!(queue.drain_ready(2).len(), 2);
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.drain_ready(10).len(), 3);
    }

    #[test]
    fn durable_inbox_prevents_reapplying_a_delivery_across_restarts() {
        let temp = tempfile::tempdir().unwrap();
        let connection = storage::open(&temp.path().join("state.db")).unwrap();

        let mut queue = EventQueue::new();
        queue.push(EventSource::Issue, NormalisedKind::Opened, "issue:24", 10);
        let first = apply_pending(&connection, &mut queue, 10).unwrap();
        assert_eq!(first.accepted.len(), 1);
        assert_eq!(first.duplicates, 0);

        // A fresh process re-delivers the very same event.
        let mut replay = EventQueue::new();
        replay.push(EventSource::Issue, NormalisedKind::Opened, "issue:24", 10);
        let second = apply_pending(&connection, &mut replay, 10).unwrap();
        assert!(second.accepted.is_empty());
        assert_eq!(second.duplicates, 1);
    }

    #[test]
    fn poll_schedule_comes_from_configuration_and_detects_due_time() {
        let config = Config::parse(crate::config::DEFAULT_CONFIG).unwrap();
        let schedule = PollSchedule::from_config(&config).unwrap();
        assert_eq!(schedule.interval_seconds(), 60);
        assert_eq!(
            schedule.full_reconcile_every(),
            DEFAULT_FULL_RECONCILE_EVERY
        );

        let last = 1_000;
        assert_eq!(schedule.next_poll_at(last), 1_060);
        assert!(!schedule.is_due(last, 1_059));
        assert!(schedule.is_due(last, 1_060));
        assert!(schedule.is_due(last, 2_000));

        assert!(PollSchedule::new(0, 10).is_err());
        assert!(PollSchedule::new(60, 0).is_err());
    }

    #[test]
    fn full_reconciliation_runs_on_its_own_cadence() {
        let schedule = PollSchedule::new(30, 4).unwrap();
        assert!(!schedule.requires_full_reconcile(0));
        assert!(!schedule.requires_full_reconcile(3));
        assert!(schedule.requires_full_reconcile(4));
        assert!(schedule.requires_full_reconcile(8));
        assert!(!schedule.requires_full_reconcile(9));
    }

    #[test]
    fn backoff_grows_caps_and_resets() {
        let mut backoff = Backoff::new(5, 60).unwrap();
        assert_eq!(backoff.delay_seconds(), 0);
        assert_eq!(backoff.record_failure(), 5);
        assert_eq!(backoff.record_failure(), 10);
        assert_eq!(backoff.record_failure(), 20);
        assert_eq!(backoff.record_failure(), 40);
        assert_eq!(backoff.record_failure(), 60);
        assert_eq!(backoff.record_failure(), 60);
        assert_eq!(backoff.failures(), 6);

        backoff.record_success();
        assert_eq!(backoff.failures(), 0);
        assert_eq!(backoff.delay_seconds(), 0);
        assert_eq!(backoff.record_failure(), 5);

        assert!(Backoff::new(0, 10).is_err());
        assert!(Backoff::new(10, 5).is_err());

        // A long outage must not overflow into a negative delay.
        let mut long = Backoff::new(1, 3_600).unwrap();
        for _ in 0..200 {
            let delay = long.record_failure();
            assert!((1..=3_600).contains(&delay));
        }
    }

    #[test]
    fn api_call_rate_is_spaced_out() {
        let mut limit = RateLimit::new(2).unwrap();
        assert_eq!(limit.min_interval_seconds(), 2);
        assert!(limit.allows(100));
        limit.record_call(100);
        assert!(!limit.allows(100));
        assert_eq!(limit.wait_seconds(100), 2);
        assert_eq!(limit.wait_seconds(101), 1);
        assert!(limit.allows(102));
        limit.record_call(102);
        assert!(limit.allows(200));
        assert!(RateLimit::new(0).is_err());
    }

    #[test]
    fn event_keys_are_stable_and_source_scoped() {
        let issue = SyncEvent {
            source: EventSource::Issue,
            kind: NormalisedKind::Opened,
            resource_key: "issue:24".to_owned(),
            occurred_at: 1,
            sequence: 0,
        };
        assert_eq!(issue.dedup_key(), "issue:issue:24:opened");
        assert_eq!(issue.dedup_key(), issue.inbox_key());

        let check = SyncEvent {
            source: EventSource::Check,
            kind: NormalisedKind::Opened,
            resource_key: "issue:24".to_owned(),
            occurred_at: 1,
            sequence: 1,
        };
        assert_ne!(issue.dedup_key(), check.dedup_key());
    }
}

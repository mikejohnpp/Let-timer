use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use let_timer_core::{Database, Notifier, TaskService, TaskStatus};
use rusqlite::params;
use tokio::sync::Notify;

/// Background scheduler responsible for firing "time's up" notifications.
pub struct Scheduler {
    db: Arc<Mutex<Database>>,
    notifier: Arc<dyn Notifier>,
    notify: Arc<Notify>,
    clock: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
    events: Option<Arc<let_timer_core::EventBus>>,
}

impl Scheduler {
    pub fn new(db: Arc<Mutex<Database>>, notifier: Arc<dyn Notifier>, notify: Arc<Notify>) -> Self {
        Self {
            db,
            notifier,
            notify,
            clock: Arc::new(Utc::now),
            events: None,
        }
    }

    pub fn with_clock(
        db: Arc<Mutex<Database>>,
        notifier: Arc<dyn Notifier>,
        notify: Arc<Notify>,
        clock: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
    ) -> Self {
        Self {
            db,
            notifier,
            notify,
            clock,
            events: None,
        }
    }

    pub fn with_events(mut self, events: Arc<let_timer_core::EventBus>) -> Self {
        self.events = Some(events);
        self
    }

    /// Evaluates the running task at instant `now`.
    ///
    /// If the running task has an estimate, is overtime, and has not yet been notified
    /// for the current run, fires the notification and records `notified_at` in the DB.
    ///
    /// Returns the recommended sleep duration before the next evaluation (capped at 30 seconds).
    pub fn tick(&self, now: DateTime<Utc>) -> Result<Option<Duration>, String> {
        let db_guard = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let conn = db_guard.conn();
        let service = TaskService::new(conn);

        let snapshot = match service.current(now) {
            Ok(Some(snap)) => snap,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };

        // Only in-progress tasks with an estimated duration are scheduled
        if snapshot.task.status != TaskStatus::InProgress {
            return Ok(None);
        }

        let Some(est_mins) = snapshot.task.estimated_mins else {
            return Ok(None);
        };

        let rem_secs = snapshot.remaining_secs.unwrap_or(0);

        // Time's up: fire notification once per run
        if rem_secs <= 0 && snapshot.task.notified_at.is_none() {
            let title = "Time's up";
            let body = format!("{} ({} mins)", snapshot.task.name, est_mins);
            self.notifier.notify(title, &body);

            if let Some(bus) = &self.events {
                bus.publish(let_timer_core::Event::TimerExpired {
                    task: snapshot.task.clone(),
                    estimate_mins: est_mins,
                });
            }

            let _ = conn.execute(
                "UPDATE tasks SET notified_at = ?1 WHERE id = ?2",
                params![now.to_rfc3339(), snapshot.task.id],
            );

            // Re-check periodically while overtime (e.g. 30s)
            return Ok(Some(Duration::from_secs(30)));
        }

        if snapshot.task.notified_at.is_some() {
            // Already notified for this run; wait for state change or 30s periodic check
            return Ok(Some(Duration::from_secs(30)));
        }

        // Before deadline: sleep for min(remaining, 30s)
        let sleep_secs = (rem_secs as u64).clamp(1, 30);
        Ok(Some(Duration::from_secs(sleep_secs)))
    }

    /// Run the scheduler loop until cancelled.
    pub async fn run(self) {
        loop {
            let now = (self.clock)();
            let sleep_dur = match self.tick(now) {
                Ok(Some(dur)) => dur,
                Ok(None) => Duration::from_secs(30),
                Err(e) => {
                    tracing::error!("scheduler tick error: {e}");
                    Duration::from_secs(30)
                }
            };

            tokio::select! {
                _ = tokio::time::sleep(sleep_dur) => {},
                _ = self.notify.notified() => {},
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use let_timer_core::{NewTask, Priority, RecordingNotifier, UpdateTask};

    #[allow(clippy::type_complexity)]
    fn setup_scheduler() -> (
        Arc<Mutex<Database>>,
        Arc<RecordingNotifier>,
        Arc<Notify>,
        Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>,
        Scheduler,
    ) {
        let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let notifier = Arc::new(RecordingNotifier::new());
        let notify = Arc::new(Notify::new());
        let tokio_start = tokio::time::Instant::now();
        let test_start = Utc::now();
        let clock: Arc<dyn Fn() -> DateTime<Utc> + Send + Sync> = Arc::new(move || {
            let elapsed = tokio::time::Instant::now().duration_since(tokio_start);
            test_start + chrono::Duration::from_std(elapsed).unwrap_or_default()
        });
        let scheduler = Scheduler::with_clock(
            Arc::clone(&db),
            notifier.clone() as Arc<dyn Notifier>,
            Arc::clone(&notify),
            Arc::clone(&clock),
        );
        (db, notifier, notify, clock, scheduler)
    }

    fn create_and_start_task(
        db: &Arc<Mutex<Database>>,
        name: &str,
        estimate: Option<i64>,
        now: DateTime<Utc>,
    ) -> i64 {
        let db_guard = db.lock().unwrap();
        let conn = db_guard.conn();
        let repo = let_timer_core::TaskRepository::new(conn);
        let task = repo
            .create(&NewTask {
                workspace_id: 1,
                media_list_id: None,
                name: name.to_string(),
                description: None,
                priority: Priority::NotYet,
                estimated_mins: estimate,
                scheduled_on: None,
            })
            .unwrap();

        let service = TaskService::new(conn);
        service.start(task.id, false, now).unwrap();
        task.id
    }

    #[tokio::test]
    async fn fires_once_at_the_deadline() {
        tokio::time::pause();
        let (db, notifier, notify, clock, scheduler) = setup_scheduler();

        let handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        let start_time = clock();
        let task_id = create_and_start_task(&db, "Write report", Some(1), start_time); // 1 min = 60s
        notify.notify_one();

        // 30s elapsed: no notification
        tokio::time::advance(Duration::from_secs(30)).await;
        tokio::task::yield_now().await;
        assert_eq!(notifier.count(), 0);

        // 31s more (total 61s): deadline passed, notification fired
        tokio::time::advance(Duration::from_secs(31)).await;
        tokio::task::yield_now().await;
        assert_eq!(notifier.count(), 1);
        assert_eq!(
            notifier.notifications(),
            vec![("Time's up".to_string(), "Write report (1 mins)".to_string())]
        );

        // Verify task status is still InProgress (never automatically completed)
        {
            let db_guard = db.lock().unwrap();
            let repo = let_timer_core::TaskRepository::new(db_guard.conn());
            let task = repo.get_by_id(task_id).unwrap();
            assert_eq!(task.status, TaskStatus::InProgress);
            assert!(task.notified_at.is_some());
        }

        // Another 60s elapsed: not fired again!
        tokio::time::advance(Duration::from_secs(60)).await;
        tokio::task::yield_now().await;
        assert_eq!(notifier.count(), 1);

        handle.abort();
    }

    #[tokio::test]
    async fn pause_before_deadline_does_not_fire() {
        tokio::time::pause();
        let (db, notifier, notify, clock, scheduler) = setup_scheduler();

        let handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        let start_time = clock();
        let task_id = create_and_start_task(&db, "Gym workout", Some(1), start_time);
        notify.notify_one();

        // Advance 30s, then pause
        tokio::time::advance(Duration::from_secs(30)).await;
        tokio::task::yield_now().await;

        {
            let db_guard = db.lock().unwrap();
            let service = TaskService::new(db_guard.conn());
            service.pause(Some(task_id), clock()).unwrap();
        }
        notify.notify_one();

        // Advance 60s while paused: no notification!
        tokio::time::advance(Duration::from_secs(60)).await;
        tokio::task::yield_now().await;
        assert_eq!(notifier.count(), 0);

        handle.abort();
    }

    #[tokio::test]
    async fn resume_re_arms_notification() {
        tokio::time::pause();
        let (db, notifier, notify, clock, scheduler) = setup_scheduler();

        let handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        let start_time = clock();
        let task_id = create_and_start_task(&db, "Reading", Some(1), start_time);
        notify.notify_one();

        // 30s elapsed -> pause
        tokio::time::advance(Duration::from_secs(30)).await;
        tokio::task::yield_now().await;
        {
            let db_guard = db.lock().unwrap();
            let service = TaskService::new(db_guard.conn());
            service.pause(Some(task_id), clock()).unwrap();
        }
        notify.notify_one();

        // Paused for 100s -> no notification
        tokio::time::advance(Duration::from_secs(100)).await;
        tokio::task::yield_now().await;
        assert_eq!(notifier.count(), 0);

        // Resume: needs 30 more seconds of running time to hit 60s
        {
            let db_guard = db.lock().unwrap();
            let service = TaskService::new(db_guard.conn());
            service.resume(Some(task_id), clock()).unwrap();
        }
        notify.notify_one();

        // 20s elapsed -> still 0
        tokio::time::advance(Duration::from_secs(20)).await;
        tokio::task::yield_now().await;
        assert_eq!(notifier.count(), 0);

        // 11s more (total 31s running) -> notification fires!
        tokio::time::advance(Duration::from_secs(11)).await;
        tokio::task::yield_now().await;
        assert_eq!(notifier.count(), 1);

        handle.abort();
    }

    #[tokio::test]
    async fn restart_clears_notified_at_and_fires_again() {
        tokio::time::pause();
        let (db, notifier, notify, clock, scheduler) = setup_scheduler();

        let handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        let start_time = clock();
        let task_id = create_and_start_task(&db, "Coding", Some(1), start_time);
        notify.notify_one();

        // Pass 61s: fires notification #1
        tokio::time::advance(Duration::from_secs(61)).await;
        tokio::task::yield_now().await;
        assert_eq!(notifier.count(), 1);

        // Restart the task: run_no increments to 2, notified_at reset to NULL
        {
            let db_guard = db.lock().unwrap();
            let service = TaskService::new(db_guard.conn());
            service.restart(Some(task_id), clock()).unwrap();
        }
        notify.notify_one();

        // 30s of new run -> still 1
        tokio::time::advance(Duration::from_secs(30)).await;
        tokio::task::yield_now().await;
        assert_eq!(notifier.count(), 1);

        // 31s more (total 61s in run #2) -> fires notification #2!
        tokio::time::advance(Duration::from_secs(31)).await;
        tokio::task::yield_now().await;
        assert_eq!(notifier.count(), 2);

        handle.abort();
    }

    #[tokio::test]
    async fn extending_estimate_re_arms() {
        tokio::time::pause();
        let (db, notifier, notify, clock, scheduler) = setup_scheduler();

        let handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        let start_time = clock();
        let task_id = create_and_start_task(&db, "Cooking", Some(1), start_time);
        notify.notify_one();

        // 30s elapsed
        tokio::time::advance(Duration::from_secs(30)).await;
        tokio::task::yield_now().await;

        // Extend estimate to 3 mins (180s)
        {
            let db_guard = db.lock().unwrap();
            let service = TaskService::new(db_guard.conn());
            service
                .edit(
                    task_id,
                    UpdateTask {
                        estimated_mins: Some(Some(3)),
                        ..Default::default()
                    },
                    clock(),
                )
                .unwrap();
        }
        notify.notify_one();

        // 60s from start (30s after edit) -> old estimate expired, but new one has 120s left: no fire!
        tokio::time::advance(Duration::from_secs(40)).await;
        tokio::task::yield_now().await;
        assert_eq!(notifier.count(), 0);

        // Advance 120s more (total 190s > 180s) -> fires!
        tokio::time::advance(Duration::from_secs(120)).await;
        tokio::task::yield_now().await;
        assert_eq!(notifier.count(), 1);

        handle.abort();
    }

    #[tokio::test]
    async fn boot_time_catch_up_notification() {
        let (db, notifier, notify, _clock, scheduler) = setup_scheduler();

        // Create and start a task in the past (already overtime)
        let past_time = Utc::now() - chrono::Duration::minutes(10);
        let _task_id = create_and_start_task(&db, "Overdue task", Some(5), past_time);

        // First tick at startup immediately notices it is overtime and fires
        let now = Utc::now();
        let _ = scheduler.tick(now).unwrap();

        assert_eq!(notifier.count(), 1);
        assert_eq!(
            notifier.notifications(),
            vec![("Time's up".to_string(), "Overdue task (5 mins)".to_string())]
        );

        let _ = notify;
    }

    #[tokio::test]
    async fn task_without_estimate_never_fires() {
        tokio::time::pause();
        let (db, notifier, notify, clock, scheduler) = setup_scheduler();

        let handle = tokio::spawn(async move {
            scheduler.run().await;
        });

        let start_time = clock();
        let _task_id = create_and_start_task(&db, "Endless task", None, start_time);
        notify.notify_one();

        // Advance 2 hours
        tokio::time::advance(Duration::from_secs(7200)).await;
        tokio::task::yield_now().await;

        assert_eq!(notifier.count(), 0);

        handle.abort();
    }
}

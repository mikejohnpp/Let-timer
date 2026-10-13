use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use rusqlite::Connection;

use crate::db::settings_repository::SettingsRepository;
use crate::protocol::WorkspaceScope;

/// Request context carrying wall-clock time, user-local "today", and optional workspace scope override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestContext {
    pub now: DateTime<Utc>,
    pub today: NaiveDate,
    pub workspace: Option<WorkspaceScope>,
}

impl RequestContext {
    pub fn new(now: DateTime<Utc>, today: NaiveDate, workspace: Option<WorkspaceScope>) -> Self {
        Self {
            now,
            today,
            workspace,
        }
    }

    /// Convenience constructor for tests and UTC-only contexts.
    pub fn utc(now: DateTime<Utc>) -> Self {
        Self {
            today: now.date_naive(),
            now,
            workspace: None,
        }
    }

    /// Build a RequestContext resolving timezone from settings / env / override.
    pub fn from_settings(
        conn: &Connection,
        now: DateTime<Utc>,
        tz_override: Option<&str>,
        workspace: Option<WorkspaceScope>,
    ) -> Self {
        let tz_str = tz_override
            .map(|s| s.to_string())
            .or_else(|| std::env::var("LET_TIMER_TZ").ok())
            .or_else(|| {
                let settings = SettingsRepository::new(conn);
                settings.get_timezone().ok()
            })
            .unwrap_or_else(|| "UTC".to_string());

        let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
        let today = now.with_timezone(&tz).date_naive();

        Self {
            now,
            today,
            workspace,
        }
    }
}

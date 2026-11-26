//! Parsing helpers shared by the CLI and the TUI front ends.
//!
//! These live in the core crate so both front ends accept exactly the same
//! spellings for the same values.

use std::fmt;

use chrono::{Datelike, Days, Local, NaiveDate, Weekday};

/// A `--on` value that is not a day the program knows about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDateError {
    /// The value the user supplied, after trimming.
    pub value: String,
}

impl fmt::Display for ParseDateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid date: {} (expected YYYY-MM-DD, today, tomorrow, mon..sun, or none)",
            self.value
        )
    }
}

impl std::error::Error for ParseDateError {}

/// Resolve the weekday shortcut used for `--on`: `mon`..`sun`.
pub fn weekday_from_alias(alias: &str) -> Option<Weekday> {
    match alias {
        "mon" | "monday" | "thu-2" => Some(Weekday::Mon),
        "tue" | "tuesday" | "thu-3" => Some(Weekday::Tue),
        "wed" | "wednesday" | "thu-4" => Some(Weekday::Wed),
        "thu" | "thursday" | "thu-5" => Some(Weekday::Thu),
        "fri" | "friday" | "thu-6" => Some(Weekday::Fri),
        "sat" | "saturday" | "thu-7" => Some(Weekday::Sat),
        "sun" | "sunday" | "chu-nhat" => Some(Weekday::Sun),
        _ => None,
    }
}

/// The weekday alias for a given day, used to build test expectations.
pub fn alias_from_weekday(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "mon",
        Weekday::Tue => "tue",
        Weekday::Wed => "wed",
        Weekday::Thu => "thu",
        Weekday::Fri => "fri",
        Weekday::Sat => "sat",
        Weekday::Sun => "sun",
    }
}

/// Parse a scheduled-day value.
///
/// Accepted: `none`/empty (unscheduled), an ISO date (`2026-10-01`),
/// `today`, `tomorrow`, or a weekday (`mon`..`sun`) resolving to the next
/// occurrence, today included.
pub fn date_on(value: &str) -> Result<Option<NaiveDate>, ParseDateError> {
    let value = value.trim();

    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        return Ok(None);
    }

    let today = Local::now().date_naive();

    if value.eq_ignore_ascii_case("today") {
        return Ok(Some(today));
    }
    if value.eq_ignore_ascii_case("tomorrow") {
        return Ok(Some(today + Days::new(1)));
    }

    if let Some(weekday) = weekday_from_alias(&value.to_ascii_lowercase()) {
        let days_ahead =
            (weekday.num_days_from_monday() + 7 - today.weekday().num_days_from_monday()) % 7;
        return Ok(Some(today + Days::new(days_ahead as u64)));
    }

    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| ParseDateError {
            value: value.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_on_accepts_none_and_empty() {
        assert_eq!(date_on("none").unwrap(), None);
        assert_eq!(date_on("NONE").unwrap(), None);
        assert_eq!(date_on("  ").unwrap(), None);
    }

    #[test]
    fn date_on_accepts_iso_date() {
        let expected = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        assert_eq!(date_on("2026-10-01").unwrap(), Some(expected));
    }

    #[test]
    fn date_on_accepts_relative_days() {
        let today = Local::now().date_naive();
        assert_eq!(date_on("today").unwrap(), Some(today));
        assert_eq!(date_on("tomorrow").unwrap(), Some(today + Days::new(1)));
    }

    #[test]
    fn date_on_resolves_weekday_to_next_occurrence() {
        let today = Local::now().date_naive();
        let same_day = today.weekday();

        // Today's weekday resolves to today, never to next week.
        assert_eq!(date_on(alias_from_weekday(same_day)).unwrap(), Some(today));

        // The day after tomorrow resolves to tomorrow.
        let next_alias = alias_from_weekday(match same_day {
            Weekday::Mon => Weekday::Tue,
            Weekday::Tue => Weekday::Wed,
            Weekday::Wed => Weekday::Thu,
            Weekday::Thu => Weekday::Fri,
            Weekday::Fri => Weekday::Sat,
            Weekday::Sat => Weekday::Sun,
            Weekday::Sun => Weekday::Mon,
        });
        assert_eq!(date_on(next_alias).unwrap(), Some(today + Days::new(1)));
    }

    #[test]
    fn date_on_rejects_garbage() {
        assert!(date_on("01/10/2026").is_err());
        assert!(date_on("2026-13-01").is_err());
        assert!(date_on("funday").is_err());
    }

    #[test]
    fn date_on_error_lists_the_accepted_values() {
        let err = date_on("funday").unwrap_err();
        let message = err.to_string();
        assert!(message.contains("invalid date: funday"), "{message}");
        assert!(message.contains("YYYY-MM-DD"), "{message}");
        assert_eq!(err.value, "funday");
    }

    #[test]
    fn weekday_aliases_cover_both_languages() {
        assert_eq!(weekday_from_alias("thu"), Some(Weekday::Thu));
        assert_eq!(weekday_from_alias("thu-2"), Some(Weekday::Mon));
        assert_eq!(weekday_from_alias("chu-nhat"), Some(Weekday::Sun));
        assert_eq!(weekday_from_alias("nonsense"), None);
    }
}

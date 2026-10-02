//! Reading `settings.toml`.
//!
//! ```toml
//! # How often the list is refreshed from the daemon.
//! poll_interval_ms = 3000
//!
//! # How tall an inline list may grow before it starts scrolling.
//! inline_max_height = 15
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use super::ConfigError;

/// The file name looked for inside the config directory.
const FILE_NAME: &str = "settings.toml";

/// The shortest refresh interval worth having.
///
/// Below this the daemon is asked for the same list over and over for no
/// benefit; the numbers only change when somebody edits them.
pub const MIN_POLL_INTERVAL_MS: u64 = 100;

/// The shortest inline list worth drawing.
///
/// A list needs a line for its heading and at least one row, and anything
/// shorter shows nothing useful.
pub const MIN_INLINE_MAX_HEIGHT: u16 = 3;

/// The numbers the interface runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    poll_interval: Duration,
    inline_max_height: u16,
}

impl Settings {
    /// The values used when there is no config file.
    pub fn defaults() -> Self {
        Self {
            poll_interval: Duration::from_secs(3),
            inline_max_height: 15,
        }
    }

    /// Build settings directly, for a caller that already knows the numbers.
    pub fn new(poll_interval: Duration, inline_max_height: u16) -> Option<Self> {
        if poll_interval < Duration::from_millis(MIN_POLL_INTERVAL_MS)
            || inline_max_height < MIN_INLINE_MAX_HEIGHT
        {
            return None;
        }
        Some(Self {
            poll_interval,
            inline_max_height,
        })
    }

    /// How often to ask the daemon for the list again.
    pub fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    /// How tall an inline list may grow before it starts scrolling.
    pub fn inline_max_height(&self) -> u16 {
        self.inline_max_height
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::defaults()
    }
}

/// A settings file, as written down.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    poll_interval_ms: Option<u64>,
    inline_max_height: Option<u16>,
}

/// Where the settings live, given let-timer's config directory.
pub fn path_in(config_home: &Path) -> PathBuf {
    config_home.join(FILE_NAME)
}

/// Where the settings live on this machine.
pub fn config_path() -> Option<PathBuf> {
    super::config_home().map(|config_home| path_in(&config_home))
}

/// Read the settings from where they normally live.
///
/// A missing file is not a problem: it means the defaults.
pub fn load() -> Result<Settings, ConfigError> {
    match config_path() {
        Some(path) => load_from(&path),
        None => Ok(Settings::defaults()),
    }
}

/// Read the settings from `path`, starting from the defaults.
pub fn load_from(path: &Path) -> Result<Settings, ConfigError> {
    let Some(text) = super::read_optional(path)? else {
        return Ok(Settings::defaults());
    };

    let file: File = toml::from_str(&text).map_err(|error| ConfigError::Parse {
        path: path.to_path_buf(),
        message: error.message().to_string(),
    })?;

    let mut settings = Settings::defaults();

    if let Some(ms) = file.poll_interval_ms {
        if ms < MIN_POLL_INTERVAL_MS {
            return Err(ConfigError::OutOfRange {
                path: path.to_path_buf(),
                field: "poll_interval_ms",
                value: ms.to_string(),
                allowed: format!("cannot be below {MIN_POLL_INTERVAL_MS}"),
            });
        }
        settings.poll_interval = Duration::from_millis(ms);
    }

    if let Some(height) = file.inline_max_height {
        if height < MIN_INLINE_MAX_HEIGHT {
            return Err(ConfigError::OutOfRange {
                path: path.to_path_buf(),
                field: "inline_max_height",
                value: height.to_string(),
                allowed: format!("cannot be below {MIN_INLINE_MAX_HEIGHT}"),
            });
        }
        settings.inline_max_height = height;
    }

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch file that cleans itself up.
    struct Scratch(PathBuf);

    impl Scratch {
        fn with(self, contents: &str) -> Self {
            std::fs::create_dir_all(self.0.parent().unwrap()).unwrap();
            std::fs::write(&self.0, contents).unwrap();
            self
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn scratch(name: &str) -> Scratch {
        Scratch(std::env::temp_dir().join(format!(
            "let-timer-settings-{}-{}",
            std::process::id(),
            name
        )))
    }

    // ── the defaults ───────────────────────────────────────────────────

    #[test]
    fn the_defaults_refresh_every_three_seconds() {
        assert_eq!(Settings::defaults().poll_interval(), Duration::from_secs(3));
    }

    #[test]
    fn the_defaults_keep_an_inline_list_to_fifteen_rows() {
        assert_eq!(Settings::defaults().inline_max_height(), 15);
    }

    #[test]
    fn the_defaults_are_the_fallback() {
        assert_eq!(Settings::default(), Settings::defaults());
    }

    #[test]
    fn settings_can_be_built_directly() {
        let settings = Settings::new(Duration::from_millis(500), 8).unwrap();
        assert_eq!(settings.poll_interval(), Duration::from_millis(500));
        assert_eq!(settings.inline_max_height(), 8);
    }

    #[test]
    fn settings_built_by_hand_are_held_to_the_same_limits() {
        assert!(Settings::new(Duration::from_millis(1), 8).is_none());
        assert!(Settings::new(Duration::from_secs(1), 1).is_none());
        assert!(Settings::new(Duration::from_secs(1), MIN_INLINE_MAX_HEIGHT).is_some());
    }

    // ── no file at all ─────────────────────────────────────────────────

    #[test]
    fn a_missing_file_leaves_the_defaults_alone() {
        let path = scratch("missing").0.clone();
        let _ = std::fs::remove_file(&path);

        assert_eq!(load_from(&path).unwrap(), Settings::defaults());
    }

    #[test]
    fn an_empty_file_leaves_the_defaults_alone() {
        let file = scratch("empty").with("");
        assert_eq!(load_from(&file.0).unwrap(), Settings::defaults());
    }

    #[test]
    fn a_file_with_only_comments_leaves_the_defaults_alone() {
        let file = scratch("comments").with("# nothing to see here\n");
        assert_eq!(load_from(&file.0).unwrap(), Settings::defaults());
    }

    // ── overrides ──────────────────────────────────────────────────────

    #[test]
    fn both_numbers_can_be_set() {
        let file = scratch("both").with("poll_interval_ms = 750\ninline_max_height = 20\n");
        let settings = load_from(&file.0).unwrap();

        assert_eq!(settings.poll_interval(), Duration::from_millis(750));
        assert_eq!(settings.inline_max_height(), 20);
    }

    #[test]
    fn one_number_set_leaves_the_other_at_its_default() {
        let file = scratch("one").with("poll_interval_ms = 1000\n");
        let settings = load_from(&file.0).unwrap();

        assert_eq!(settings.poll_interval(), Duration::from_millis(1000));
        assert_eq!(settings.inline_max_height(), 15);
    }

    #[test]
    fn a_large_height_is_allowed() {
        let file = scratch("tall").with("inline_max_height = 200\n");
        assert_eq!(load_from(&file.0).unwrap().inline_max_height(), 200);
    }

    #[test]
    fn a_long_refresh_interval_is_allowed() {
        let file = scratch("slow").with("poll_interval_ms = 600000\n");
        assert_eq!(
            load_from(&file.0).unwrap().poll_interval(),
            Duration::from_secs(600)
        );
    }

    // ── numbers that make no sense ─────────────────────────────────────

    #[test]
    fn a_refresh_interval_of_zero_is_refused() {
        // Zero would mean asking the daemon as fast as the machine can manage.
        let file = scratch("zero").with("poll_interval_ms = 0\n");
        let error = load_from(&file.0).unwrap_err();

        let ConfigError::OutOfRange { field, .. } = &error else {
            panic!("expected out of range, got {error:?}");
        };
        assert_eq!(*field, "poll_interval_ms");
        assert!(error.to_string().contains("0"), "{error}");
    }

    #[test]
    fn a_refresh_interval_below_the_floor_is_refused() {
        let file = scratch("too-fast").with("poll_interval_ms = 10\n");
        let error = load_from(&file.0).unwrap_err();
        assert!(matches!(error, ConfigError::OutOfRange { .. }), "{error:?}");
    }

    #[test]
    fn the_floor_itself_is_allowed() {
        let file =
            scratch("at-floor").with(&format!("poll_interval_ms = {MIN_POLL_INTERVAL_MS}\n"));
        assert!(load_from(&file.0).is_ok());
    }

    #[test]
    fn an_inline_height_of_zero_is_refused() {
        // Zero rows would draw nothing at all and look like a broken command.
        let file = scratch("no-rows").with("inline_max_height = 0\n");
        let error = load_from(&file.0).unwrap_err();

        let ConfigError::OutOfRange { field, .. } = &error else {
            panic!("expected out of range, got {error:?}");
        };
        assert_eq!(*field, "inline_max_height");
    }

    #[test]
    fn an_inline_height_below_the_floor_is_refused() {
        let file = scratch("too-short").with("inline_max_height = 2\n");
        assert!(matches!(
            load_from(&file.0).unwrap_err(),
            ConfigError::OutOfRange { .. }
        ));
    }

    #[test]
    fn a_height_of_more_rows_than_fit_is_allowed() {
        // The list scrolls rather than refusing; the caller decides what fits.
        let file = scratch("taller-than-terminal").with("inline_max_height = 65535\n");
        assert_eq!(load_from(&file.0).unwrap().inline_max_height(), 65535);
    }

    #[test]
    fn a_negative_number_is_reported() {
        let file = scratch("negative").with("inline_max_height = -1\n");
        assert!(matches!(
            load_from(&file.0).unwrap_err(),
            ConfigError::Parse { .. }
        ));
    }

    #[test]
    fn a_number_where_a_string_belongs_is_reported() {
        let file = scratch("wrong-type").with("poll_interval_ms = \"3s\"\n");
        let error = load_from(&file.0).unwrap_err();
        assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
    }

    #[test]
    fn a_misspelled_setting_is_reported_rather_than_ignored() {
        let file = scratch("typo").with("poll_intervall_ms = 3000\n");
        let error = load_from(&file.0).unwrap_err();

        assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
        assert!(error.to_string().contains("poll_intervall_ms"), "{error}");
    }

    #[test]
    fn broken_toml_is_reported() {
        let file = scratch("broken").with("poll_interval_ms = ");
        assert!(matches!(
            load_from(&file.0).unwrap_err(),
            ConfigError::Parse { .. }
        ));
    }

    #[test]
    fn a_file_that_is_not_a_table_is_reported() {
        let file = scratch("not-a-table").with("hello\n");
        assert!(matches!(
            load_from(&file.0).unwrap_err(),
            ConfigError::Parse { .. }
        ));
    }

    #[test]
    fn a_file_that_cannot_be_read_is_reported() {
        let path =
            std::env::temp_dir().join(format!("let-timer-settings-dir-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        let error = load_from(&path).unwrap_err();

        let _ = std::fs::remove_dir_all(&path);
        assert!(matches!(error, ConfigError::Read { .. }), "{error:?}");
    }

    // ── where the file lives ───────────────────────────────────────────

    #[test]
    fn the_file_is_named_settings_toml_next_to_the_keymap() {
        let config_home = Path::new("/home/someone/.config/let-timer");

        assert_eq!(
            path_in(config_home),
            PathBuf::from("/home/someone/.config/let-timer/settings.toml")
        );
        assert_eq!(
            super::super::keymap_file::path_in(config_home),
            PathBuf::from("/home/someone/.config/let-timer/keymap.toml")
        );
    }
}

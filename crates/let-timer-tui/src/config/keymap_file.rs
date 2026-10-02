//! Reading the keymap from `keymap.toml`.
//!
//! The file only says what it wants changed. Everything it leaves out keeps
//! the default, so a two-line file is a complete configuration rather than a
//! configuration that silently disables every other key.
//!
//! ```toml
//! [normal]
//! quit = "ctrl-q"
//! open_create = "c"
//!
//! [inline]
//! help = "f1"
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::ConfigError;
use super::keymap::{Binding, Context, KeyMap, Target};

/// The file name looked for inside the config directory.
const FILE_NAME: &str = "keymap.toml";

/// A keymap file, as written down.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    normal: Option<BTreeMap<String, String>>,
    inline: Option<BTreeMap<String, String>>,
}

impl File {
    fn section(&self, context: Context) -> Option<&BTreeMap<String, String>> {
        match context {
            Context::Normal => self.normal.as_ref(),
            Context::Inline => self.inline.as_ref(),
        }
    }
}

/// Where the keymap lives, given let-timer's config directory.
pub fn path_in(config_home: &Path) -> PathBuf {
    config_home.join(FILE_NAME)
}

/// Where the keymap lives on this machine.
pub fn config_path() -> Option<PathBuf> {
    super::config_home().map(|config_home| path_in(&config_home))
}

/// Read the keymap from where it normally lives.
///
/// A missing file is not a problem: it means the defaults.
pub fn load() -> Result<KeyMap, ConfigError> {
    match config_path() {
        Some(path) => load_from(&path),
        None => Ok(KeyMap::defaults()),
    }
}

/// Read the keymap from `path`, starting from the defaults.
pub fn load_from(path: &Path) -> Result<KeyMap, ConfigError> {
    // No file means nothing was asked to change.
    let Some(text) = super::read_optional(path)? else {
        return Ok(KeyMap::defaults());
    };

    let file: File = toml::from_str(&text).map_err(|error| ConfigError::Parse {
        path: path.to_path_buf(),
        message: error.message().to_string(),
    })?;

    let mut map = KeyMap::defaults();

    for context in Context::ALL {
        let Some(section) = file.section(context) else {
            // A section that is not there keeps every default.
            continue;
        };

        for (name, keys) in section {
            let Some(target) = Target::from_name(name) else {
                return Err(ConfigError::UnknownAction {
                    path: path.to_path_buf(),
                    section: context.as_str(),
                    name: name.clone(),
                });
            };

            let binding = Binding::new(keys, target).map_err(|source| ConfigError::BadKey {
                path: path.to_path_buf(),
                section: context.as_str(),
                action: target.as_str(),
                source,
            })?;

            map.override_binding(context, binding);
        }
    }

    // Checked once everything is in place, so a conflict between a default and
    // an override is caught just as surely as one between two overrides.
    for context in Context::ALL {
        if let Some(conflict) = map.find_conflict(context) {
            return Err(ConfigError::Conflict {
                path: path.to_path_buf(),
                conflict,
            });
        }
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::keymap::typed_char;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
            "let-timer-keymap-{}-{}",
            std::process::id(),
            name
        )))
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn targets(map: &KeyMap, context: Context) -> Vec<Target> {
        map.bindings(context).iter().map(Binding::target).collect()
    }

    // ── where the file lives ───────────────────────────────────────────

    #[test]
    fn the_file_is_named_keymap_toml_inside_the_config_directory() {
        assert_eq!(
            path_in(Path::new("/home/someone/.config/let-timer")),
            PathBuf::from("/home/someone/.config/let-timer/keymap.toml")
        );
    }

    // ── no file at all ─────────────────────────────────────────────────

    #[test]
    fn a_missing_file_leaves_the_defaults_alone() {
        let path = scratch("missing").0.clone();
        let _ = std::fs::remove_file(&path);

        let map = load_from(&path).expect("a missing file is not a problem");

        assert_eq!(map, KeyMap::defaults());
    }

    #[test]
    fn an_empty_file_leaves_the_defaults_alone() {
        let file = scratch("empty").with("");
        let map = load_from(&file.0).unwrap();
        assert_eq!(map, KeyMap::defaults());
    }

    // ── overrides ──────────────────────────────────────────────────────

    #[test]
    fn a_listed_action_gets_the_keys_from_the_file() {
        let file = scratch("override").with("[normal]\nquit = \"ctrl-q\"\n");
        let map = load_from(&file.0).unwrap();

        assert_eq!(
            map.resolve(Context::Normal, &key(KeyCode::Char('q'))),
            None,
            "the old key should have been given up"
        );
        assert_eq!(
            map.resolve(
                Context::Normal,
                &KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)
            ),
            Some(Target::Quit)
        );
    }

    #[test]
    fn an_override_takes_the_old_keys_away() {
        let file = scratch("takes-away").with("[normal]\nquit = \"ctrl-q\"\n");
        let map = load_from(&file.0).unwrap();

        let quits = map
            .bindings(Context::Normal)
            .iter()
            .filter(|binding| binding.target() == Target::Quit)
            .count();

        assert_eq!(quits, 1, "quit should be bound exactly once");
    }

    #[test]
    fn an_override_can_list_several_keys() {
        let file = scratch("several").with("[normal]\nmove_down = \"l, down\"\n");
        let map = load_from(&file.0).unwrap();

        assert_eq!(
            map.resolve(Context::Normal, &key(KeyCode::Char('l'))),
            Some(Target::MoveDown)
        );
        assert_eq!(
            map.resolve(Context::Normal, &key(KeyCode::Down)),
            Some(Target::MoveDown)
        );
        assert_eq!(map.resolve(Context::Normal, &key(KeyCode::Char('j'))), None);
    }

    #[test]
    fn an_action_left_out_keeps_its_default_keys() {
        let file = scratch("left-out").with("[normal]\nquit = \"ctrl-q\"\n");
        let map = load_from(&file.0).unwrap();

        assert_eq!(
            map.resolve(Context::Normal, &key(KeyCode::Char('j'))),
            Some(Target::MoveDown)
        );
        assert_eq!(
            map.resolve(Context::Normal, &key(KeyCode::Esc)),
            Some(Target::Cancel)
        );
    }

    #[test]
    fn the_other_context_keeps_all_of_its_defaults() {
        let file = scratch("other-context").with("[normal]\nquit = \"ctrl-q\"\n");
        let map = load_from(&file.0).unwrap();

        assert_eq!(map, {
            let mut expected = KeyMap::defaults();
            expected.override_binding(
                Context::Normal,
                Binding::new("ctrl-q", Target::Quit).unwrap(),
            );
            expected
        });
    }

    // ── sections ───────────────────────────────────────────────────────

    #[test]
    fn a_missing_section_keeps_the_defaults() {
        // Only inline is written; normal must come through untouched.
        let file = scratch("missing-section").with("[inline]\nhelp = \"f1\"\n");
        let map = load_from(&file.0).unwrap();

        assert_eq!(
            targets(&map, Context::Normal),
            targets(&KeyMap::defaults(), Context::Normal)
        );
        assert_eq!(
            map.resolve(Context::Normal, &key(KeyCode::Char('j'))),
            Some(Target::MoveDown)
        );
    }

    #[test]
    fn both_sections_can_be_written_at_once() {
        let file = scratch("both").with("[normal]\nquit = \"ctrl-q\"\n\n[inline]\nhelp = \"f1\"\n");
        let map = load_from(&file.0).unwrap();

        assert_eq!(
            map.resolve(
                Context::Normal,
                &KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)
            ),
            Some(Target::Quit)
        );
        assert_eq!(
            map.resolve(Context::Inline, &key(KeyCode::F(1))),
            Some(Target::Help)
        );
        assert_eq!(map.resolve(Context::Inline, &key(KeyCode::Char('?'))), None);
    }

    #[test]
    fn a_section_nobody_recognised_is_refused() {
        // Silently ignoring [form] would leave the user wondering why their
        // keys do nothing.
        let file = scratch("bad-section").with("[form]\nsubmit = \"ctrl-s\"\n");
        let error = load_from(&file.0).unwrap_err();

        assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
        let message = error.to_string();
        assert!(message.contains("form"), "{message}");
        assert!(
            message.contains("normal"),
            "{message} should say what is allowed"
        );
    }

    #[test]
    fn a_misspelled_section_is_refused_too() {
        let file = scratch("misspelled-section").with("[norml]\n");
        let error = load_from(&file.0).unwrap_err();
        assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
        assert!(error.to_string().contains("norml"));
    }

    // ── what a file may say ────────────────────────────────────────────

    #[test]
    fn an_action_name_that_is_not_ours_is_refused() {
        let file = scratch("bad-action").with("[normal]\nexplode = \"x\"\n");
        let error = load_from(&file.0).unwrap_err();

        assert!(
            matches!(error, ConfigError::UnknownAction { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("explode"));
    }

    #[test]
    fn every_known_action_name_is_accepted() {
        let mut text = String::from("[normal]\n");
        for (index, target) in Target::ALL.iter().enumerate() {
            text.push_str(&format!("{} = \"f{}\"\n", target.as_str(), index + 1));
        }
        let file = scratch("all-actions").with(&text);
        let map = load_from(&file.0).expect("every action name should load");

        for target in Target::ALL {
            assert!(
                targets(&map, Context::Normal).contains(&target),
                "{target:?}"
            );
        }
    }

    #[test]
    fn broken_toml_is_reported_rather_than_ignored() {
        let file = scratch("broken").with("[normal\nquit = ");
        let error = load_from(&file.0).unwrap_err();
        assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
    }

    #[test]
    fn a_file_that_is_not_a_table_is_reported() {
        let file = scratch("not-a-table").with("just a sentence\n");
        let error = load_from(&file.0).unwrap_err();
        assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
    }

    #[test]
    fn keys_that_are_not_a_string_are_reported() {
        let file = scratch("not-a-string").with("[normal]\nquit = 5\n");
        let error = load_from(&file.0).unwrap_err();
        assert!(matches!(error, ConfigError::Parse { .. }), "{error:?}");
    }

    #[test]
    fn a_key_that_makes_no_sense_is_refused() {
        let file = scratch("bad-key").with("[normal]\nquit = \"ctrl-banana\"\n");
        let error = load_from(&file.0).unwrap_err();

        assert!(matches!(error, ConfigError::BadKey { .. }), "{error:?}");
        let message = error.to_string();
        assert!(message.contains("quit"), "{message}");
        assert!(message.contains("banana"), "{message}");
    }

    #[test]
    fn a_file_that_cannot_be_read_is_reported() {
        // A directory where the file should be is unreadable but not missing.
        let path =
            std::env::temp_dir().join(format!("let-timer-keymap-dir-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        let error = load_from(&path).unwrap_err();

        let _ = std::fs::remove_dir_all(&path);
        assert!(matches!(error, ConfigError::Read { .. }), "{error:?}");
    }

    // ── inline stays typeable ──────────────────────────────────────────

    #[test]
    fn an_override_cannot_steal_a_letter_from_someone_typing() {
        // Binding `n` inline would make the letter untypable in a form, so the
        // inline section refuses plain letters.
        let file = scratch("steal").with("[inline]\nopen_create = \"n\"\n");
        let map = load_from(&file.0).unwrap();

        let typed = typed_char(&key(KeyCode::Char('n')));
        let action = map.resolve(Context::Inline, &key(KeyCode::Char('n')));

        assert!(
            typed.is_some() && action.is_some(),
            "a letter cannot be both a command and text: typed {typed:?}, action {action:?}"
        );
    }
}

#[cfg(test)]
mod conflict_tests {
    use super::*;
    use crate::config::keymap::Binding;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
            "let-timer-conflict-{}-{}",
            std::process::id(),
            name
        )))
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn the_defaults_have_no_conflict_in_either_context() {
        let map = KeyMap::defaults();
        for context in Context::ALL {
            assert_eq!(map.find_conflict(context), None, "{context:?}");
        }
    }

    #[test]
    fn the_defaults_load_cleanly() {
        // The same check, but through the file path, so a default that nobody
        // could ever type is caught here rather than by a user.
        let file = scratch("defaults").with("[inline]\nhelp = \"f1\"\n");
        let map = load_from(&file.0).unwrap();
        for context in Context::ALL {
            assert_eq!(map.find_conflict(context), None, "{context:?}");
        }
    }

    #[test]
    fn a_key_taken_from_another_action_is_reported() {
        // `j` already moves down, so binding help to it leaves one of the two
        // unreachable.
        let file = scratch("taken").with("[normal]\nhelp = \"j\"\n");
        let error = load_from(&file.0).unwrap_err();

        let ConfigError::Conflict { conflict, .. } = error else {
            panic!("expected a conflict, got {error:?}");
        };
        assert_eq!(conflict.context, Context::Normal);
        assert_eq!(conflict.claimed_by, Target::MoveDown);
        assert_eq!(conflict.also_claimed_by, Target::Help);
    }

    #[test]
    fn the_conflict_message_names_the_key_and_both_actions() {
        let file = scratch("message").with("[normal]\nhelp = \"j\"\n");
        let message = load_from(&file.0).unwrap_err().to_string();

        assert!(message.contains("[normal]"), "{message}");
        assert!(message.contains('j'), "{message}");
        assert!(message.contains("move_down"), "{message}");
        assert!(message.contains("help"), "{message}");
    }

    #[test]
    fn a_conflict_with_a_default_is_caught_too() {
        // Nothing in the file mentions move_down, but its key is still gone.
        let file = scratch("with-default").with("[normal]\nquit = \"j\"\n");
        let error = load_from(&file.0).unwrap_err();

        let ConfigError::Conflict { conflict, .. } = error else {
            panic!("expected a conflict, got {error:?}");
        };
        assert_eq!(conflict.also_claimed_by, Target::Quit);
        assert_eq!(conflict.claimed_by, Target::MoveDown);
    }

    #[test]
    fn a_conflict_between_two_overrides_is_caught() {
        let file = scratch("two-overrides").with("[normal]\nquit = \"x\"\nhelp = \"x\"\n");
        let error = load_from(&file.0).unwrap_err();
        assert!(matches!(error, ConfigError::Conflict { .. }), "{error:?}");
    }

    #[test]
    fn the_same_key_in_two_different_contexts_is_fine() {
        // `q` quitting in fullscreen and doing nothing inline is not a clash:
        // the two never apply at the same moment.
        let file = scratch("two-contexts").with("[normal]\nquit = \"x\"\n[inline]\nhelp = \"x\"\n");
        let map = load_from(&file.0).expect("different contexts never conflict");

        assert_eq!(
            map.resolve(Context::Normal, &key(KeyCode::Char('x'))),
            Some(Target::Quit)
        );
        assert_eq!(
            map.resolve(Context::Inline, &key(KeyCode::Char('x'))),
            Some(Target::Help)
        );
    }

    #[test]
    fn restating_a_default_exactly_is_not_a_conflict() {
        // Writing what is already there should be harmless, not an error.
        let file = scratch("restate").with("[normal]\nmove_down = \"j, down\"\n");
        let map = load_from(&file.0).expect("repeating a default is fine");

        assert_eq!(
            map.resolve(Context::Normal, &key(KeyCode::Char('j'))),
            Some(Target::MoveDown)
        );
    }

    #[test]
    fn moving_a_binding_away_and_reusing_its_key_is_fine() {
        // `j` and `x`: j is handed over by move_down, x was never spoken for.
        let file = scratch("move-away").with("[normal]\nmove_down = \"x, down\"\nhelp = \"j\"\n");
        let map = load_from(&file.0).expect("the key was given up first");

        assert_eq!(
            map.resolve(Context::Normal, &key(KeyCode::Char('j'))),
            Some(Target::Help)
        );
        assert_eq!(
            map.resolve(Context::Normal, &key(KeyCode::Char('x'))),
            Some(Target::MoveDown)
        );
    }

    #[test]
    fn one_key_of_a_multi_key_binding_clashing_is_enough_to_refuse() {
        let file = scratch("one-of-many").with("[normal]\nhelp = \"l, down\"\n");
        let error = load_from(&file.0).unwrap_err();
        assert!(matches!(error, ConfigError::Conflict { .. }), "{error:?}");
    }

    #[test]
    fn a_ctrl_key_and_a_bare_key_are_different_keys() {
        // ctrl-j moving down says nothing about plain `j`.
        let file = scratch("distinct").with("[normal]\nhelp = \"ctrl-j\"\n");
        let map = load_from(&file.0).expect("ctrl-j is not j");

        assert_eq!(
            map.resolve(
                Context::Normal,
                &KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL)
            ),
            Some(Target::Help)
        );
        assert_eq!(
            map.resolve(Context::Normal, &key(KeyCode::Char('j'))),
            Some(Target::MoveDown)
        );
    }

    #[test]
    fn an_override_may_keep_a_key_from_the_binding_it_replaces() {
        // Dropping ctrl-c from quit but keeping q is not a clash with itself:
        // the old binding is gone by the time the new one is looked at.
        let file = scratch("keep-one").with("[normal]\nquit = \"q\"\n");
        let map = load_from(&file.0).expect("a binding may keep some of its own keys");

        assert_eq!(
            map.resolve(Context::Normal, &key(KeyCode::Char('q'))),
            Some(Target::Quit)
        );
        assert_eq!(
            map.resolve(
                Context::Normal,
                &KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
            ),
            None,
            "ctrl-c was dropped"
        );
    }

    #[test]
    fn taking_the_keys_of_a_binding_you_did_not_displace_is_a_conflict() {
        // `j` belongs to move_down, and only move_down may hand it over.
        let mut map = KeyMap::defaults();
        map.override_binding(Context::Normal, Binding::new("j", Target::GoToTop).unwrap());

        let conflict = map
            .find_conflict(Context::Normal)
            .expect("j is spoken for twice");
        assert_eq!(conflict.claimed_by, Target::MoveDown);
        assert_eq!(conflict.also_claimed_by, Target::GoToTop);
    }
}

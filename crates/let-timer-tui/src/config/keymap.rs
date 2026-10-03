//! Turning key presses into actions.
//!
//! The keymap is a table and nothing more: it knows which key means "move
//! down" and it knows which key means "leave", but it does not know what is on
//! screen. Deciding what `e` means depends on whether a row is selected, and
//! that is the event loop's business.
//!
//! Key descriptions are written the way people write them in a config file:
//! `j`, `G`, `?`, `esc`, `ctrl-c`, `shift-tab`, `alt-enter`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;

/// What a key press asks for.
///
/// Two of these need data the table cannot supply: editing and deleting act on
/// whichever row is selected, which only the event loop knows. They stay
/// unresolved here on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    MoveDown,
    MoveUp,
    GoToTop,
    GoToBottom,
    OpenCreate,
    /// Unresolved: needs the selected task.
    OpenEdit,
    /// Unresolved: needs the selected task.
    ConfirmDelete,
    NextField,
    PreviousField,
    OpenCalendar,
    /// Unresolved: needs the day the calendar has on it.
    PickDate,
    ClearDate,
    CalendarPrevDay,
    CalendarNextDay,
    CalendarPrevWeek,
    CalendarNextWeek,
    /// Unresolved: needs the workspace the sidebar has highlighted.
    ApplyWorkspaceFilter,
    /// Show every workspace again.
    ClearWorkspaceFilter,
    /// Move the sidebar's highlight, not the list's.
    WorkspaceNext,
    WorkspacePrev,
    Submit,
    Cancel,
    Refresh,
    Help,
    DismissToast,
    Quit,
}

impl Target {
    /// The name used in the config file.
    pub fn as_str(&self) -> &'static str {
        match self {
            Target::MoveDown => "move_down",
            Target::MoveUp => "move_up",
            Target::GoToTop => "go_to_top",
            Target::GoToBottom => "go_to_bottom",
            Target::OpenCreate => "open_create",
            Target::OpenEdit => "open_edit",
            Target::ConfirmDelete => "confirm_delete",
            Target::NextField => "next_field",
            Target::PreviousField => "previous_field",
            Target::OpenCalendar => "open_calendar",
            Target::PickDate => "pick_date",
            Target::CalendarPrevDay => "calendar_prev_day",
            Target::CalendarNextDay => "calendar_next_day",
            Target::CalendarPrevWeek => "calendar_prev_week",
            Target::CalendarNextWeek => "calendar_next_week",
            Target::ClearDate => "clear_date",
            Target::ApplyWorkspaceFilter => "apply_workspace_filter",
            Target::ClearWorkspaceFilter => "clear_workspace_filter",
            Target::WorkspaceNext => "workspace_next",
            Target::WorkspacePrev => "workspace_prev",
            Target::Submit => "submit",
            Target::Cancel => "cancel",
            Target::Refresh => "refresh",
            Target::Help => "help",
            Target::DismissToast => "dismiss_toast",
            Target::Quit => "quit",
        }
    }

    /// Whether this target only moves the highlight around.
    ///
    /// A form swallows everything except the keys it has a use for, so a key it
    /// has no use for has to be asked whether it is safe to let through. Only
    /// navigation is: `d` pressed while a form is open, with the focus on a
    /// field that cannot be typed into, would otherwise open a delete dialog
    /// over somebody's half-written name.
    pub fn is_navigation(self) -> bool {
        matches!(
            self,
            Target::MoveDown | Target::MoveUp | Target::GoToTop | Target::GoToBottom
        )
    }

    /// Read a name from a config file, or `None` if there is no such action.
    pub fn from_name(name: &str) -> Option<Target> {
        Target::ALL
            .iter()
            .copied()
            .find(|target| target.as_str() == name)
    }

    /// Every action a binding may name, so a config can be checked against them.
    pub const ALL: [Target; 26] = [
        Target::MoveDown,
        Target::MoveUp,
        Target::GoToTop,
        Target::GoToBottom,
        Target::OpenCreate,
        Target::OpenEdit,
        Target::ConfirmDelete,
        Target::NextField,
        Target::PreviousField,
        Target::OpenCalendar,
        Target::PickDate,
        Target::ClearDate,
        Target::CalendarPrevDay,
        Target::CalendarNextDay,
        Target::CalendarPrevWeek,
        Target::CalendarNextWeek,
        Target::ApplyWorkspaceFilter,
        Target::ClearWorkspaceFilter,
        Target::WorkspaceNext,
        Target::WorkspacePrev,
        Target::Submit,
        Target::Cancel,
        Target::Refresh,
        Target::Help,
        Target::DismissToast,
        Target::Quit,
    ];
}

impl Target {
    /// Turn this into the action it stands for.
    ///
    /// `component` is the record type on screen, needed by `open_create`.
    /// Editing and deleting have no answer here and return `None`.
    pub fn into_action(self, component: crate::action::Component) -> Option<Action> {
        match self {
            Target::MoveDown => Some(Action::MoveSelection(1)),
            Target::MoveUp => Some(Action::MoveSelection(-1)),
            Target::GoToTop => Some(Action::Select(0)),
            Target::GoToBottom => Some(Action::Select(usize::MAX)),
            Target::OpenCreate => Some(Action::OpenCreate(component)),
            Target::NextField => Some(Action::FormNextField),
            Target::PreviousField => Some(Action::FormPrevField),
            Target::OpenCalendar => Some(Action::OpenCalendar),
            Target::ClearDate => Some(Action::ClearDate),
            Target::CalendarPrevDay => Some(Action::CalendarMove(-1)),
            Target::CalendarNextDay => Some(Action::CalendarMove(1)),
            Target::CalendarPrevWeek => Some(Action::CalendarMove(-7)),
            Target::CalendarNextWeek => Some(Action::CalendarMove(7)),
            Target::ClearWorkspaceFilter => Some(Action::SetWorkspaceFilter(None)),
            Target::WorkspaceNext => Some(Action::MoveWorkspaceSelection(1)),
            Target::WorkspacePrev => Some(Action::MoveWorkspaceSelection(-1)),
            Target::Submit => Some(Action::Submit),
            Target::Cancel => Some(Action::Cancel),
            Target::Refresh => Some(Action::Tick),
            Target::Help => Some(Action::Help),
            Target::DismissToast => Some(Action::DismissToast),
            Target::Quit => Some(Action::Quit),
            // A date is not a thing a key can name: the day the calendar has on
            // it is only known once the calendar is open. Neither is a
            // workspace: which one is highlighted belongs to the sidebar, and
            // only the loop can see both.
            Target::OpenEdit
            | Target::ConfirmDelete
            | Target::PickDate
            | Target::ApplyWorkspaceFilter => None,
        }
    }
}

/// One key press bound to one action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    keys: Vec<Key>,
    target: Target,
}

impl Binding {
    /// Bind a target to a key written the way a config file writes it.
    pub fn new(keys: &str, target: Target) -> Result<Self, ParseKeyError> {
        let keys = keys
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty());
        let keys: Result<Vec<_>, _> = keys.map(Key::parse).collect();
        Ok(Self {
            keys: keys?,
            target,
        })
    }

    /// The action this binding stands for.
    pub fn target(&self) -> Target {
        self.target
    }

    /// How this binding is written in a config file.
    pub fn keys_as_string(&self) -> String {
        self.keys
            .iter()
            .map(|key| key.config_name())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Whether this binding covers `key`.
    pub fn matches(&self, key: &KeyEvent) -> bool {
        self.keys.contains(&Key::from_event(*key))
    }
}

/// A key, as written in a config file.
///
/// Keys are normalised so that a description means the same thing however the
/// terminal wrote it down. Letters are folded to lower case and keep shift as
/// a flag, which is what lets `g` and `G` stay two different keys. Other
/// characters are left alone and drop shift, because a terminal already sends
/// the shifted character: there is no `!` and `shift-1` distinction to be had
/// without a keyboard layout table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    code: KeyCode,
    control: bool,
    alt: bool,
    shift: bool,
}

impl Key {
    /// Read one key description such as `j`, `G`, `?`, `esc` or `ctrl-c`.
    pub fn parse(written: &str) -> Result<Self, ParseKeyError> {
        let mut control = false;
        let mut alt = false;
        let mut shift = false;
        let mut name = written;

        for part in written.split('-') {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => control = true,
                "alt" | "option" => alt = true,
                "shift" => shift = true,
                _ => {
                    name = part;
                    break;
                }
            }
        }

        let code = if shift && name.eq_ignore_ascii_case("tab") {
            // Terminals send shift-tab as its own key rather than as tab held
            // with shift, so it has to be spelled out here.
            KeyCode::BackTab
        } else {
            code_from_name(&name.to_lowercase())?
        };

        // An upper-case letter is the same key held with shift.
        let shifted = shift || name.chars().any(char::is_uppercase);

        Ok(Self {
            code,
            control,
            alt,
            shift: shifted && is_letter(code),
        })
    }

    /// The key as crossterm reports it.
    ///
    /// See [`Key`] for why shift is kept only for letters.
    fn from_event(event: KeyEvent) -> Self {
        let shift = event.modifiers.contains(KeyModifiers::SHIFT);
        let code = match event.code {
            KeyCode::Char(letter) if letter.is_alphabetic() => {
                KeyCode::Char(letter.to_lowercase().next().unwrap_or(letter))
            }
            other => other,
        };

        Self {
            code,
            control: event.modifiers.contains(KeyModifiers::CONTROL),
            alt: event.modifiers.contains(KeyModifiers::ALT),
            shift: shift && is_letter(code),
        }
    }

    /// How this key is written in a config file.
    fn config_name(self) -> String {
        let mut out = String::new();
        if self.control {
            out.push_str("ctrl-");
        }
        if self.alt {
            out.push_str("alt-");
        }
        match (self.shift, self.code) {
            // A letter held with shift is just written in capitals.
            (true, KeyCode::Char(letter)) => {
                out.push_str(&letter.to_uppercase().to_string());
                return out;
            }
            (true, KeyCode::Tab) => {
                out.push_str("shift-tab");
                return out;
            }
            _ => {}
        }
        out.push_str(&code_from_config_name(&self.code));
        out
    }
}

/// A key description that makes no sense.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseKeyError {
    /// What was written.
    pub written: String,
    /// Why it could not be read.
    pub reason: String,
}

impl std::fmt::Display for ParseKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.written, self.reason)
    }
}

impl std::error::Error for ParseKeyError {}

/// Whether this key is a letter, and so cares whether shift was held.
fn is_letter(code: KeyCode) -> bool {
    match code {
        KeyCode::Char(letter) => letter.is_alphabetic(),
        _ => false,
    }
}

/// Named keys that are not a single character.
fn code_from_name(name: &str) -> Result<KeyCode, ParseKeyError> {
    let code = match name {
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "space" => KeyCode::Char(' '),
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        other if other.chars().count() == 1 => KeyCode::Char(other.chars().next().unwrap()),
        "" => {
            return Err(ParseKeyError {
                written: String::new(),
                reason: "no key given".to_string(),
            });
        }
        // Function keys past f12 are written the same way as the rest.
        other if function_number(other).is_some() => {
            KeyCode::F(function_number(other).expect("just checked"))
        }
        other => {
            return Err(ParseKeyError {
                written: other.to_string(),
                reason: "unknown key".to_string(),
            });
        }
    };
    Ok(code)
}

/// Which function key `f3` or `f24` names, if it names one.
fn function_number(name: &str) -> Option<u8> {
    let digits = name.strip_prefix('f')?;
    digits
        .parse()
        .ok()
        .filter(|number: &u8| (1..=24).contains(number))
}

/// How a key code is spelled in a config file.
fn code_from_config_name(code: &KeyCode) -> String {
    match code {
        KeyCode::Char(' ') => "space".to_string(),
        KeyCode::Char(letter) => letter.to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Esc => "esc".to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::BackTab => "shift-tab".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Delete => "delete".to_string(),
        KeyCode::Insert => "insert".to_string(),
        KeyCode::Home => "home".to_string(),
        KeyCode::End => "end".to_string(),
        KeyCode::PageUp => "pageup".to_string(),
        KeyCode::PageDown => "pagedown".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::F(number) => format!("f{number}"),
        other => format!("{other:?}").to_lowercase(),
    }
}

/// Which key bindings apply right now.
///
/// The places keys are read behave differently. A list has nothing to type into,
/// so letters are free to be commands. The date picker is a modal of its own, so
/// the arrows that move a list move the day in it instead, and the sidebar is a
/// pane of its own whose arrows move a workspace rather than a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    /// The list, with a form over it or not.
    Normal,
    /// The date picker, drawn over a form.
    Calendar,
    /// The list of workspaces down the left.
    Sidebar,
}

impl Context {
    /// The section name used in the config file.
    pub fn as_str(&self) -> &'static str {
        match self {
            Context::Normal => "normal",
            Context::Calendar => "calendar",
            Context::Sidebar => "sidebar",
        }
    }

    /// Read a section name, or `None` if there is no such section.
    pub fn from_name(name: &str) -> Option<Context> {
        match name {
            "normal" => Some(Context::Normal),
            "calendar" => Some(Context::Calendar),
            "sidebar" => Some(Context::Sidebar),
            _ => None,
        }
    }

    /// Every section, so a config file can be checked against them.
    pub const ALL: [Context; 3] = [Context::Normal, Context::Calendar, Context::Sidebar];

    /// How many sections there are, so the keymap can hold one list each.
    pub const COUNT: usize = 3;

    /// Which slot of the keymap holds this context's bindings.
    ///
    /// The order has to match [`Context::ALL`], which is the order a config
    /// file's sections are checked in.
    fn index(self) -> usize {
        match self {
            Context::Normal => 0,
            Context::Calendar => 1,
            Context::Sidebar => 2,
        }
    }
}

/// Which keys do what.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyMap {
    /// One list per [`Context`], indexed by [`Context::index`].
    contexts: [Vec<Binding>; Context::COUNT],
}

impl KeyMap {
    /// The bindings used when there is no config file.
    ///
    /// Nothing here can fail to parse, since every key is written out in full.
    pub fn defaults() -> Self {
        Self {
            contexts: [
                bindings(&[
                    ("j, down", Target::MoveDown),
                    ("k, up", Target::MoveUp),
                    ("g, home", Target::GoToTop),
                    ("G, end", Target::GoToBottom),
                    ("n", Target::OpenCreate),
                    ("e", Target::OpenEdit),
                    ("d", Target::ConfirmDelete),
                    ("tab", Target::NextField),
                    ("shift-tab", Target::PreviousField),
                    ("c", Target::OpenCalendar),
                    ("enter", Target::Submit),
                    ("r", Target::Refresh),
                    ("esc", Target::Cancel),
                    ("?", Target::Help),
                    ("q, ctrl-c", Target::Quit),
                ]),
                bindings(&[
                    // The picker is a modal of its own, so the arrows that move a
                    // list move the day here instead. Without its own context they
                    // would have to fight over the same keys.
                    ("left", Target::CalendarPrevDay),
                    ("right", Target::CalendarNextDay),
                    ("up", Target::CalendarPrevWeek),
                    ("down", Target::CalendarNextWeek),
                    ("enter", Target::Submit),
                    ("esc", Target::Cancel),
                ]),
                bindings(&[
                    // The sidebar's own keys. Its arrows move a workspace rather
                    // than a task, and they cannot be the list's own arrows
                    // through the dispatcher, which hands every action to every
                    // store.
                    ("j, down", Target::WorkspaceNext),
                    ("k, up", Target::WorkspacePrev),
                    ("g, home", Target::GoToTop),
                    ("G, end", Target::GoToBottom),
                    // `enter` applies the workspace the sidebar has highlighted,
                    // overriding the form's submit because there is no form in
                    // front of one.
                    ("enter", Target::ApplyWorkspaceFilter),
                    // Escape from the sidebar drops the filter rather than
                    // dropping focus: leaving a pane and undoing what the pane
                    // did are two different things to want, and one key should
                    // not mean both. `tab` is the way out.
                    ("esc", Target::ClearWorkspaceFilter),
                    // Focus has to be walkable from here as well as from the
                    // list, or `tab` would land on a pane and leave no way out
                    // of it.
                    ("tab", Target::NextField),
                    ("shift-tab", Target::PreviousField),
                ]),
            ],
        }
    }

    /// What `key` asks for in `context`, if anything.
    ///
    /// The first binding that covers the key wins, so a later binding cannot
    /// quietly shadow an earlier one unless it was placed on purpose.
    pub fn resolve(&self, context: Context, key: &KeyEvent) -> Option<Target> {
        self.bindings(context)
            .iter()
            .find(|binding| binding.matches(key))
            .map(Binding::target)
    }

    /// Replace the binding for one action.
    ///
    /// Saying `quit = "ctrl-q"` in a config file has to take `q` away from
    /// quitting, or the file would only ever add keys. So the default binding
    /// for this action goes, and the new one takes its place at the end.
    pub fn override_binding(&mut self, context: Context, binding: Binding) {
        let list = &mut self.contexts[context.index()];
        list.retain(|existing| existing.target() != binding.target());
        list.push(binding);
    }

    /// The bindings in force for `context`.
    pub fn bindings(&self, context: Context) -> &[Binding] {
        &self.contexts[context.index()]
    }

    /// The same list, for a caller that has to change its order.
    ///
    /// Only the tests want this. Every other caller uses `override_binding`,
    /// which replaces whatever was bound before instead of shadowing it, and a
    /// keymap with two bindings on one key cannot be written any other way.
    #[cfg(test)]
    fn bindings_mut(&mut self, context: Context) -> &mut Vec<Binding> {
        &mut self.contexts[context.index()]
    }

    /// A key that two actions in the same context both claim.
    ///
    /// Such a keymap cannot be predicted: one of the two actions would simply
    /// never happen, and which one depends on the order the bindings happen to
    /// be in. A config file that produces this is refused rather than loaded.
    pub fn find_conflict(&self, context: Context) -> Option<KeyConflict> {
        let list = self.bindings(context);
        for (index, first) in list.iter().enumerate() {
            for second in list.iter().skip(index + 1) {
                for key in &first.keys {
                    if second.keys.contains(key) {
                        return Some(KeyConflict {
                            context,
                            key: *key,
                            claimed_by: first.target(),
                            also_claimed_by: second.target(),
                        });
                    }
                }
            }
        }
        None
    }

    /// Every binding, whatever context it belongs to.
    pub fn all(&self) -> impl Iterator<Item = (Context, &Binding)> {
        Context::ALL
            .into_iter()
            .flat_map(|context| self.bindings(context).iter().map(move |b| (context, b)))
    }
}

/// Two actions in one context claiming the same key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyConflict {
    /// Where it happens.
    pub context: Context,
    /// The key in dispute.
    pub key: Key,
    /// The action whose binding was found first.
    pub claimed_by: Target,
    /// The action that can never fire.
    pub also_claimed_by: Target,
}

impl std::fmt::Display for KeyConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "in [{}] the key {} is bound to both {} and {}",
            self.context.as_str(),
            self.key.config_name(),
            self.claimed_by.as_str(),
            self.also_claimed_by.as_str()
        )
    }
}

/// Build the default bindings from a written-out table.
fn bindings(table: &[(&str, Target)]) -> Vec<Binding> {
    table
        .iter()
        .map(|(keys, target)| Binding::new(keys, *target).expect("the default keymap is valid"))
        .collect()
}

/// The character `key` types, if it types one.
///
/// This is the fallback for text input: a key that no binding claims becomes a
/// character when it can be one. Control and alt turn a key into a command
/// rather than a character, so they never come through here.
pub fn typed_char(key: &KeyEvent) -> Option<char> {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }
    match key.code {
        KeyCode::Char(letter) => Some(letter),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn modified(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    // ── key parsing ────────────────────────────────────────────────────

    #[test]
    fn a_plain_letter_parses() {
        let parsed = Key::parse("j").unwrap();
        assert_eq!(parsed, Key::from_event(key(KeyCode::Char('j'))));
    }

    #[test]
    fn an_uppercase_letter_is_that_letter_held_with_shift() {
        assert_eq!(
            Key::parse("G").unwrap(),
            Key::from_event(modified(KeyCode::Char('G'), KeyModifiers::SHIFT))
        );
    }

    #[test]
    fn an_uppercase_and_a_lower_case_letter_are_different_keys() {
        // This is what lets `g` go to the top and `G` to the bottom.
        assert_ne!(Key::parse("G").unwrap(), Key::parse("g").unwrap());
        assert_ne!(
            Key::from_event(modified(KeyCode::Char('G'), KeyModifiers::SHIFT)),
            Key::from_event(key(KeyCode::Char('g')))
        );
    }

    #[test]
    fn a_shift_prefix_and_a_capital_letter_describe_the_same_key() {
        assert_eq!(Key::parse("shift-g").unwrap(), Key::parse("G").unwrap());
    }

    #[test]
    fn a_digit_is_not_confused_with_its_shifted_symbol() {
        assert_ne!(Key::parse("1").unwrap(), Key::parse("!").unwrap());
        assert_eq!(
            Key::parse("!").unwrap(),
            Key::from_event(modified(KeyCode::Char('!'), KeyModifiers::SHIFT)),
            "a terminal sends the shifted character, and shift is dropped for it"
        );
    }

    #[test]
    fn a_punctuation_key_parses() {
        assert_eq!(
            Key::parse("?").unwrap(),
            Key::from_event(key(KeyCode::Char('?')))
        );
    }

    #[test]
    fn a_space_key_parses() {
        assert_eq!(
            Key::parse("space").unwrap(),
            Key::from_event(key(KeyCode::Char(' ')))
        );
    }

    #[test]
    fn a_named_key_parses() {
        for (written, code) in [
            ("enter", KeyCode::Enter),
            ("esc", KeyCode::Esc),
            ("tab", KeyCode::Tab),
            ("backspace", KeyCode::Backspace),
            ("up", KeyCode::Up),
            ("f5", KeyCode::F(5)),
        ] {
            assert_eq!(
                Key::parse(written).unwrap(),
                Key::from_event(key(code)),
                "{written}"
            );
        }
    }

    #[test]
    fn an_alias_parses_to_the_same_key() {
        assert_eq!(Key::parse("escape").unwrap(), Key::parse("esc").unwrap());
        assert_eq!(Key::parse("return").unwrap(), Key::parse("enter").unwrap());
    }

    #[test]
    fn shift_tab_parses_to_back_tab() {
        assert_eq!(
            Key::parse("shift-tab").unwrap(),
            Key::from_event(key(KeyCode::BackTab))
        );
    }

    #[test]
    fn a_modifier_is_kept_as_a_flag() {
        let parsed = Key::parse("ctrl-c").unwrap();
        assert_eq!(
            parsed,
            Key::from_event(modified(KeyCode::Char('c'), KeyModifiers::CONTROL))
        );
        assert_ne!(
            parsed,
            Key::from_event(key(KeyCode::Char('c'))),
            "ctrl-c must stay distinct from c"
        );
    }

    #[test]
    fn two_modifiers_can_be_combined() {
        assert_eq!(
            Key::parse("ctrl-alt-delete").unwrap(),
            Key::from_event(modified(
                KeyCode::Delete,
                KeyModifiers::CONTROL | KeyModifiers::ALT
            ))
        );
    }

    #[test]
    fn the_ctrl_prefix_is_spelled_either_way() {
        assert_eq!(
            Key::parse("ctrl-c").unwrap(),
            Key::parse("control-c").unwrap()
        );
    }

    #[test]
    fn an_unknown_key_is_refused() {
        let error = Key::parse("banana").unwrap_err();
        assert_eq!(error.written, "banana");
        assert_eq!(error.reason, "unknown key");
    }

    #[test]
    fn an_empty_key_is_refused() {
        assert!(Key::parse("").is_err());
    }

    // ── bindings ───────────────────────────────────────────────────────

    #[test]
    fn a_binding_matches_the_key_it_names() {
        let binding = Binding::new("j", Target::MoveDown).unwrap();
        assert!(binding.matches(&key(KeyCode::Char('j'))));
        assert!(!binding.matches(&key(KeyCode::Char('k'))));
    }

    #[test]
    fn a_binding_can_hold_several_keys() {
        let binding = Binding::new("j, down", Target::MoveDown).unwrap();
        assert!(binding.matches(&key(KeyCode::Char('j'))));
        assert!(binding.matches(&key(KeyCode::Down)));
        assert!(!binding.matches(&key(KeyCode::Char('l'))));
    }

    #[test]
    fn spaces_around_keys_in_a_list_are_ignored() {
        let binding = Binding::new(" j , down ", Target::MoveDown).unwrap();
        assert!(binding.matches(&key(KeyCode::Char('j'))));
        assert!(binding.matches(&key(KeyCode::Down)));
    }

    #[test]
    fn a_bad_key_in_a_list_fails_the_whole_binding() {
        assert!(Binding::new("j, banana", Target::MoveDown).is_err());
    }

    #[test]
    fn a_binding_writes_its_keys_back_out() {
        assert_eq!(
            Binding::new("ctrl-c", Target::Quit)
                .unwrap()
                .keys_as_string(),
            "ctrl-c"
        );
        assert_eq!(
            Binding::new("j, down", Target::MoveDown)
                .unwrap()
                .keys_as_string(),
            "j,down"
        );
        assert_eq!(
            Binding::new("shift-tab", Target::PreviousField)
                .unwrap()
                .keys_as_string(),
            "shift-tab"
        );
    }

    #[test]
    fn a_key_round_trips_through_its_config_name() {
        for written in [
            "j", "esc", "enter", "tab", "space", "ctrl-c", "alt-up", "f1",
        ] {
            let parsed = Key::parse(written).unwrap();
            let written_again = parsed.config_name();
            assert_eq!(
                Key::parse(&written_again).unwrap(),
                parsed,
                "{written} came back as {written_again}"
            );
        }
    }

    // ── targets ────────────────────────────────────────────────────────

    #[test]
    fn every_target_has_a_distinct_name() {
        let mut names: Vec<&str> = Target::ALL.iter().map(|target| target.as_str()).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();

        assert_eq!(names.len(), count, "two actions share a name");
        assert_eq!(Target::ALL.len(), 26, "every target is in the list");
    }

    #[test]
    fn a_target_is_found_by_name() {
        assert_eq!(Target::from_name("quit"), Some(Target::Quit));
        assert_eq!(Target::from_name("move_down"), Some(Target::MoveDown));
    }

    #[test]
    fn an_unknown_action_name_is_refused() {
        assert_eq!(Target::from_name("explode"), None);
        assert_eq!(Target::from_name(""), None);
    }

    fn action_of(target: Target) -> Option<Action> {
        target.into_action(crate::action::Component::Task)
    }

    #[test]
    fn a_simple_target_becomes_its_action() {
        assert!(matches!(
            action_of(Target::MoveDown),
            Some(Action::MoveSelection(1))
        ));
        assert!(matches!(
            action_of(Target::MoveUp),
            Some(Action::MoveSelection(-1))
        ));
        assert!(matches!(
            action_of(Target::GoToTop),
            Some(Action::Select(0))
        ));
    }

    #[test]
    fn opening_a_form_uses_the_component_on_screen() {
        assert!(matches!(
            Target::OpenCreate.into_action(crate::action::Component::Workspace),
            Some(Action::OpenCreate(crate::action::Component::Workspace))
        ));
    }

    #[test]
    fn going_to_the_bottom_asks_for_a_row_number_the_store_clamps() {
        assert!(matches!(
            action_of(Target::GoToBottom),
            Some(Action::Select(usize::MAX))
        ));
    }

    #[test]
    fn edit_and_delete_have_no_action_without_a_selected_row() {
        assert!(
            Target::OpenEdit
                .into_action(crate::action::Component::Task)
                .is_none()
        );
        assert!(
            Target::ConfirmDelete
                .into_action(crate::action::Component::Task)
                .is_none()
        );
    }

    #[test]
    fn the_rest_of_the_targets_become_their_actions() {
        assert!(matches!(
            action_of(Target::NextField),
            Some(Action::FormNextField)
        ));
        assert!(matches!(
            action_of(Target::PreviousField),
            Some(Action::FormPrevField)
        ));
        assert!(matches!(
            action_of(Target::OpenCalendar),
            Some(Action::OpenCalendar)
        ));
        // A date is looked up in the calendar, not built from a key.
        assert!(action_of(Target::PickDate).is_none());
        assert!(matches!(
            action_of(Target::CalendarNextDay),
            Some(Action::CalendarMove(1))
        ));
        assert!(matches!(
            action_of(Target::CalendarPrevWeek),
            Some(Action::CalendarMove(-7))
        ));
        assert!(matches!(
            action_of(Target::ClearDate),
            Some(Action::ClearDate)
        ));
        assert!(matches!(action_of(Target::Submit), Some(Action::Submit)));
        assert!(matches!(action_of(Target::Cancel), Some(Action::Cancel)));
        assert!(matches!(action_of(Target::Help), Some(Action::Help)));
        assert!(matches!(
            action_of(Target::DismissToast),
            Some(Action::DismissToast)
        ));
        assert!(matches!(action_of(Target::Quit), Some(Action::Quit)));

        // The one action with no target of its own: a refresh is just a tick.
        assert!(matches!(action_of(Target::Refresh), Some(Action::Tick)));
    }

    // ── contexts ───────────────────────────────────────────────────────

    fn press(code: KeyCode) -> KeyEvent {
        key(code)
    }

    #[test]
    fn a_context_is_found_by_section_name() {
        assert_eq!(Context::from_name("normal"), Some(Context::Normal));
        assert_eq!(Context::from_name("calendar"), Some(Context::Calendar));
        assert_eq!(Context::from_name("form"), None);
        // The panel that used to be drawn in the terminal is not a context
        // any more, so a file still carrying its section is refused rather than
        // half read.
        assert_eq!(Context::from_name("inline"), None);
        assert_eq!(Context::Normal.as_str(), "normal");
        assert_eq!(Context::Calendar.as_str(), "calendar");
    }

    #[test]
    fn every_context_has_a_distinct_section_name() {
        let names: Vec<&str> = Context::ALL.iter().map(|c| c.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len());
    }

    // ── resolving ──────────────────────────────────────────────────────

    #[test]
    fn the_default_normal_keys_resolve() {
        let map = KeyMap::defaults();
        assert_eq!(
            map.resolve(Context::Normal, &press(KeyCode::Char('j'))),
            Some(Target::MoveDown)
        );
        assert_eq!(
            map.resolve(Context::Normal, &press(KeyCode::Char('q'))),
            Some(Target::Quit)
        );
        assert_eq!(
            map.resolve(Context::Normal, &press(KeyCode::Char('?'))),
            Some(Target::Help)
        );
        assert_eq!(
            map.resolve(Context::Normal, &press(KeyCode::Esc)),
            Some(Target::Cancel)
        );
    }

    #[test]
    fn an_arrow_key_reaches_the_same_action_as_its_letter() {
        let map = KeyMap::defaults();
        assert_eq!(
            map.resolve(Context::Normal, &press(KeyCode::Down)),
            Some(Target::MoveDown)
        );
        assert_eq!(
            map.resolve(Context::Normal, &press(KeyCode::Char('j'))),
            Some(Target::MoveDown)
        );
    }

    #[test]
    fn ctrl_c_quits_from_the_list_and_only_from_the_list() {
        let map = KeyMap::defaults();
        let ctrl_c = modified(KeyCode::Char('c'), KeyModifiers::CONTROL);

        assert_eq!(map.resolve(Context::Normal, &ctrl_c), Some(Target::Quit));
        // The picker is left with the keys that dismiss it rather than the one
        // that ends the program, so a stray ctrl-c over a form does not take
        // the work with it.
        assert_eq!(map.resolve(Context::Calendar, &ctrl_c), None);
    }

    #[test]
    fn a_bare_letter_is_a_command() {
        let map = KeyMap::defaults();
        assert_eq!(
            map.resolve(Context::Normal, &press(KeyCode::Char('n'))),
            Some(Target::OpenCreate)
        );
    }

    #[test]
    fn the_arrow_keys_move_the_list() {
        let map = KeyMap::defaults();
        assert_eq!(
            map.resolve(Context::Normal, &press(KeyCode::Up)),
            Some(Target::MoveUp)
        );
        assert_eq!(
            map.resolve(Context::Normal, &press(KeyCode::Down)),
            Some(Target::MoveDown)
        );
    }

    #[test]
    fn an_unbound_key_resolves_to_nothing() {
        let map = KeyMap::defaults();
        assert_eq!(
            map.resolve(Context::Normal, &press(KeyCode::Char('Z'))),
            None
        );
        assert_eq!(map.resolve(Context::Normal, &press(KeyCode::F(12))), None);
    }

    #[test]
    fn the_first_binding_that_matches_wins() {
        let mut map = KeyMap::defaults();
        map.bindings_mut(Context::Normal)
            .insert(0, Binding::new("j", Target::GoToTop).unwrap());
        assert_eq!(
            map.resolve(Context::Normal, &press(KeyCode::Char('j'))),
            Some(Target::GoToTop)
        );
    }

    #[test]
    fn the_two_contexts_keep_separate_bindings() {
        // The same key has to be allowed to mean different things in the two
        // places, or the picker could never have had the arrows to itself.
        let map = KeyMap::defaults();
        let down = press(KeyCode::Down);

        assert_eq!(map.resolve(Context::Normal, &down), Some(Target::MoveDown));
        assert_eq!(
            map.resolve(Context::Calendar, &down),
            Some(Target::CalendarNextWeek)
        );
    }

    #[test]
    fn every_default_binding_is_valid() {
        let map = KeyMap::defaults();
        let total = map.all().count();
        assert_eq!(
            total,
            Context::ALL
                .iter()
                .map(|context| map.bindings(*context).len())
                .sum::<usize>(),
            "every binding belongs to exactly one section"
        );
        assert!(total > 0);
    }

    #[test]
    fn no_default_binding_shadows_an_earlier_one() {
        // Two bindings on the same key in one context means the second can
        // never fire, which is a mistake rather than a decision.
        let map = KeyMap::defaults();
        for context in Context::ALL {
            let list = map.bindings(context);
            for (i, first) in list.iter().enumerate() {
                for second in list.iter().skip(i + 1) {
                    for key in first.keys.iter() {
                        assert!(
                            !second.keys.contains(key),
                            "in {} both {:?} and {:?} claim {key:?}",
                            context.as_str(),
                            first.target,
                            second.target
                        );
                    }
                }
            }
        }
    }

    // ── typed characters ───────────────────────────────────────────────

    #[test]
    fn a_letter_types_itself() {
        assert_eq!(typed_char(&press(KeyCode::Char('a'))), Some('a'));
        assert_eq!(typed_char(&press(KeyCode::Char('7'))), Some('7'));
    }

    #[test]
    fn an_uppercase_letter_types_the_capital() {
        assert_eq!(
            typed_char(&modified(KeyCode::Char('A'), KeyModifiers::SHIFT)),
            Some('A')
        );
    }

    #[test]
    fn a_space_types_a_space() {
        assert_eq!(typed_char(&press(KeyCode::Char(' '))), Some(' '));
    }

    #[test]
    fn a_control_key_types_nothing() {
        assert_eq!(
            typed_char(&modified(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            None
        );
    }

    #[test]
    fn an_alt_key_types_nothing() {
        assert_eq!(
            typed_char(&modified(KeyCode::Char('f'), KeyModifiers::ALT)),
            None
        );
    }

    #[test]
    fn a_navigation_key_types_nothing() {
        for code in [
            KeyCode::Enter,
            KeyCode::Esc,
            KeyCode::Tab,
            KeyCode::BackTab,
            KeyCode::Backspace,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Left,
            KeyCode::Right,
        ] {
            assert_eq!(typed_char(&press(code)), None, "{code:?}");
        }
    }
}

//! Where the application's events come from.
//!
//! Two sources feed the loop: the terminal and the clock. Both are turned into
//! an [`AppEvent`] here so that everything above this file deals in one type,
//! and so that the terminal's noise — key releases, mouse movement, focus
//! changes — is dropped at the edge instead of being matched against later.
//!
//! Keys stay [`KeyEvent`]s at this level rather than becoming actions. Turning
//! a key into an action needs the keymap and the current context, and that is
//! the event loop's business, not the reader's.

use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyEvent, KeyEventKind};
use futures_util::{FutureExt, Stream, StreamExt};
use tokio::time::{Interval, MissedTickBehavior};

/// Something that happened while the application was waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    /// A key was pressed.
    Key(KeyEvent),
    /// The terminal changed size.
    Resize,
    /// Time to refresh from the daemon.
    Tick,
}

impl AppEvent {
    /// Read a terminal event, or `None` if there is nothing in it for us.
    ///
    /// Key releases are dropped. Terminals that tell a press from a release
    /// report both, and acting on the release as well would fire every binding
    /// twice.
    pub fn from_terminal(event: Event) -> Option<AppEvent> {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => Some(AppEvent::Key(key)),
            Event::Resize(_, _) => Some(AppEvent::Resize),
            _ => None,
        }
    }
}

/// The keyboard stopped working part way through.
///
/// Kept apart from the end of input, which is not a failure at all: a user who
/// closes the terminal's input is quitting, and the loop should be able to say
/// so rather than complain.
#[derive(Debug)]
pub struct EventError {
    message: String,
}

impl std::fmt::Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the keyboard stopped working: {}", self.message)
    }
}

impl std::error::Error for EventError {}

/// The clock half of the event source.
#[derive(Debug)]
pub struct Ticker {
    interval: Interval,
}

impl Ticker {
    /// A ticker that fires every `period`.
    ///
    /// The first fire is a whole period away rather than immediate. That is
    /// the opposite of what `tokio::time::interval` does, and it is deliberate:
    /// an immediate first fire would make the application ask for its list
    /// before it has finished starting up, and would make the timing of
    /// everything after it depend on that accident. Whoever starts the
    /// application asks for the first list themselves, once, on purpose.
    ///
    /// A missed tick is skipped rather than made up. If the application was
    /// busy for a second and the period is 200ms, five refreshes would be owed;
    /// running them back to back would ask the daemon for the same list five
    /// times in a row and still be out of date the moment it finished. One
    /// refresh afterwards is what was actually wanted.
    pub fn new(period: Duration) -> Self {
        let start = tokio::time::Instant::now() + period;
        let mut interval = tokio::time::interval_at(start, period);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        Self { interval }
    }

    /// Wait for the next tick.
    pub async fn tick(&mut self) {
        self.interval.tick().await;
    }

    /// The period this ticker was built with.
    pub fn period(&self) -> Duration {
        self.interval.period()
    }
}

/// The terminal and the clock together.
///
/// Generic over where the terminal events come from, because a real
/// [`EventStream`] cannot be read without a terminal and would otherwise leave
/// this loop — the part where a tick and a keypress race — untested.
#[derive(Debug)]
pub struct Events<S = EventStream> {
    terminal: S,
    ticker: Ticker,
}

impl Events<EventStream> {
    /// Start reading the real keyboard and count time.
    pub fn new(period: Duration) -> Self {
        Self::with_stream(EventStream::new(), period)
    }
}

impl<S> Events<S>
where
    S: Stream<Item = Result<Event, std::io::Error>> + Unpin,
{
    /// Take events from `terminal` and a clock ticking every `period`.
    pub fn with_stream(terminal: S, period: Duration) -> Self {
        Self {
            terminal,
            ticker: Ticker::new(period),
        }
    }

    /// Wait for the next event.
    ///
    /// `Ok(None)` means the input ended, which is not an error: it is what
    /// closing the terminal's input looks like from in here. Events with
    /// nothing in them for us are skipped rather than reported, so a key
    /// release does not look like the end of input.
    pub async fn next(&mut self) -> Result<Option<AppEvent>, EventError> {
        loop {
            tokio::select! {
                from_terminal = self.terminal.next() => match from_terminal {
                    Some(Ok(event)) => {
                        if let Some(event) = AppEvent::from_terminal(event) {
                            // println!("Has an event with {:?}",event);
                            return Ok(Some(event));
                        }
                    }
                    Some(Err(error)) => {
                        return Err(EventError {
                            message: error.to_string(),
                        });
                    }
                    None => {
                        // The keyboard is finished, but a tick that has come due
                        // while it was still working is still owed: end of input
                        // means nothing more will arrive, not that what has
                        // already arrived should be thrown away. Whether the
                        // tick is due is asked without waiting, so a quiet
                        // clock still means the end of input and not a hang.
                        return match self.ticker.tick().now_or_never() {
                            Some(()) => Ok(Some(AppEvent::Tick)),
                            None => Ok(None),
                        };
                    }
                },
                _ = self.ticker.tick() => return Ok(Some(AppEvent::Tick)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{
        KeyCode, KeyEvent, KeyEventState, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use futures_util::stream;
    use std::io;

    use super::*;

    /// Long enough that the clock never wins, so a test about keys is about
    /// keys alone.
    const NEVER: Duration = Duration::from_secs(3600);

    fn key_event(code: KeyCode, kind: KeyEventKind) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind,
            state: KeyEventState::NONE,
        })
    }

    fn press(code: KeyCode) -> Event {
        key_event(code, KeyEventKind::Press)
    }

    /// The same key press as the event loop will see it.
    fn pressed(code: KeyCode) -> AppEvent {
        AppEvent::Key(match press(code) {
            Event::Key(key) => key,
            other => panic!("a key press is not {other:?}"),
        })
    }

    /// A terminal that has already said everything it is going to say.
    fn said(events: Vec<Event>) -> impl Stream<Item = Result<Event, io::Error>> + Unpin {
        stream::iter(events.into_iter().map(Ok))
    }

    /// A terminal with nothing to say and no end in sight.
    fn quiet() -> impl Stream<Item = Result<Event, io::Error>> + Unpin {
        stream::pending()
    }

    // ── reading a terminal event ───────────────────────────────────────

    #[test]
    fn a_pressed_key_is_an_event() {
        let event = AppEvent::from_terminal(press(KeyCode::Char('j')));
        assert!(matches!(event, Some(AppEvent::Key(key)) if key.code == KeyCode::Char('j')));
    }

    #[test]
    fn a_released_key_is_not() {
        // Acting on releases as well as presses would run every binding twice.
        assert_eq!(
            AppEvent::from_terminal(key_event(KeyCode::Char('j'), KeyEventKind::Release)),
            None
        );
    }

    #[test]
    fn a_held_key_repeating_is_not_pressed_again() {
        // A repeat is the terminal saying "still down", not a new press, and
        // treating it as one would scroll a list as fast as the key repeats.
        assert_eq!(
            AppEvent::from_terminal(key_event(KeyCode::Down, KeyEventKind::Repeat)),
            None
        );
    }

    #[test]
    fn a_resize_is_an_event() {
        assert_eq!(
            AppEvent::from_terminal(Event::Resize(80, 24)),
            Some(AppEvent::Resize)
        );
    }

    #[test]
    fn the_size_in_a_resize_does_not_matter_here() {
        // Working out what still fits is the renderer's business.
        assert_eq!(
            AppEvent::from_terminal(Event::Resize(1, 1)),
            Some(AppEvent::Resize)
        );
        assert_eq!(
            AppEvent::from_terminal(Event::Resize(9999, 9999)),
            Some(AppEvent::Resize)
        );
    }

    #[test]
    fn a_key_arrives_with_its_modifiers_intact() {
        // The keymap tells ctrl-c from c by exactly these flags, so they have
        // to survive the trip.
        let event = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        let Some(AppEvent::Key(key)) = AppEvent::from_terminal(event) else {
            panic!("a pressed key should be an event");
        };
        assert!(key.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn mouse_events_are_dropped() {
        // Mouse reporting is never switched on, so these can only turn up as
        // noise from some other program having left it on.
        for kind in [
            MouseEventKind::Moved,
            MouseEventKind::Down(MouseButton::Left),
        ] {
            let event = Event::Mouse(MouseEvent {
                kind,
                column: 3,
                row: 4,
                modifiers: KeyModifiers::NONE,
            });
            assert_eq!(AppEvent::from_terminal(event), None);
        }
    }

    #[test]
    fn focus_events_are_dropped() {
        // Losing focus is not a reason to redraw, and gaining it is not a key.
        assert_eq!(AppEvent::from_terminal(Event::FocusGained), None);
        assert_eq!(AppEvent::from_terminal(Event::FocusLost), None);
    }

    // ── keys arriving ──────────────────────────────────────────────────

    #[tokio::test(start_paused = true)]
    async fn a_typed_key_arrives() {
        let mut events = Events::with_stream(said(vec![press(KeyCode::Char('q'))]), NEVER);

        let event = events.next().await.unwrap();

        assert!(matches!(event, Some(AppEvent::Key(key)) if key.code == KeyCode::Char('q')));
    }

    #[tokio::test(start_paused = true)]
    async fn keys_arrive_in_the_order_they_were_typed() {
        let typed = vec![
            press(KeyCode::Char('j')),
            press(KeyCode::Char('k')),
            press(KeyCode::Enter),
        ];
        let mut events = Events::with_stream(said(typed), NEVER);

        for expected in [KeyCode::Char('j'), KeyCode::Char('k'), KeyCode::Enter] {
            let event = events.next().await.unwrap();
            assert!(matches!(event, Some(AppEvent::Key(key)) if key.code == expected));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_resize_arrives_like_any_other_event() {
        let mut events = Events::with_stream(said(vec![Event::Resize(100, 40)]), NEVER);

        assert_eq!(events.next().await.unwrap(), Some(AppEvent::Resize));
    }

    #[tokio::test(start_paused = true)]
    async fn a_dropped_event_does_not_stop_the_next_one() {
        // A key release must not be mistaken for the end of input, which would
        // take the whole application down with it on any terminal that reports
        // releases.
        let typed = vec![
            key_event(KeyCode::Char('a'), KeyEventKind::Release),
            press(KeyCode::Char('b')),
        ];
        let mut events = Events::with_stream(said(typed), NEVER);

        let event = events.next().await.unwrap();

        assert!(matches!(event, Some(AppEvent::Key(key)) if key.code == KeyCode::Char('b')));
    }

    // ── the end of input ───────────────────────────────────────────────

    #[tokio::test(start_paused = true)]
    async fn the_end_of_input_is_not_an_error() {
        let mut events = Events::with_stream(said(vec![]), NEVER);

        assert_eq!(events.next().await.unwrap(), None);
    }

    #[tokio::test(start_paused = true)]
    async fn a_stream_of_only_dropped_events_still_ends_at_the_end() {
        let noise = vec![
            key_event(KeyCode::Char('a'), KeyEventKind::Release),
            Event::FocusGained,
            Event::FocusLost,
        ];
        let mut events = Events::with_stream(said(noise), NEVER);

        assert_eq!(events.next().await.unwrap(), None);
    }

    #[tokio::test(start_paused = true)]
    async fn a_run_of_dropped_events_keeps_waiting_rather_than_ending() {
        // This is the bug worth being certain about: reporting a dropped event
        // as the end of input would quit the application on the first release.
        let noise = vec![
            Event::FocusGained,
            key_event(KeyCode::Char('a'), KeyEventKind::Release),
            Event::FocusLost,
            key_event(KeyCode::Down, KeyEventKind::Repeat),
        ];
        let mut events = Events::with_stream(said(noise), NEVER);

        let event = tokio::time::timeout(Duration::from_millis(10), events.next())
            .await
            .expect("noise should be skipped, not reported")
            .unwrap();

        assert_eq!(event, None, "nothing left to report");
    }

    #[tokio::test(start_paused = true)]
    async fn the_end_of_input_arrives_after_the_last_key() {
        let mut events = Events::with_stream(said(vec![press(KeyCode::Char('q'))]), NEVER);

        assert!(events.next().await.unwrap().is_some());
        assert_eq!(events.next().await.unwrap(), None);
    }

    #[tokio::test(start_paused = true)]
    async fn a_read_failure_is_reported() {
        let broken = stream::iter(vec![Err(io::Error::new(io::ErrorKind::BrokenPipe, "gone"))]);
        let mut events = Events::with_stream(broken, NEVER);

        let error = events.next().await.unwrap_err();

        assert!(error.to_string().contains("gone"), "{error}");
    }

    // ── the clock ──────────────────────────────────────────────────────

    #[tokio::test(start_paused = true)]
    async fn the_first_tick_waits_a_whole_period() {
        let mut ticker = Ticker::new(Duration::from_secs(3));

        assert!(
            tokio::time::timeout(Duration::from_millis(2900), ticker.tick())
                .await
                .is_err(),
            "nothing should arrive early"
        );
        tokio::time::timeout(Duration::from_millis(200), ticker.tick())
            .await
            .expect("and then a tick at three seconds");
    }

    #[tokio::test(start_paused = true)]
    async fn the_period_is_remembered() {
        assert_eq!(
            Ticker::new(Duration::from_millis(750)).period(),
            Duration::from_millis(750)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_ticker_keeps_firing() {
        let mut ticker = Ticker::new(Duration::from_millis(100));
        for _ in 0..5 {
            tokio::time::timeout(Duration::from_millis(200), ticker.tick())
                .await
                .expect("each tick should arrive in turn");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_missed_tick_is_skipped_rather_than_made_up() {
        let mut ticker = Ticker::new(Duration::from_secs(1));

        // The application is busy for five whole periods.
        tokio::time::advance(Duration::from_secs(5)).await;
        ticker.tick().await;

        assert!(
            tokio::time::timeout(Duration::from_millis(10), ticker.tick())
                .await
                .is_err(),
            "the backlog should not be worked off in a burst"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_tick_arrives_when_nothing_is_typed() {
        let mut events = Events::with_stream(quiet(), Duration::from_secs(1));

        tokio::time::advance(Duration::from_millis(1100)).await;

        assert_eq!(events.next().await.unwrap(), Some(AppEvent::Tick));
    }

    #[tokio::test(start_paused = true)]
    async fn ticks_keep_arriving_while_nothing_is_typed() {
        let mut events = Events::with_stream(quiet(), Duration::from_millis(100));

        for _ in 0..3 {
            tokio::time::advance(Duration::from_millis(110)).await;
            assert_eq!(events.next().await.unwrap(), Some(AppEvent::Tick));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_tick_that_came_due_before_the_keyboard_closed_is_still_delivered() {
        // The clock and the end of input race, and the outcome should not depend
        // on which branch the runtime happened to look at first.
        for _ in 0..16 {
            let typed = said(vec![press(KeyCode::Char('j'))]);
            let mut events = Events::with_stream(typed, Duration::from_millis(100));

            tokio::time::advance(Duration::from_millis(110)).await;
            // Which of the two arrives first is up to the runtime; that neither
            // is lost, and that the loop still says when it is over, is not.
            let mut seen = vec![events.next().await.unwrap()];
            seen.push(events.next().await.unwrap());
            assert!(
                seen.contains(&Some(pressed(KeyCode::Char('j'))))
                    && seen.contains(&Some(AppEvent::Tick)),
                "a refresh that came due was lost on the way out: {seen:?}"
            );
            assert_eq!(events.next().await.unwrap(), None);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_quiet_clock_at_the_end_of_input_is_the_end_and_not_a_hang() {
        let typed = said(vec![press(KeyCode::Char('j'))]);
        let mut events = Events::with_stream(typed, NEVER);

        assert_eq!(
            events.next().await.unwrap(),
            Some(pressed(KeyCode::Char('j')))
        );
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), events.next())
                .await
                .expect("a quiet clock must not hang the loop")
                .unwrap(),
            None
        );
    }

    #[tokio::test(start_paused = true)]
    async fn keys_and_ticks_both_get_through() {
        // Whichever the loop happens to notice first, neither may be lost.
        let typed = said(vec![press(KeyCode::Char('j')), Event::Resize(80, 24)]);
        let mut events = Events::with_stream(typed, Duration::from_millis(100));

        // Whichever order the loop notices them in, none of the three may go
        // missing: the stream runs out after two events, so reading on past it
        // gives the end of input and nothing else.
        let mut seen = Vec::new();
        tokio::time::advance(Duration::from_millis(110)).await;
        for _ in 0..6 {
            let Some(event) = events.next().await.unwrap() else {
                break;
            };
            seen.push(event);
            let got_tick = seen.contains(&AppEvent::Tick);
            let got_key = seen.iter().any(|event| matches!(event, AppEvent::Key(_)));
            if got_tick && got_key && seen.contains(&AppEvent::Resize) {
                break;
            }
        }

        assert!(seen.contains(&AppEvent::Tick), "{seen:?}");
        assert!(
            seen.iter().any(|event| matches!(event, AppEvent::Key(_))),
            "{seen:?}"
        );
        assert!(seen.contains(&AppEvent::Resize), "{seen:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn a_key_never_waits_for_the_next_tick() {
        // The whole point of the clock is the refresh; a keystroke must not be
        // stuck behind it.
        let mut events = Events::with_stream(said(vec![press(KeyCode::Char('q'))]), NEVER);

        let event = tokio::time::timeout(Duration::from_millis(10), events.next())
            .await
            .expect("a key should arrive at once")
            .unwrap();

        assert!(matches!(event, Some(AppEvent::Key(_))));
    }

    // ── errors ─────────────────────────────────────────────────────────

    #[test]
    fn an_event_error_says_what_went_wrong() {
        let error = EventError {
            message: "broken pipe".to_string(),
        };
        assert!(error.to_string().contains("broken pipe"), "{error}");
    }
}

//! A status line for a step that takes a while, like a long `package.install`, so comtrya doesn't
//! look stuck: `⠋ CommandExec with: brew install git … 12s`. It's drawn on the terminal over and
//! over in place, and wiped before the next log line

use std::io::Write;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

/// How long a step runs before the status line shows up
const AFTER: Duration = Duration::from_secs(1);

/// How often the status line is drawn again
const TICK: Duration = Duration::from_millis(100);

/// The frames of the spinner
const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Labels longer than this are cut, so the line fits a terminal
const WIDTH: usize = 60;

/// Whether to show the status line: only on a terminal, and only where nothing else shows that
/// a step is working. `-v` prints the program's lines already, a dry run runs nothing, and with
/// several jobs one line can't stand for all the steps that run
pub(super) fn wanted(terminal: bool, verbose: u8, dry_run: bool, jobs: u16) -> bool {
    terminal && verbose == 0 && !dry_run && jobs == 1
}

/// The status line, and the thread that draws it
pub(super) struct Progress {
    /// What the drawing thread shares
    state: Arc<Mutex<State>>,
}

/// The step that runs, and whether the line is on the terminal now
struct State {
    /// Where the line is drawn
    out: Box<dyn Write + Send>,
    /// The step that runs, and since when
    running: Option<(String, Instant)>,
    /// Whether a line is drawn that has to be wiped
    shown: bool,
}

impl Progress {
    /// A status line on stdout, where the log goes
    pub(super) fn stdout() -> Self {
        Self::new(Box::new(std::io::stdout()), AFTER)
    }

    /// A status line on `out`, shown once a step ran for `after`
    fn new(out: Box<dyn Write + Send>, after: Duration) -> Self {
        let state = Arc::new(Mutex::new(State {
            out,
            running: None,
            shown: false,
        }));
        let weak = Arc::downgrade(&state);
        std::thread::spawn(move || draw(&weak, after));

        Self { state }
    }

    /// `label` starts to run
    pub(super) fn start(&self, label: &str) {
        let mut state = self.state();
        state.wipe();
        state.running = Some((cut(label), Instant::now()));
    }

    /// Nothing runs: the line is wiped, so the next log line starts on a clean line
    pub(super) fn stop(&self) {
        let mut state = self.state();
        state.wipe();
        state.running = None;
    }

    /// The shared state. A panic while it's locked only loses the status line
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl State {
    /// Removes the drawn line, if there is one
    fn wipe(&mut self) {
        if self.shown {
            let _ = write!(self.out, "\r\x1b[2K");
            let _ = self.out.flush();
            self.shown = false;
        }
    }

    /// Draws the line for the running step with spinner frame `frame`, once it ran for `after`
    fn draw(&mut self, frame: usize, after: Duration) {
        let Some((label, since)) = &self.running else {
            return;
        };
        let elapsed = since.elapsed();
        if elapsed < after {
            return;
        }

        let spinner = FRAMES[frame % FRAMES.len()];
        let line = format!("\r\x1b[2K{spinner} {label} … {}s", elapsed.as_secs());
        let _ = self.out.write_all(line.as_bytes());
        let _ = self.out.flush();
        self.shown = true;
    }
}

/// Draws the line again every [`TICK`], until the [`Progress`] is gone
fn draw(state: &Weak<Mutex<State>>, after: Duration) {
    for frame in 0.. {
        std::thread::sleep(TICK);
        let Some(state) = state.upgrade() else {
            return;
        };
        let mut state = state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.draw(frame, after);
    }
}

/// `label` on one line, at most [`WIDTH`] characters
fn cut(label: &str) -> String {
    let line = label.lines().next().unwrap_or_default();
    match line.char_indices().nth(WIDTH) {
        Some((end, _)) => format!("{}…", &line[..end]),
        None => line.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A writer the test can read back
    #[derive(Clone, Default)]
    struct Shared(Arc<Mutex<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Shared {
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).to_string()
        }
    }

    #[test]
    fn only_on_a_terminal_without_other_signs_of_life() {
        assert!(wanted(true, 0, false, 1));
        assert!(!wanted(false, 0, false, 1));
        assert!(!wanted(true, 1, false, 1));
        assert!(!wanted(true, 0, true, 1));
        assert!(!wanted(true, 0, false, 2));
    }

    #[test]
    fn a_long_step_shows_up_and_is_wiped_when_it_ends() {
        let out = Shared::default();
        let progress = Progress::new(Box::new(out.clone()), Duration::from_millis(50));

        progress.start("brew install git");
        std::thread::sleep(Duration::from_millis(400));
        progress.stop();

        let text = out.text();
        assert!(text.contains("brew install git … 0s"), "{text:?}");
        assert!(text.ends_with("\r\x1b[2K"), "{text:?}");
    }

    #[test]
    fn a_quick_step_shows_nothing() {
        let out = Shared::default();
        let progress = Progress::new(Box::new(out.clone()), Duration::from_secs(60));

        progress.start("mkdir");
        std::thread::sleep(Duration::from_millis(250));
        progress.stop();

        assert_eq!("", out.text());
    }

    #[test]
    fn long_labels_are_cut_to_one_short_line() {
        assert_eq!("a", cut("a\nb"));
        assert_eq!(
            format!("{}…", "x".repeat(WIDTH)),
            cut(&"x".repeat(WIDTH + 5))
        );
    }
}

// Horizon Essentials: the single global state (crate::game::essentials_state).
//
// One `State` per thread: the window's. In singleplayer, the integrated server
// runs on another thread and would have its own; the helpers only call the Core
// from the window (`native_window()`).
//
// Disk: the game thread never touches it. Loading and saving go through the
// "Essentials I/O" thread (essentials_store::Io). If that thread cannot start,
// the Core works without it (as before) rather than losing the settings.
//
// Asynchronous loading: until the configuration has been read, `loaded` is false,
// the HUD draws nothing and the modules receive nothing. A reload keeps the
// current configuration displayed until the new one arrives.
//
// Resilience: access goes through `with`, which refuses a nested access (a hook
// fired during another one) instead of panicking, which would stop the game.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::game::essentials_config::{Config, Resolved};
use crate::game::essentials_registry::{ModuleId, Modules};
use crate::game::essentials_store::{self as store, Io, Loaded, Reply, Request, SaveTimer};
use crate::game::essentials_types::Action;

/// At most this many messages waiting for the chat (the oldest ones are dropped).
const MAX_MESSAGES: usize = 16;
/// On shutdown, the time given to the I/O thread to finish writing.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

pub struct State {
    pub dir: PathBuf,
    pub profile: String,
    pub config: Config,
    /// The configuration ready for rendering, recomputed on every change.
    pub resolved: Resolved,
    /// False until the configuration has been read once.
    pub loaded: bool,
    /// Increases on every configuration change: widgets and the UI compare this
    /// number to know whether to rebuild their shapes.
    pub revision: u64,
    pub timer: SaveTimer,
    /// The messages to show in the chat, as soon as the player is in a world.
    pub messages: VecDeque<String>,
    /// The state of each module.
    pub modules: Modules,
    /// The ticks seen since loading.
    pub tick: u64,
    io: Option<Io>,
    /// The last save sent and the last one confirmed by the I/O thread.
    save_sent: u64,
    save_done: u64,
    reloading: bool,
}

impl State {
    fn empty(dir: &Path) -> State {
        let config = Config::default();
        State {
            dir: dir.to_owned(),
            profile: store::DEFAULT_PROFILE.to_owned(),
            resolved: config.resolve(),
            config,
            loaded: false,
            revision: 0,
            timer: SaveTimer::default(),
            messages: VecDeque::new(),
            modules: Modules::default(),
            tick: 0,
            io: None,
            save_sent: 0,
            save_done: 0,
            reloading: false,
        }
    }

    /// Starts: asks the I/O thread to read and returns at once.
    pub fn start(dir: &Path) -> State {
        let mut s = State::empty(dir);
        match Io::spawn() {
            Some(io) if io.send(Request::Load { dir: dir.to_owned() }).is_ok() => s.io = Some(io),
            _ => {
                // No I/O thread: direct read, only once.
                s.finish_load(store::load(dir));
            }
        }
        s
    }

    /// Direct read, without a thread (tests).
    pub fn load(dir: &Path) -> State {
        let mut s = State::empty(dir);
        s.finish_load(store::load(dir));
        s
    }

    fn finish_load(&mut self, loaded: Loaded) {
        self.profile = loaded.profile;
        self.config = loaded.config;
        for w in loaded.warnings {
            self.say(w);
        }
        self.loaded = true;
        self.changed();
        if std::mem::take(&mut self.reloading) {
            self.say("configuration reloaded");
        }
    }

    /// A message for the chat, prefixed with the mod name.
    pub fn say(&mut self, text: impl Into<String>) {
        if self.messages.len() == MAX_MESSAGES {
            self.messages.pop_front();
        }
        self.messages.push_back(format!("[Essentials] {}", text.into()));
    }

    /// A reply from the I/O thread is expected (read or write in progress).
    fn awaiting(&self) -> bool {
        !self.loaded || self.reloading || self.save_done < self.save_sent
    }

    /// Picks up the I/O thread's replies, without waiting. When nothing is expected
    /// (almost always), the channel is not even checked.
    pub fn poll(&mut self) {
        if !self.awaiting() {
            return;
        }
        while let Some(reply) = self.io.as_ref().and_then(Io::try_recv) {
            self.handle(reply);
        }
    }

    fn handle(&mut self, reply: Reply) {
        match reply {
            Reply::Loaded(loaded) => self.finish_load(loaded),
            Reply::Saved { seq, error } => {
                self.save_done = self.save_done.max(seq);
                if let Some(e) = error {
                    self.say(format!("could not save ({e}): settings kept in memory"));
                }
            }
            Reply::Stopped => {}
        }
    }

    /// Applies an action. True if something changed (or will change: reload).
    pub fn apply(&mut self, action: Action) -> bool {
        if !self.loaded {
            self.say("configuration is loading, try again in a moment");
            return false;
        }
        if action == Action::Reload {
            self.reload();
            return true;
        }
        if !self.config.apply(action) {
            return false;
        }
        self.changed();
        self.timer.mark();
        true
    }

    /// Reads the disk again, after sending any pending change (the I/O thread
    /// handles requests in order: the write comes before the read). The current
    /// configuration stays displayed until the new one arrives.
    pub fn reload(&mut self) {
        self.flush();
        self.reloading = true;
        let sent = self.io.as_ref().is_some_and(|io| io.send(Request::Load { dir: self.dir.clone() }).is_ok());
        if !sent {
            self.io = None;
            let loaded = store::load(&self.dir);
            self.finish_load(loaded);
        }
    }

    fn changed(&mut self) {
        let before = self.resolved.on;
        self.resolved = self.config.resolve();
        self.revision += 1;
        // Notifies every module whose enabled state changed.
        for id in ModuleId::all() {
            let on = self.resolved.on[id.index()];
            if on != before[id.index()] {
                self.modules.get_mut(id).on_enabled(on);
            }
        }
    }

    /// Time passes: picks up the I/O thread's replies, and sends the save once the
    /// configuration has not changed for one second.
    pub fn advance(&mut self, elapsed_ms: f32) {
        self.poll();
        if self.loaded && self.timer.advance(elapsed_ms) {
            self.flush();
        }
    }

    /// Sends the pending change to the I/O thread now. The JSON is prepared here
    /// (a few µs), the write happens on the I/O thread.
    pub fn flush(&mut self) {
        if !self.timer.pending() {
            return;
        }
        self.timer.done();
        let json = self.config.to_json();
        self.save_sent += 1;
        let request = Request::Save {
            dir: self.dir.clone(),
            profile: self.profile.clone(),
            json,
            seq: self.save_sent,
        };
        let refused = match &self.io {
            Some(io) => io.send(request).err(),
            None => Some(request),
        };
        if let Some(Request::Save { json, seq, .. }) = refused {
            // No I/O thread any more: direct write rather than losing the settings.
            self.io = None;
            if let Err(e) = store::save_json(&self.dir, &self.profile, &json) {
                self.say(format!("could not save ({e}): settings kept in memory"));
            }
            self.save_done = seq;
        }
    }

    /// Game shutdown: sends the last change and waits until the I/O thread has
    /// written everything (at most `SHUTDOWN_TIMEOUT`).
    pub fn shutdown(&mut self) {
        self.flush();
        if let Some(io) = self.io.take() {
            for reply in io.finish(SHUTDOWN_TIMEOUT) {
                self.handle(reply);
            }
        }
    }

    /// True when nothing is waiting on the disk: configuration read (and read again
    /// if a reload is in progress) and saves confirmed.
    pub fn idle(&self) -> bool {
        self.loaded && !self.reloading && !self.timer.pending() && self.save_done >= self.save_sent
    }

    /// Waits until nothing is waiting on the disk (tests and benchmarks; never in game).
    pub fn wait_idle(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while !self.idle() {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            match self.io.as_ref().and_then(|io| io.recv_timeout(left)) {
                Some(reply) => self.handle(reply),
                None => return self.idle(),
            }
        }
        true
    }

    /// The next message for the chat.
    pub fn next_message(&mut self) -> Option<String> {
        self.messages.pop_front()
    }
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Gives the state to `f`, starting it the first time (read requested from the I/O
/// thread, from the folder `dir` gives). None if the state is already borrowed
/// (nested access): the caller does nothing rather than crash the game.
pub fn with<R>(dir: impl FnOnce() -> PathBuf, f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|cell| {
        let mut slot = cell.try_borrow_mut().ok()?;
        let state = slot.get_or_insert_with(|| State::start(&dir()));
        Some(f(state))
    })
}

/// The state if it already exists, loaded or not (never created here).
pub fn with_existing<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|cell| {
        let mut slot = cell.try_borrow_mut().ok()?;
        slot.as_mut().map(f)
    })
}

/// The state if it is loaded: for rendering, which never starts anything.
pub fn with_loaded<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|cell| {
        let mut slot = cell.try_borrow_mut().ok()?;
        slot.as_mut().filter(|s| s.loaded).map(f)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::essentials_store::tests::TempDir;
    use crate::game::essentials_store::{DEFAULT_PROFILE, profile_path};

    const WAIT: Duration = Duration::from_secs(5);

    fn started(d: &TempDir) -> State {
        let mut s = State::start(&d.0);
        assert!(s.wait_idle(WAIT), "asynchronous loading");
        s
    }

    #[test]
    fn loading_is_asynchronous_and_the_hud_waits() {
        let d = TempDir::new("state-async");
        let mut s = State::start(&d.0);
        // Right after starting, nothing may have been read yet: nothing is drawn,
        // and an action is cleanly refused.
        if !s.loaded {
            assert!(!s.apply(Action::Toggle(ModuleId::Status)));
            assert!(s.next_message().unwrap().contains("is loading"));
        }
        assert!(s.wait_idle(WAIT));
        assert!(s.loaded);
    }

    #[test]
    fn an_action_is_saved_after_one_quiet_second() {
        let d = TempDir::new("state-save");
        let mut s = started(&d);
        let on = s.resolved.on[ModuleId::Status.index()];
        assert!(s.apply(Action::Toggle(ModuleId::Status)));
        assert_eq!(s.resolved.on[ModuleId::Status.index()], !on, "Resolved follows the configuration");
        s.advance(400.0);
        assert!(!profile_path(&d.0, DEFAULT_PROFILE).exists(), "not yet");
        s.advance(700.0);
        assert!(s.wait_idle(WAIT));
        let saved = store::load(&d.0);
        assert_eq!(saved.config.module(ModuleId::Status).on, !on);
    }

    #[test]
    fn a_save_during_another_one_is_not_lost() {
        let d = TempDir::new("state-burst");
        let mut s = started(&d);
        // 50 changes sent without waiting: the last state must reach the disk.
        for _ in 0..50 {
            s.apply(Action::Toggle(ModuleId::Status));
            s.flush();
        }
        let expected = s.config.clone();
        assert!(s.wait_idle(WAIT));
        assert_eq!(store::load(&d.0).config, expected);
    }

    #[test]
    fn shutdown_writes_the_last_change() {
        let d = TempDir::new("state-shutdown");
        let mut s = started(&d);
        s.apply(Action::Toggle(ModuleId::Status));
        // Shut down right away, without waiting for the quiet second.
        s.shutdown();
        assert_eq!(store::load(&d.0).config, s.config);
    }

    #[test]
    fn an_action_without_effect_saves_nothing() {
        let d = TempDir::new("state-no-effect");
        let mut s = started(&d);
        let on = s.resolved.on[ModuleId::Status.index()];
        let revision = s.revision;
        assert!(!s.apply(Action::Set(ModuleId::Status, on)));
        assert_eq!(s.revision, revision);
        s.advance(5000.0);
        assert!(s.wait_idle(WAIT));
        assert!(!profile_path(&d.0, DEFAULT_PROFILE).exists());
    }

    #[test]
    fn reload_reads_the_disk_without_clearing_the_display() {
        let d = TempDir::new("state-reload");
        let mut s = started(&d);
        let mut other = Config::default();
        other.apply(Action::Toggle(ModuleId::Status));
        store::save(&d.0, DEFAULT_PROFILE, &other).unwrap();
        assert!(s.apply(Action::Reload));
        assert!(s.loaded, "the old configuration stays displayed while reading again");
        assert!(s.wait_idle(WAIT));
        assert_eq!(s.config, other);
        assert_eq!(s.next_message().as_deref(), Some("[Essentials] configuration reloaded"));
    }

    #[test]
    fn a_write_error_becomes_a_message() {
        let d = TempDir::new("state-error");
        let mut s = started(&d);
        // A file in place of the "profiles" folder: the write fails.
        std::fs::write(d.0.join("profiles"), "not a folder").unwrap();
        s.apply(Action::Toggle(ModuleId::Status));
        s.flush();
        assert!(s.wait_idle(WAIT));
        assert!(s.next_message().unwrap().contains("could not save"));
    }

    #[test]
    fn warnings_go_to_the_chat() {
        let d = TempDir::new("state-warnings");
        std::fs::write(d.0.join("active"), "Forbidden").unwrap();
        let mut s = started(&d);
        assert!(s.next_message().unwrap().starts_with("[Essentials] invalid active profile"));
        assert_eq!(s.next_message(), None);
        for i in 0..40 {
            s.say(format!("{i}"));
        }
        assert_eq!(s.messages.len(), MAX_MESSAGES, "the queue is bounded");
    }

    #[test]
    fn a_nested_access_is_refused_without_panicking() {
        let d = TempDir::new("state-nested");
        let dir = d.0.clone();
        let outer = with(
            || dir.clone(),
            |_| with_loaded(|_| ()).is_none() && with(|| d.0.clone(), |_| ()).is_none(),
        );
        assert_eq!(outer, Some(true));
        assert!(with_existing(|s| s.wait_idle(WAIT)).unwrap());
        assert!(with_loaded(|s| s.profile.clone()).is_some(), "loaded once, kept afterwards");
    }
}

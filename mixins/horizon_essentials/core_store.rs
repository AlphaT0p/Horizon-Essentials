// Horizon Essentials: disk storage (crate::game::essentials_store).
//
// Persistence: the folder comes from `native_data_dir`, inside the instance folder
// (horizon-mods/.horizon-mod-data/horizon_essentials/), never inside a version
// folder: settings survive rebuilds and updates.
//
//   active                  the name of the active profile
//   profiles/<name>.json    a profile (essentials_config format)
//
// Atomic write: temporary file, then rename, so that a crash never leaves a
// half-written file. A corrupted file is copied to `<name>.json.bak` before
// being replaced: nothing is lost.

use std::fs;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::game::essentials_config::Config;

pub const DEFAULT_PROFILE: &str = "default";
/// Saving waits for one second without changes (a drag in the HUD Editor makes
/// dozens of changes: only one file is written).
pub const SAVE_DELAY_MS: f32 = 1000.0;

/// What `load` found.
#[derive(Debug)]
pub struct Loaded {
    pub profile: String,
    pub config: Config,
    pub warnings: Vec<String>,
    /// The names of the profiles on disk, sorted (the loaded one included).
    pub profiles: Vec<String>,
}

/// A profile name that is safe as a file name: 1 to 32 characters `a-z`, `0-9`, `_`, `-`.
pub fn valid_profile_name(name: &str) -> bool {
    (1..=32).contains(&name.len())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// A valid profile name, kept by value so that an `Action` stays `Copy`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ProfileName {
    len: u8,
    bytes: [u8; 32],
}

impl ProfileName {
    /// None when `name` is not a valid profile name.
    pub fn new(name: &str) -> Option<ProfileName> {
        if !valid_profile_name(name) {
            return None;
        }
        let mut bytes = [0; 32];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        Some(ProfileName {
            len: name.len() as u8,
            bytes,
        })
    }

    pub fn as_str(&self) -> &str {
        // Valid names are ASCII.
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }
}

impl std::fmt::Debug for ProfileName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

pub fn profile_path(dir: &Path, profile: &str) -> PathBuf {
    dir.join("profiles").join(format!("{profile}.json"))
}

/// The profiles on disk: the valid names of `profiles/*.json`, sorted.
fn list_profiles(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir.join("profiles"))
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let stem = name.strip_suffix(".json")?;
                    valid_profile_name(stem).then(|| stem.to_owned())
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// Reads the active profile. Never panics and always returns a valid configuration.
pub fn load(dir: &Path) -> Loaded {
    load_profile(dir, None)
}

/// Reads profile `wanted`, or the active one when None. A profile with no file yet gives the
/// default configuration (it is created at its first save).
pub fn load_profile(dir: &Path, wanted: Option<&str>) -> Loaded {
    count_disk_op();
    let mut warnings = Vec::new();
    let profile = match wanted {
        Some(name) => name.to_owned(),
        None => match fs::read_to_string(dir.join("active")) {
            Ok(text) => {
                let name = text.trim();
                if valid_profile_name(name) {
                    name.to_owned()
                } else {
                    warnings.push(format!(
                        "invalid active profile ({name:?}): using profile \"{DEFAULT_PROFILE}\""
                    ));
                    DEFAULT_PROFILE.to_owned()
                }
            }
            Err(_) => DEFAULT_PROFILE.to_owned(),
        },
    };
    let path = profile_path(dir, &profile);
    let config = match fs::read_to_string(&path) {
        Ok(text) => {
            let (config, found) = Config::parse(&text);
            if !found.is_empty() {
                let backup = path.with_extension("json.bak");
                if fs::copy(&path, &backup).is_ok() {
                    warnings.push(format!("old file copied to profiles/{profile}.json.bak"));
                }
                warnings.splice(0..0, found);
            }
            config
        }
        // First launch: no file, no warning.
        Err(e) if e.kind() == io::ErrorKind::NotFound => Config::default(),
        Err(e) => {
            warnings.push(format!(
                "unreadable profile \"{profile}\" ({e}): default values"
            ));
            Config::default()
        }
    };
    let mut profiles = list_profiles(dir);
    if !profiles.contains(&profile) {
        profiles.push(profile.clone());
        profiles.sort();
    }
    Loaded {
        profile,
        config,
        warnings,
        profiles,
    }
}

/// Deletes profile `profile`'s file (its `.bak` copy, if any, stays). Refused for the active
/// profile, so `active` never names a missing file.
pub fn delete_profile(dir: &Path, profile: &str) -> io::Result<()> {
    count_disk_op();
    if !valid_profile_name(profile) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid profile name",
        ));
    }
    if fs::read_to_string(dir.join("active")).is_ok_and(|a| a.trim() == profile) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "it is the active profile",
        ));
    }
    fs::remove_file(profile_path(dir, profile))
}

/// Writes the profile, and `active` if it changed.
pub fn save(dir: &Path, profile: &str, config: &Config) -> io::Result<()> {
    save_json(dir, profile, &config.to_json())
}

/// Like `save`, with the JSON already prepared (by the game thread).
pub fn save_json(dir: &Path, profile: &str, json: &str) -> io::Result<()> {
    count_disk_op();
    write_atomic(&profile_path(dir, profile), json.as_bytes())?;
    let active = dir.join("active");
    if fs::read_to_string(&active)
        .map(|a| a.trim() != profile)
        .unwrap_or(true)
    {
        write_atomic(&active, profile.as_bytes())?;
    }
    Ok(())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, bytes)?;
    // On Windows as elsewhere, `rename` replaces the existing target.
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// The delay before saving, counted in frame time: frames keep running when
/// the escape menu pauses the game, unlike ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SaveTimer {
    pending: bool,
    quiet_ms: f32,
}

impl SaveTimer {
    /// A change has just happened.
    pub fn mark(&mut self) {
        self.pending = true;
        self.quiet_ms = 0.0;
    }

    /// Time passes. True when it is time to save.
    pub fn advance(&mut self, elapsed_ms: f32) -> bool {
        if !self.pending {
            return false;
        }
        self.quiet_ms += elapsed_ms.max(0.0);
        self.quiet_ms >= SAVE_DELAY_MS
    }

    pub fn done(&mut self) {
        *self = SaveTimer::default();
    }

    pub fn pending(&self) -> bool {
        self.pending
    }
}

// ── I/O thread ──────────────────────────────────────────────────────────────
//
// The disk is never touched by the game (render) thread: it sends requests to
// the "Essentials I/O" thread through a channel and picks up the replies without
// waiting. Nothing is shared: every request carries its data by value.
//
// - A request that arrives during a write waits in the channel: it is never lost.
// - Several pending saves of the same profile, with no load between them, only
//   write the last one (it contains every change).
// - A panic during an operation is caught and becomes an error.
// - On shutdown, `finish` completes the pending writes (with a maximum delay).

/// A request to the I/O thread.
#[derive(Debug)]
pub enum Request {
    /// Reads `profile`, or the active profile when None.
    Load {
        dir: PathBuf,
        profile: Option<String>,
    },
    Save {
        dir: PathBuf,
        profile: String,
        json: String,
        seq: u64,
    },
    Delete {
        dir: PathBuf,
        profile: String,
    },
    Stop,
}

/// A reply from the I/O thread.
#[derive(Debug)]
pub enum Reply {
    Loaded(Loaded),
    Saved {
        seq: u64,
        error: Option<String>,
    },
    Deleted {
        profile: String,
        error: Option<String>,
    },
    Stopped,
}

/// The "game" side of the I/O thread.
#[derive(Debug)]
pub struct Io {
    tx: Sender<Request>,
    rx: Receiver<Reply>,
    handle: Option<JoinHandle<()>>,
}

impl Io {
    /// Starts the thread. None if it cannot start: the caller then works without
    /// a thread (as before), instead of losing the settings.
    pub fn spawn() -> Option<Io> {
        let (tx, requests) = mpsc::channel();
        let (replies, rx) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("Essentials I/O".into())
            .spawn(move || worker(requests, replies))
            .ok()?;
        Some(Io {
            tx,
            rx,
            handle: Some(handle),
        })
    }

    /// Sends a request. If the thread is gone, the request is handed back
    /// (the caller can then handle it itself).
    pub fn send(&self, request: Request) -> Result<(), Request> {
        self.tx.send(request).map_err(|e| e.0)
    }

    /// The next reply that arrived, without waiting (allocates nothing if the channel is empty).
    pub fn try_recv(&self) -> Option<Reply> {
        self.rx.try_recv().ok()
    }

    /// Waits for a reply for at most `timeout`.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<Reply> {
        self.rx.recv_timeout(timeout).ok()
    }

    /// Shutdown: asks the thread to stop and returns the replies received until
    /// then (at most `timeout`). The thread is joined if it stopped in time;
    /// otherwise it is left to finish on its own (the process exits, and atomic
    /// writes guarantee that no file is left half-written).
    pub fn finish(mut self, timeout: Duration) -> Vec<Reply> {
        let mut replies = Vec::new();
        if self.send(Request::Stop).is_err() {
            return replies;
        }
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left) {
                Ok(Reply::Stopped) => {
                    if let Some(handle) = self.handle.take() {
                        let _ = handle.join();
                    }
                    break;
                }
                Ok(reply) => replies.push(reply),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        replies
    }
}

fn worker(requests: Receiver<Request>, replies: Sender<Reply>) {
    while let Ok(first) = requests.recv() {
        let mut batch = vec![first];
        batch.extend(requests.try_iter());
        for (i, request) in batch.iter().enumerate() {
            let reply = match request {
                Request::Stop => {
                    let _ = replies.send(Reply::Stopped);
                    return;
                }
                Request::Load { dir, profile } => {
                    let wanted = profile.as_deref();
                    let loaded = catch_unwind(AssertUnwindSafe(|| load_profile(dir, wanted)))
                        .unwrap_or_else(|_| {
                            let profile = wanted.unwrap_or(DEFAULT_PROFILE).to_owned();
                            Loaded {
                                profiles: vec![profile.clone()],
                                profile,
                                config: Config::default(),
                                warnings: vec![
                                    "internal error while reading: default values".to_owned(),
                                ],
                            }
                        });
                    Reply::Loaded(loaded)
                }
                Request::Delete { dir, profile } => {
                    let error =
                        match catch_unwind(AssertUnwindSafe(|| delete_profile(dir, profile))) {
                            Ok(Ok(())) => None,
                            Ok(Err(e)) => Some(e.to_string()),
                            Err(_) => Some("internal error while deleting".to_owned()),
                        };
                    Reply::Deleted {
                        profile: profile.clone(),
                        error,
                    }
                }
                Request::Save {
                    dir,
                    profile,
                    json,
                    seq,
                } => {
                    if superseded(&batch[i + 1..], dir, profile) {
                        Reply::Saved {
                            seq: *seq,
                            error: None,
                        }
                    } else {
                        let error = match catch_unwind(AssertUnwindSafe(|| {
                            save_json(dir, profile, json)
                        })) {
                            Ok(Ok(())) => None,
                            Ok(Err(e)) => Some(e.to_string()),
                            Err(_) => Some("internal error while writing".to_owned()),
                        };
                        Reply::Saved { seq: *seq, error }
                    }
                }
            };
            if replies.send(reply).is_err() {
                return;
            }
        }
    }
}

/// A newer save of the same profile follows, with no load or deletion in between.
fn superseded(later: &[Request], dir: &Path, profile: &str) -> bool {
    for r in later {
        match r {
            Request::Load { .. } | Request::Delete { .. } | Request::Stop => return false,
            Request::Save {
                dir: d, profile: p, ..
            } if d == dir && p == profile => return true,
            Request::Save { .. } => {}
        }
    }
    false
}

// Counts the current thread's disk operations, for tests (nothing in game).
#[cfg(test)]
thread_local! {
    pub(crate) static DISK_OPS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn count_disk_op() {
    DISK_OPS.with(|c| c.set(c.get() + 1));
}

#[cfg(not(test))]
fn count_disk_op() {}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::game::essentials_registry::ModuleId;
    use crate::game::essentials_types::Action;

    /// A temporary folder of its own for a test, deleted at the end.
    pub(crate) struct TempDir(pub PathBuf);
    impl TempDir {
        pub(crate) fn new(name: &str) -> TempDir {
            let dir = std::env::temp_dir()
                .join(format!("horizon-essentials-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn first_launch_without_a_file() {
        let d = TempDir::new("empty");
        let l = load(&d.0);
        assert_eq!(l.profile, DEFAULT_PROFILE);
        assert_eq!(l.config, Config::default());
        assert!(l.warnings.is_empty());
    }

    #[test]
    fn save_then_read_back() {
        let d = TempDir::new("round-trip");
        let mut c = Config::default();
        c.apply(Action::Toggle(ModuleId::Status));
        save(&d.0, "pvp", &c).unwrap();
        assert_eq!(fs::read_to_string(d.0.join("active")).unwrap(), "pvp");
        assert!(
            !profile_path(&d.0, "pvp")
                .with_extension("json.tmp")
                .exists(),
            "no leftover temporary file"
        );
        let l = load(&d.0);
        assert_eq!((l.profile.as_str(), &l.config), ("pvp", &c));
        assert!(l.warnings.is_empty());
    }

    #[test]
    fn a_corrupted_file_is_kept_as_bak() {
        let d = TempDir::new("corrupted");
        let path = profile_path(&d.0, DEFAULT_PROFILE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ this is not JSON").unwrap();
        let l = load(&d.0);
        assert_eq!(l.config, Config::default());
        assert_eq!(l.warnings.len(), 2, "{:?}", l.warnings);
        assert_eq!(
            fs::read_to_string(path.with_extension("json.bak")).unwrap(),
            "{ this is not JSON"
        );
        assert!(path.exists(), "the original is never deleted when reading");
    }

    #[test]
    fn a_dangerous_active_profile_is_rejected() {
        let d = TempDir::new("active");
        fs::write(d.0.join("active"), "../../elsewhere").unwrap();
        let l = load(&d.0);
        assert_eq!(l.profile, DEFAULT_PROFILE);
        assert_eq!(l.warnings.len(), 1);
        assert!(valid_profile_name("pvp_2"));
        assert!(!valid_profile_name(""));
        assert!(!valid_profile_name("Uppercase"));
    }

    #[test]
    fn the_io_thread_writes_the_latest_version_without_losing_anything() {
        let d = TempDir::new("io-burst");
        let io = Io::spawn().expect("I/O thread");
        let mut c = Config::default();
        for seq in 1..=100 {
            c.apply(Action::Toggle(ModuleId::Status));
            let save = Request::Save {
                dir: d.0.clone(),
                profile: "default".into(),
                json: c.to_json(),
                seq,
            };
            assert!(io.send(save).is_ok());
        }
        // A read after the writes sees the last one (order is kept).
        assert!(
            io.send(Request::Load {
                dir: d.0.clone(),
                profile: None
            })
            .is_ok()
        );
        let replies = io.finish(Duration::from_secs(5));
        let saved = replies
            .iter()
            .filter(|r| matches!(r, Reply::Saved { .. }))
            .count();
        assert_eq!(saved, 100, "every request gets its reply");
        assert!(
            replies
                .iter()
                .all(|r| !matches!(r, Reply::Saved { error: Some(_), .. }))
        );
        let Some(Reply::Loaded(l)) = replies.iter().find(|r| matches!(r, Reply::Loaded(_))) else {
            panic!("no read");
        };
        assert_eq!(l.config, c);
    }

    #[test]
    fn a_save_is_skipped_only_if_a_newer_one_follows_without_a_read() {
        let save = |p: &str| Request::Save {
            dir: PathBuf::from("d"),
            profile: p.into(),
            json: String::new(),
            seq: 0,
        };
        let load = Request::Load {
            dir: PathBuf::from("d"),
            profile: None,
        };
        let d = Path::new("d");
        assert!(superseded(&[save("default")], d, "default"));
        assert!(
            !superseded(&[save("pvp")], d, "default"),
            "another profile replaces nothing"
        );
        assert!(
            !superseded(&[load, save("default")], d, "default"),
            "a read in between"
        );
        assert!(!superseded(&[], d, "default"));
    }

    #[test]
    fn shutdown_waits_for_the_writes() {
        let d = TempDir::new("io-shutdown");
        let io = Io::spawn().expect("I/O thread");
        let mut c = Config::default();
        c.apply(Action::Toggle(ModuleId::Status));
        let save = Request::Save {
            dir: d.0.clone(),
            profile: "default".into(),
            json: c.to_json(),
            seq: 1,
        };
        assert!(io.send(save).is_ok());
        io.finish(Duration::from_secs(5));
        assert_eq!(load(&d.0).config, c, "written before `finish` returns");
    }

    #[test]
    fn the_save_delay() {
        let mut t = SaveTimer::default();
        assert!(!t.advance(5000.0), "nothing to save");
        t.mark();
        assert!(!t.advance(600.0));
        t.mark();
        assert!(!t.advance(600.0), "a new change restarts the delay");
        assert!(t.advance(500.0));
        t.done();
        assert!(!t.pending());
    }

    #[test]
    fn profile_names_are_checked_once_and_copied_by_value() {
        let name = ProfileName::new("pvp_2").unwrap();
        assert_eq!(name.as_str(), "pvp_2");
        let copy = name;
        assert_eq!(copy, name);
        assert!(ProfileName::new("").is_none());
        assert!(ProfileName::new("Uppercase").is_none());
        assert!(ProfileName::new("../x").is_none());
        assert!(ProfileName::new(&"a".repeat(33)).is_none());
        assert_eq!(
            ProfileName::new(&"a".repeat(32)).unwrap().as_str().len(),
            32
        );
    }

    #[test]
    fn loading_lists_the_profiles_and_reads_the_one_asked_for() {
        let d = TempDir::new("profiles");
        let mut pvp = Config::default();
        pvp.apply(Action::Toggle(ModuleId::Status));
        save(&d.0, "pvp", &pvp).unwrap();
        save(&d.0, "default", &Config::default()).unwrap();
        fs::write(d.0.join("profiles").join("Bad Name.json"), "{}").unwrap();
        let l = load_profile(&d.0, Some("pvp"));
        assert_eq!((l.profile.as_str(), &l.config), ("pvp", &pvp));
        assert_eq!(
            l.profiles,
            ["default", "pvp"],
            "invalid names are not profiles"
        );
        let new = load_profile(&d.0, Some("new"));
        assert_eq!(new.config, Config::default(), "no file yet: the defaults");
        assert_eq!(new.profiles, ["default", "new", "pvp"]);
        assert!(new.warnings.is_empty());
    }

    #[test]
    fn deleting_a_profile_spares_the_active_one() {
        let d = TempDir::new("delete");
        save(&d.0, "pvp", &Config::default()).unwrap();
        save(&d.0, "default", &Config::default()).unwrap(); // `default` is active now
        assert!(delete_profile(&d.0, "default").is_err());
        assert!(profile_path(&d.0, "default").exists());
        delete_profile(&d.0, "pvp").unwrap();
        assert!(!profile_path(&d.0, "pvp").exists());
        assert!(delete_profile(&d.0, "pvp").is_err(), "already gone");
        assert!(
            delete_profile(&d.0, "../active").is_err(),
            "never outside profiles/"
        );
    }
}

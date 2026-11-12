//! The launcher without its window, for programs that start it
//! (RecompLauncher, see `recomplauncher.json` at the repository's top and
//! the launcher README, "RecompLauncher"):
//!
//! - `--prepare --game <copy>`: what Home > Install the game does for a
//!   player who has nothing yet, or brings an install up to date: the copy
//!   checked (the BCUS98120 disc image or a folder with PS3_GAME/) and
//!   remembered as the disc, Apple's Command Line Tools, the install's own
//!   checkout of the sources pulled, then tools/install.sh (the build tools,
//!   toolkit, venv, decrypt, analyse, lift, SPU, build). It prints
//!   `progress <0..1> <text>` lines, the scripts' output as plain lines, and
//!   exits with one of the codes in `Exit`, 0 only when the game's build is
//!   there afterwards. Then, in the same run and with the bar going on, the
//!   online version (tools/build_online.sh: the game's 01.60 update from
//!   Sony, decrypted and built beside the campaign's). That part never fails
//!   the setup: the campaign is ready either way, the last progress line says
//!   whether online is too, and why not; what went wrong is kept in
//!   <data>/online-build.json for `--play-online` to say (its own exit codes).
//!   RES_NO_ONLINE=1 leaves it out.
//! - `--play` with `RECOMPLAUNCHER=1`: the game started at once (the same
//!   Play as the window's button: settings, save backup, log), no window; the
//!   launcher's process becomes the game's (exec), so its exit code and its
//!   lifetime are the game's. `--game <copy>` uses and remembers that disc.
//! - `--play-online` with `RECOMPLAUNCHER=1` (RecompLauncher's Online tile):
//!   the same for the window's Play online (online_run in lib.rs: the online
//!   ID, the 01.60 build, the online folder, online_env with its frame-rate
//!   guarantee), or the exit code that says what is missing.
//!
//! All of them decide in `run()` before any window or Tauri event loop
//! exists. The install itself is the window's (install_job, spawn_install,
//! pull_command in lib.rs); only where its output goes differs (OutputSink).

use crate::*;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Instant;

/// The headless prepare's and `--play`'s exit codes (as recomplauncher.json's
/// exit_codes says them to the player).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// Prepare: the game is built. Play: the game was quit normally.
    Ok,
    /// No copy given, not found, not the US disc BCUS98120, or the image
    /// would not mount.
    Copy,
    /// Something the player has to do first: Apple's Command Line Tools
    /// (their installer is opened), RPCS3 when the decrypt step needs it, or
    /// not a Mac.
    NeedsPlayer,
    /// The port's sources or a download the install makes (toolkit, Python
    /// packages, decryption keys) failed: the connection.
    Sources,
    /// The install or the build failed (install.log has why).
    Build,
    /// Another prepare, an install in the same checkout, or the game itself
    /// is running.
    Running,
    /// Play: the game is not built yet.
    NotBuilt,
    /// The launcher failed on its own (its settings, starting a program).
    Failed,
    /// Play online: no online ID set, or not a valid one.
    OnlineId,
    /// Play online: the 01.60 build (or the update's files) is not there,
    /// and no setup said why.
    OnlineNotBuilt,
    /// Play online: the last setup could not download what the online
    /// version needs (Sony's update packages, the decryption keys).
    OnlineDownload,
    /// Play online: the last setup could not build the online version.
    OnlineBuild,
    /// Play online: the campaign has not done its first boot's HDD install,
    /// which the online folder is made from.
    OnlineInstall,
}

impl Exit {
    pub const ALL: [Exit; 13] = [
        Exit::Ok,
        Exit::Copy,
        Exit::NeedsPlayer,
        Exit::Sources,
        Exit::Build,
        Exit::Running,
        Exit::NotBuilt,
        Exit::Failed,
        Exit::OnlineId,
        Exit::OnlineNotBuilt,
        Exit::OnlineDownload,
        Exit::OnlineBuild,
        Exit::OnlineInstall,
    ];

    pub fn code(self) -> i32 {
        match self {
            Exit::Ok => 0,
            Exit::Copy => 40,
            Exit::NeedsPlayer => 41,
            Exit::Sources => 42,
            Exit::Build => 43,
            Exit::Running => 44,
            Exit::NotBuilt => 45,
            Exit::Failed => 46,
            Exit::OnlineId => 47,
            Exit::OnlineNotBuilt => 48,
            Exit::OnlineDownload => 49,
            Exit::OnlineBuild => 50,
            Exit::OnlineInstall => 51,
        }
    }

    fn from_code(c: i32) -> Option<Exit> {
        Exit::ALL.into_iter().find(|e| e.code() == c)
    }

    /// What the player is told (recomplauncher.json's exit_codes; "ok" for
    /// a normal end).
    pub fn says(self) -> &'static str {
        match self {
            Exit::Ok => "ok",
            Exit::Copy => "Your copy isn't the one this port reads: it needs your own Resistance 2 US disc, BCUS98120 version 1.00, as a disc image (.iso) or a folder with PS3_GAME and PS3_DISC.SFB. Pick it again on the game's page.",
            Exit::NeedsPlayer => "Your Mac needs something first: Apple's Command Line Tools (their installer opened: finish it), or RPCS3 when it says so. Then set the game up again.",
            Exit::Sources => "Couldn't download what the build needs (the port's sources, the toolkit or its Python packages). Check the internet connection and set the game up again.",
            Exit::Build => "Building the game failed. Check there are about 6 GB free, then set it up again: it continues where it stopped. install.log in the logs folder says why.",
            Exit::Running => "Resistance 2 is already running, or another setup of it is. Close it or wait for it, then try again.",
            Exit::NotBuilt => "The game isn't built yet. Set it up again on the game's page (the first time takes 30 to 45 minutes).",
            Exit::Failed => "The Resistance 2 launcher couldn't start it. Open the launcher itself to see why.",
            Exit::OnlineId => "Online play needs your online ID, the name you go by on PS Rewired (3 to 16 letters, digits, - or _, starting with a letter). Set it once in the Resistance 2 Launcher window, under Home > Online: the Resistance 2 Launcher button on the game's page opens it. Then start online again.",
            Exit::OnlineNotBuilt => "The online version (the game's 01.60 update) isn't built yet. Set the game up again on its page: the setup builds it after the campaign (about 30 minutes and 4 GB more).",
            Exit::OnlineDownload => "Couldn't download what the online version needs (the game's official 01.60 update from Sony, or its decryption keys). The campaign plays as it is. Check the internet connection, then set the game up again on its page.",
            Exit::OnlineBuild => "Building the online version failed; the campaign plays as it is. Check there are about 4 GB free, then set the game up again on its page: it continues where it stopped. install-online.log in the logs folder says why.",
            Exit::OnlineInstall => "Play the campaign once first: its first start copies the game's data from your disc, and online play starts from that copy. Then start online again.",
        }
    }
}

// ---------------------------------------------------------------------------
// The command line
// ---------------------------------------------------------------------------

pub const PREPARE_ARG: &str = "--prepare";
pub const GAME_ARG: &str = "--game";

#[derive(Debug, PartialEq)]
pub enum Mode {
    /// `--prepare [--game <copy>]`: the headless install.
    Prepare { game: Option<PathBuf> },
    /// `--play [--game <copy>]` started by RecompLauncher: the game, no window.
    PlayDirect { game: Option<PathBuf> },
    /// `--play-online [--game <copy>]` started by RecompLauncher: online
    /// play, no window (it wins over `--play`: the Online tile's arguments
    /// come after start.args).
    PlayOnlineDirect { game: Option<PathBuf> },
    /// The window (with `--play`: Play once it is up; with `--play-online`:
    /// Play online).
    Window { game: Option<PathBuf> },
}

/// `--game <path>` or `--game=<path>`.
pub fn game_arg(args: &[String]) -> Option<PathBuf> {
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        if let Some(v) = a.strip_prefix("--game=") {
            return Some(PathBuf::from(v)).filter(|p| !p.as_os_str().is_empty());
        }
        if a == GAME_ARG {
            return it.next().filter(|v| !v.is_empty()).map(PathBuf::from);
        }
    }
    None
}

/// What the command line (argv, program first) asks for; `recomplauncher`:
/// RECOMPLAUNCHER=1 is set (under_recomplauncher).
pub fn mode(args: &[String], recomplauncher: bool) -> Mode {
    let game = game_arg(args);
    if args.iter().skip(1).any(|a| a == PREPARE_ARG) {
        Mode::Prepare { game }
    } else if recomplauncher && play_shortcut::online_requested(args) {
        Mode::PlayOnlineDirect { game }
    } else if recomplauncher && play_shortcut::requested(args.iter().cloned()) {
        Mode::PlayDirect { game }
    } else {
        Mode::Window { game }
    }
}

/// RecompLauncher sets RECOMPLAUNCHER=1 for every game it starts.
pub fn under_recomplauncher() -> bool {
    std::env::var("RECOMPLAUNCHER").is_ok_and(|v| v.trim() == "1")
}

// ---------------------------------------------------------------------------
// The player's copy
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum GameCopy {
    /// A folder with PS3_GAME/ (its root).
    Folder(PathBuf),
    /// A disc image (.iso).
    Image(PathBuf),
}

/// A disc ID as written in PS3_DISC.SFB or PARAM.SFO, compared without the
/// dash: "BCUS-98120" and "BCUS98120" are the same.
fn id_key(v: &str) -> String {
    v.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_uppercase()
}

/// The TITLE_ID in a PS3_DISC.SFB: ".SFB", then 0x20-byte entries from 0x20
/// (a 16-byte name, the value's offset and length, big-endian).
pub fn sfb_title_id(d: &[u8]) -> Option<String> {
    if d.len() < 0x40 || &d[..4] != b".SFB" {
        return None;
    }
    let be = |o: usize| u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]) as usize;
    let mut at = 0x20;
    while at + 0x20 <= d.len().min(0x200) {
        let name = &d[at..at + 16];
        if name.split(|&b| b == 0).next() == Some(b"TITLE_ID") {
            let (off, len) = (be(at + 16), be(at + 20));
            let v = d.get(off..off.checked_add(len)?)?;
            return Some(String::from_utf8_lossy(v.split(|&b| b == 0).next().unwrap_or(&[])).trim().to_string());
        }
        at += 0x20;
    }
    None
}

/// A file or folder in an ISO 9660 directory.
struct IsoEntry {
    name: String,
    lba: u64,
    size: u64,
    dir: bool,
}

fn read_exact_at(f: &mut fs::File, at: u64, len: usize) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; len];
    f.seek(SeekFrom::Start(at)).ok()?;
    f.read_exact(&mut buf).ok()?;
    Some(buf)
}

/// An ISO 9660 image's block size and root directory (its primary volume
/// descriptor, sector 16), when it is one.
fn iso_root(f: &mut fs::File) -> Option<(u64, u64, u64)> {
    let pvd = read_exact_at(f, 16 * 2048, 2048)?;
    if pvd[0] != 1 || &pvd[1..6] != b"CD001" {
        return None;
    }
    let le32 = |o: usize| u32::from_le_bytes([pvd[o], pvd[o + 1], pvd[o + 2], pvd[o + 3]]) as u64;
    let block = u16::from_le_bytes([pvd[128], pvd[129]]) as u64;
    let block = if block == 0 { 2048 } else { block };
    Some((block, le32(156 + 2), le32(156 + 10)))
}

/// The entries of an ISO 9660 directory (at most 1 MiB of it is read).
fn iso_dir(f: &mut fs::File, block: u64, lba: u64, size: u64) -> Vec<IsoEntry> {
    let Some(d) = read_exact_at(f, lba * block, size.min(1 << 20) as usize) else { return Vec::new() };
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + 33 < d.len() {
        let len = d[at] as usize;
        if len == 0 {
            // records never cross a block: the rest of this one is padding
            at = (at / block as usize + 1) * block as usize;
            continue;
        }
        if len < 34 || at + len > d.len() {
            break;
        }
        let r = &d[at..at + len];
        let le32 = |o: usize| u32::from_le_bytes([r[o], r[o + 1], r[o + 2], r[o + 3]]) as u64;
        let name_len = r[32] as usize;
        let raw = &r[33..(33 + name_len).min(len)];
        if raw != [0] && raw != [1] {
            let name = String::from_utf8_lossy(raw);
            let name = name.split(';').next().unwrap_or("").trim_end_matches('.').to_string();
            out.push(IsoEntry { name, lba: le32(2), size: le32(10), dir: r[25] & 2 != 0 });
        }
        at += len;
    }
    out
}

/// A disc image's TITLE_ID (from PS3_DISC.SFB) and PARAM.SFO version, read
/// from its ISO 9660 file system without mounting it. Errors are the
/// player's.
fn image_ids(p: &Path) -> Result<(String, String), String> {
    let mut f = fs::File::open(p).map_err(|e| format!("Couldn't read {}: {e}", p.display()))?;
    let (block, lba, size) = iso_root(&mut f).ok_or_else(|| format!("{} isn't a disc image (.iso): pick your Resistance 2 disc image or its folder.", p.display()))?;
    let root = iso_dir(&mut f, block, lba, size);
    let find = |es: &[IsoEntry], n: &str| es.iter().position(|e| e.name.eq_ignore_ascii_case(n));
    let sfb = find(&root, "PS3_DISC.SFB").filter(|&i| !root[i].dir).ok_or_else(|| format!("{} isn't a PS3 game disc (it has no PS3_DISC.SFB).", p.display()))?;
    let e = &root[sfb];
    let id = read_exact_at(&mut f, e.lba * block, e.size.min(64 * 1024) as usize).and_then(|d| sfb_title_id(&d)).unwrap_or_default();
    // PS3_GAME/PARAM.SFO's version, when it can be read
    let version = find(&root, "PS3_GAME")
        .filter(|&i| root[i].dir)
        .map(|i| iso_dir(&mut f, block, root[i].lba, root[i].size))
        .and_then(|g| g.into_iter().find(|e| e.name.eq_ignore_ascii_case("PARAM.SFO") && !e.dir))
        .and_then(|e| read_exact_at(&mut f, e.lba * block, e.size.min(64 * 1024) as usize))
        .map(|d| saves::sfo_values(&d))
        .and_then(|v| v.get("VERSION").or(v.get("APP_VER")).cloned())
        .unwrap_or_default();
    Ok((id, version))
}

fn wrong_disc(id: &str) -> String {
    if id.is_empty() {
        "That disc's ID can't be read: this port needs your Resistance 2 US disc, BCUS98120.".into()
    } else {
        format!("That disc is {id}, not Resistance 2's US disc BCUS98120 (other regions don't work yet).")
    }
}

/// Whether `p` is the player's BCUS98120 disc: a disc image whose
/// PS3_DISC.SFB says BCUS-98120, or a folder with PS3_GAME/ whose PARAM.SFO
/// says BCUS98120 (and whose PS3_DISC.SFB, when there, agrees) and that has
/// the EBOOT. Reads only; returns the copy and its version (may be empty).
/// The error is said to the player.
pub fn check_copy(p: &Path) -> Result<(GameCopy, String), String> {
    if !p.exists() {
        return Err(format!("{} isn't there: pick your Resistance 2 disc image (.iso) or its folder again.", p.display()));
    }
    if p.is_dir() {
        let root = disc_root_of(p).ok_or_else(|| format!("{} has no PS3_GAME folder: pick the folder that holds PS3_GAME and PS3_DISC.SFB, or the disc image (.iso).", p.display()))?;
        let sfo = saves::read_sfo(&root.join("PS3_GAME/PARAM.SFO")).ok_or("Its PS3_GAME/PARAM.SFO can't be read: the copy may be incomplete.")?;
        let id = sfo.get("TITLE_ID").cloned().unwrap_or_default();
        if id_key(&id) != TITLE_ID {
            return Err(wrong_disc(&id));
        }
        if let Ok(d) = fs::read(root.join("PS3_DISC.SFB")) {
            let sfb = sfb_title_id(&d).unwrap_or_default();
            if id_key(&sfb) != TITLE_ID {
                return Err(wrong_disc(&sfb));
            }
        }
        if !root.join("PS3_GAME/USRDIR/EBOOT.BIN").is_file() {
            return Err("That folder has no PS3_GAME/USRDIR/EBOOT.BIN: the copy is incomplete.".into());
        }
        let version = sfo.get("VERSION").or(sfo.get("APP_VER")).cloned().unwrap_or_default();
        return Ok((GameCopy::Folder(root), version));
    }
    let (id, version) = image_ids(p)?;
    if id_key(&id) != TITLE_ID {
        return Err(wrong_disc(&id));
    }
    Ok((GameCopy::Image(p.to_path_buf()), version))
}

/// `--game` with the window: check it and remember it as the disc before
/// the window opens. Returns whether `--play` may go ahead (no `--game`, or a
/// good one).
pub fn window_game(game: Option<&Path>) -> bool {
    let Some(g) = game else { return true };
    match check_copy(g).and_then(|_| remember_disc(g)) {
        Ok(root) => {
            eprintln!("[launcher] --game: the disc is {}", root.display());
            true
        }
        Err(e) => {
            eprintln!("[launcher] --game {}: {e} (--play is not started)", g.display());
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Progress
// ---------------------------------------------------------------------------

/// One output line, ignoring a closed stdout (the program that started us
/// may have stopped reading).
fn say(line: &str) {
    let mut o = std::io::stdout().lock();
    let _ = writeln!(o, "{line}");
    let _ = o.flush();
}

/// `progress <fraction> <text>` (RecompLauncher's prepare contract): the
/// fraction from 0 to 1 with three decimals, the text on one line.
pub fn progress_line(f: f64, text: &str) -> String {
    let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("progress {:.3} {text}", f.clamp(0.0, 1.0))
}

/// A terminal's colour and bold escapes (\x1b[...m) taken out.
pub fn strip_esc(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\x1b' && it.peek() == Some(&'[') {
            it.next();
            for c in it.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// A script's output line as it is printed: without escapes, and indented
/// when it would read as a progress line itself.
pub fn log_line(raw: &str) -> String {
    let l = strip_esc(raw);
    if l.starts_with("progress") {
        format!("  {l}")
    } else {
        l
    }
}

/// The progress lines printed: the fraction never goes back, and a line is
/// printed only when the text changes or the fraction moves by half a
/// percent.
#[derive(Default)]
pub struct Reporter {
    last: f64,
    printed: f64,
    text: String,
    any: bool,
}

impl Reporter {
    pub fn update(&mut self, f: f64, text: &str) -> Option<String> {
        let f = f.clamp(0.0, 1.0).max(self.last);
        self.last = f;
        if self.any && text == self.text && f < self.printed + 0.005 && !(f >= 1.0 && self.printed < 1.0) {
            return None;
        }
        self.any = true;
        self.printed = f;
        self.text = text.to_string();
        Some(progress_line(f, text))
    }

    /// The same, always printed (a step's end or a failure's reason).
    pub fn force(&mut self, f: f64, text: &str) -> String {
        self.any = false;
        self.update(f, text).unwrap_or_default()
    }

    fn say(&mut self, f: f64, text: &str) {
        if let Some(l) = self.update(f, text) {
            say(&l);
        }
    }

    fn fail(&mut self, text: &str) {
        let l = self.force(self.last, text);
        say(&l);
    }

    #[cfg(test)]
    pub fn last(&self) -> f64 {
        self.last
    }
}

/// The install's steps, as the window's progress bar has them
/// (src/progress.ts: the same ids and weights, a step's rough share of a full
/// install on an Apple-silicon Mac).
pub const STEP_IDS: [&str; 9] = ["sources", "deps", "toolkit", "venv", "decrypt", "analyse", "lift", "spu", "build"];
const WEIGHT: [f64; 9] = [1.0, 8.0, 2.0, 3.0, 2.0, 6.0, 35.0, 6.0, 37.0];

/// tools/build_online.sh's steps (the online version), each one's rough
/// share of its run (the download depends on the connection: 340 MB).
pub const ONLINE_STEP_IDS: [&str; 8] = ["download", "extract", "stage", "decrypt", "analyse", "lift", "spu", "build"];
const ONLINE_WEIGHT: [f64; 8] = [10.0, 5.0, 1.0, 1.0, 5.0, 36.0, 6.0, 36.0];

/// A run whose "==> <step>" lines move the bar: its steps, their weights,
/// the text under the bar for each, and the share of the whole bar it
/// fills (the campaign's install, then the online version's).
pub struct Plan {
    pub ids: &'static [&'static str],
    weights: &'static [f64],
    text: fn(Option<&str>) -> &'static str,
    from: f64,
    to: f64,
}

/// The share of the bar before the install starts (the copy, the tools, the
/// sources' update), where the campaign's install ends and the online
/// version's starts, and where that ends ("Ready to play" is 1).
const INSTALL_FROM: f64 = 0.04;
const CAMPAIGN_TO: f64 = 0.62;
const INSTALL_TO: f64 = 0.99;

pub const CAMPAIGN: Plan = Plan { ids: &STEP_IDS, weights: &WEIGHT, text: stage_text, from: INSTALL_FROM, to: CAMPAIGN_TO };
pub const ONLINE: Plan = Plan { ids: &ONLINE_STEP_IDS, weights: &ONLINE_WEIGHT, text: online_stage_text, from: CAMPAIGN_TO, to: INSTALL_TO };

impl Plan {
    /// The whole bar, for this run's own share done.
    pub fn overall(&self, fraction: f64) -> f64 {
        self.from + (self.to - self.from) * fraction.clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum StepState {
    Pending,
    Running,
    Done,
    Skipped,
    Error,
}

#[derive(Clone, Copy)]
struct Step {
    state: StepState,
    start: f64,
    sub: Option<f64>,
}

/// The install's progress from the scripts' "==> <step>" lines, as
/// src/progress.ts reads them: the next step ends the one before, steps gone
/// past without a line count as skipped, "==> [k/n]" moves the running step
/// along (tools/bootstrap_deps.sh), "==> done" ends the run. Times are in
/// seconds from any start.
pub struct StepTracker {
    plan: &'static Plan,
    steps: Vec<Step>,
}

impl Default for StepTracker {
    fn default() -> Self {
        StepTracker::new(&CAMPAIGN)
    }
}

impl StepTracker {
    pub fn new(plan: &'static Plan) -> Self {
        StepTracker { plan, steps: vec![Step { state: StepState::Pending, start: 0.0, sub: None }; plan.ids.len()] }
    }

    fn current(&mut self) -> Option<&mut Step> {
        self.steps.iter_mut().find(|s| s.state == StepState::Running)
    }

    fn close(&mut self, state: StepState) {
        if let Some(c) = self.current() {
            c.state = state;
        }
    }

    pub fn feed(&mut self, raw: &str, t: f64) {
        let line = strip_esc(raw);
        // "==>", then at least one space
        let Some(rest) = line.strip_prefix("==>").filter(|r| r.starts_with(char::is_whitespace)) else { return };
        let rest = rest.trim_start();
        if let Some(k) = rest.strip_prefix('[').and_then(|r| r.split_once(']')).and_then(|(kn, _)| kn.split_once('/')) {
            if let (Ok(k), Ok(n)) = (k.0.trim().parse::<f64>(), k.1.trim().parse::<f64>()) {
                if let Some(c) = self.current() {
                    c.sub = Some(((k - 1.0) / n.max(1.0)).clamp(0.0, 1.0));
                }
            }
            return;
        }
        let word: String = rest.chars().take_while(|c| c.is_ascii_lowercase()).collect();
        if word.is_empty() {
            return;
        }
        if word == "done" {
            self.close(StepState::Done);
            return;
        }
        let Some(i) = self.plan.ids.iter().position(|s| *s == word) else { return };
        self.close(StepState::Done);
        for s in &mut self.steps[..i] {
            if s.state == StepState::Pending {
                s.state = StepState::Skipped;
            }
        }
        self.steps[i] = Step { state: StepState::Running, start: t, sub: None };
    }

    /// The run ended: the running step done or failed; on success what never
    /// ran counts as skipped.
    pub fn finish(&mut self, ok: bool) {
        self.close(if ok { StepState::Done } else { StepState::Error });
        if ok {
            for s in &mut self.steps {
                if s.state == StepState::Pending {
                    s.state = StepState::Skipped;
                }
            }
        }
    }

    /// The share of the run done at `t`: a running step counts as far as it
    /// says, or as half done after two minutes and at most 90 %.
    pub fn fraction(&self, t: f64) -> f64 {
        let total: f64 = self.plan.weights.iter().sum();
        let mut done = 0.0;
        for (s, w) in self.steps.iter().zip(self.plan.weights) {
            match s.state {
                StepState::Done | StepState::Skipped => done += w,
                StepState::Running => {
                    let el = (t - s.start).max(0.0);
                    done += w * s.sub.unwrap_or(0.0).max((el / (el + 120.0)).min(0.9));
                }
                _ => {}
            }
        }
        done / total
    }

    /// The step running, or the one that failed.
    pub fn stage(&self) -> Option<&'static str> {
        self.steps.iter().position(|s| matches!(s.state, StepState::Running | StepState::Error)).map(|i| self.plan.ids[i])
    }

    /// The whole bar at `t`, and the text under it.
    pub fn bar(&self, t: f64) -> (f64, &'static str) {
        (self.plan.overall(self.fraction(t)), (self.plan.text)(self.stage()))
    }
}

/// A step's text under the progress bar.
pub fn stage_text(id: Option<&str>) -> &'static str {
    match id {
        Some("sources") => "Downloading the port's sources",
        Some("deps") => "Installing the build tools",
        Some("toolkit") => "Setting up the toolkit",
        Some("venv") => "Setting up Python",
        Some("decrypt") => "Decrypting EBOOT.BIN from your disc",
        Some("analyse") => "Analysing the game's code",
        Some("lift") => "Translating the game's code (the longest step)",
        Some("spu") => "Translating the SPU programs",
        Some("build") => "Compiling the game (the other long step)",
        _ => "Starting the build",
    }
}

/// An online step's text under the progress bar.
pub fn online_stage_text(id: Option<&str>) -> &'static str {
    match id {
        Some("download") => "Online: downloading the game's official 01.60 update from Sony (340 MB)",
        Some("extract") => "Online: unpacking the 01.60 update",
        Some("stage") => "Online: putting the update's files in place",
        Some("decrypt") => "Online: decrypting the 01.60 executable",
        Some("analyse") => "Online: analysing the 01.60 code",
        Some("lift") => "Online: translating the 01.60 code (the longest step)",
        Some("spu") => "Online: translating the SPU programs",
        Some("build") => "Online: compiling the online version",
        _ => "Online: starting the online version's build",
    }
}

const NETWORK: &[&str] = &["could not download", "could not resolve host", "failed to connect", "unable to access", "connection timed out", "network is unreachable", "check the internet connection and"];

/// Why the install failed, from the step it stopped at and its last lines
/// (as the window's installError tells them apart).
pub fn classify(stage: Option<&str>, tail: &[String]) -> Exit {
    let text = tail.join("\n").to_ascii_lowercase();
    let has = |n: &[&str]| n.iter().any(|x| text.contains(x));
    if has(&["another install is already running"]) {
        Exit::Running
    } else if has(NETWORK) || stage == Some("sources") {
        Exit::Sources
    } else if has(&["the disc is not mounted"]) {
        Exit::Copy
    } else {
        Exit::Build
    }
}

/// Why the online version's build failed: Sony's packages or the keys could
/// not be downloaded, or anything else.
pub fn classify_online(stage: Option<&str>, tail: &[String]) -> Exit {
    let text = tail.join("\n").to_ascii_lowercase();
    if NETWORK.iter().any(|x| text.contains(x)) || stage == Some("download") {
        Exit::OnlineDownload
    } else {
        Exit::OnlineBuild
    }
}

/// The install's output, for the headless prepare: every line printed
/// (log_line), the steps followed, progress lines as the bar moves, the last
/// lines kept for classify, and the exit code sent on.
struct Headless {
    st: Mutex<HeadlessState>,
    done: Mutex<Option<mpsc::Sender<i32>>>,
    t0: Instant,
}

struct HeadlessState {
    steps: StepTracker,
    rep: Reporter,
    tail: VecDeque<String>,
    errors: Vec<String>,
}

impl Headless {
    fn new(plan: &'static Plan, rep: Reporter, done: mpsc::Sender<i32>) -> Self {
        Headless {
            st: Mutex::new(HeadlessState { steps: StepTracker::new(plan), rep, tail: VecDeque::new(), errors: Vec::new() }),
            done: Mutex::new(Some(done)),
            t0: Instant::now(),
        }
    }

    fn tick(&self) {
        let t = self.t0.elapsed().as_secs_f64();
        let mut st = self.st.lock().unwrap();
        let (f, text) = st.steps.bar(t);
        if let Some(l) = st.rep.update(f, text) {
            say(&l);
        }
    }
}

impl OutputSink for Headless {
    fn line(&self, raw: &str) {
        let line = log_line(raw);
        say(&line);
        {
            let mut st = self.st.lock().unwrap();
            st.steps.feed(&line, self.t0.elapsed().as_secs_f64());
            if let Some(e) = line.strip_prefix("ERROR:") {
                st.errors.push(e.trim().to_string());
            }
            st.tail.push_back(line);
            if st.tail.len() > 400 {
                st.tail.pop_front();
            }
        }
        self.tick();
    }
    fn exit(&self, code: i32) {
        if let Some(tx) = self.done.lock().unwrap().take() {
            let _ = tx.send(code);
        }
    }
}

/// A child's output printed as plain lines (the sources' pull), its code
/// sent on.
struct Lines(Mutex<Option<mpsc::Sender<i32>>>);

impl OutputSink for Lines {
    fn line(&self, line: &str) {
        say(&log_line(line));
    }
    fn exit(&self, code: i32) {
        if let Some(tx) = self.0.lock().unwrap().take() {
            let _ = tx.send(code);
        }
    }
}

/// One headless prepare at a time: an exclusive lock on <data>/prepare.lock,
/// held while the returned file is open (the system drops it when the
/// process ends, however it ends). None: another one holds it.
fn take_lock() -> Result<Option<fs::File>, String> {
    let data = data_root();
    fs::create_dir_all(&data).map_err(|e| format!("{}: {e}", data.display()))?;
    let p = data.join("prepare.lock");
    let f = fs::OpenOptions::new().create(true).truncate(false).write(true).open(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    match f.try_lock() {
        Ok(()) => Ok(Some(f)),
        Err(fs::TryLockError::WouldBlock) => Ok(None),
        Err(fs::TryLockError::Error(e)) => Err(format!("{}: {e}", p.display())),
    }
}

/// This game's build is running (started anywhere): a process whose command
/// line has build/BCUS98120/ResistanceRecomp, or the online version's
/// build/BCUS98120_v160/ResistanceRecomp. Not the name alone: the other
/// Resistance ports' builds are called ResistanceRecomp too. Only looked at.
fn game_running() -> bool {
    if cfg!(windows) {
        return process_running(GAME_EXE);
    }
    Command::new("pgrep")
        .args(["-f", &game_pattern()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// game_running's pgrep pattern (extended regular expression).
fn game_pattern() -> String {
    format!("{TITLE_ID}(_v160)?/{GAME_EXE}( |$)")
}

fn same_dir(a: &Path, b: &Path) -> bool {
    a.canonicalize().ok().zip(b.canonicalize().ok()).is_some_and(|(a, b)| a == b)
}

/// `--prepare`'s process: run_prepare, then on a failure what its exit code
/// means to the player as the last line. Returns the exit code.
pub fn prepare_main(game: Option<&Path>) -> i32 {
    let e = run_prepare(game);
    if e != Exit::Ok {
        say(&format!("Exit code {}: {}", e.code(), e.says()));
    }
    e.code()
}

/// `--prepare --game <copy>`: see the module's comment. Prints progress and
/// log lines on stdout; never opens a window.
pub fn run_prepare(game: Option<&Path>) -> Exit {
    let mut rep = Reporter::default();
    rep.say(0.0, "Checking your copy of the game");
    if !cfg!(target_os = "macos") || !host_supported() {
        rep.fail("This port builds and runs on a Mac (Apple silicon) only.");
        return Exit::NeedsPlayer;
    }
    let Some(game) = game else {
        rep.fail("No copy of the game given: --game <your Resistance 2 BCUS98120 disc image or folder>.");
        return Exit::Copy;
    };
    // read only so far: nothing is written before the copy is known good
    let (copy, version) = match check_copy(game) {
        Ok(c) => c,
        Err(e) => {
            rep.fail(&e);
            return Exit::Copy;
        }
    };
    say(&format!("Your copy: {} (BCUS98120{})", game.display(), if version.is_empty() { String::new() } else { format!(", version {version}") }));

    let _lock = match take_lock() {
        Ok(Some(f)) => f,
        Ok(None) => {
            rep.fail("Another setup of Resistance 2 is already running.");
            return Exit::Running;
        }
        Err(e) => {
            rep.fail(&format!("Couldn't write the launcher's folder: {e}"));
            return Exit::Failed;
        }
    };
    if game_running() {
        rep.fail("Resistance 2 is running: close it, then set it up again.");
        return Exit::Running;
    }

    // the disc, remembered as the launcher's own (Game > Disc)
    if let GameCopy::Image(img) = &copy {
        rep.say(0.01, "Mounting your disc image");
        if let Err(e) = attach_image(img) {
            rep.fail(&format!("Your disc image wouldn't mount: {e}"));
            return Exit::Copy;
        }
    }
    match remember_disc(game) {
        Ok(root) => say(&format!("The disc: {}", root.display())),
        Err(e) => {
            rep.fail(&format!("Couldn't remember the disc in the launcher's settings: {e}"));
            return Exit::Failed;
        }
    }

    // Apple's Command Line Tools: their installer needs the player
    rep.say(0.02, "Checking Apple's Command Line Tools");
    if !clt_installed() {
        let opened = open_clt_installer().is_ok();
        rep.fail(if opened {
            "Apple's Command Line Tools are needed: finish their installer (it just opened), then set the game up again."
        } else {
            "Apple's Command Line Tools are needed: run xcode-select --install in Terminal, then set the game up again."
        });
        return Exit::NeedsPlayer;
    }

    // the install's own checkout: the newest sources (a checkout of the
    // player's own, set by hand or found elsewhere, is left as it is)
    let st = load_settings_file();
    if let Some(root) = port_root(&st).filter(|r| r.join("tools/rebuild.sh").is_file() && same_dir(r, &src_dir())) {
        let src = update::sources(&root, false);
        if src.git && !src.upstream.is_empty() && src.dirty == 0 {
            rep.say(0.03, "Getting the newest sources");
            say(&format!("$ git -C {} pull --ff-only", root.display()));
            let (tx, rx) = mpsc::channel();
            let code = match pull_command(&root).spawn() {
                Ok(child) => {
                    let t = stream_output(child, None, Arc::new(Lines(Mutex::new(Some(tx)))));
                    let c = rx.recv().unwrap_or(-1);
                    let _ = t.join();
                    c
                }
                Err(e) => {
                    say(&format!("git: {e}"));
                    -1
                }
            };
            if code != 0 {
                say("Couldn't get the newest sources (no connection?): building the ones already here.");
            }
        }
    }

    // the window's Install the game, with its output here
    rep.say(INSTALL_FROM, stage_text(None));
    let job = match install_job() {
        Ok(j) => j,
        Err(InstallRefusal::Clt) => {
            rep.fail("Apple's Command Line Tools are needed: run xcode-select --install, then set the game up again.");
            return Exit::NeedsPlayer;
        }
        Err(InstallRefusal::NoDisc) => {
            rep.fail("Your disc isn't mounted: pick your disc image again.");
            return Exit::Copy;
        }
        Err(InstallRefusal::Rpcs3) => {
            rep.fail("Decrypting the game needs RPCS3 here: install it (rpcs3.net), then set the game up again.");
            return Exit::NeedsPlayer;
        }
        Err(InstallRefusal::Other(e)) => {
            rep.fail(&e);
            return Exit::Failed;
        }
    };
    let at = job.at.clone();
    let (rep, result) = run_job(job, &CAMPAIGN, rep);
    let mut rep = rep;
    if let Err((stage, tail, why)) = result {
        rep.fail(&why);
        say(&format!("The install's log: {}", install_log_path().display()));
        return classify(stage, &tail);
    }
    let b = resolve_builds(&at);
    if !b.ok {
        rep.fail(&format!("The build ended, but {} is missing.", b.missing.join(", ")));
        return Exit::Build;
    }

    // the online version, after the campaign: never the setup's failure
    if std::env::var_os("RES_NO_ONLINE").is_some_and(|v| v == "1") {
        let l = rep.force(1.0, "Ready to play (the online version was left out: RES_NO_ONLINE=1)");
        say(&l);
        return Exit::Ok;
    }
    let (mut rep, online) = prepare_online(&at, rep);
    let text = match &online {
        Ok(()) => "Ready to play, online too".to_string(),
        Err((e, why)) => format!(
            "Ready to play the campaign. The online version isn't ready: {} (starting online says so, exit code {}; set the game up again to try again)",
            why.trim_end_matches('.'),
            e.code()
        ),
    };
    let l = rep.force(1.0, &text);
    say(&l);
    Exit::Ok
}

/// What a step that failed left: the step, the run's last lines, and the
/// reason to show (its last "ERROR:" line, else which step failed).
type JobFailure = (Option<&'static str>, Vec<String>, String);

/// Run an install job (the campaign's install, the online build) with its
/// output on stdout and the bar moving through `plan`'s share, from where
/// `rep` left it; the reporter comes back for what follows.
fn run_job(job: InstallJob, plan: &'static Plan, rep: Reporter) -> (Reporter, Result<(), JobFailure>) {
    let (tx, rx) = mpsc::channel();
    let sink = Arc::new(Headless::new(plan, rep, tx));
    let ended = match spawn_install(job, sink.clone()) {
        Ok((_, t)) => t,
        Err(e) => {
            let rep = std::mem::take(&mut sink.st.lock().unwrap().rep);
            return (rep, Err((None, Vec::new(), format!("Couldn't start it: {e}"))));
        }
    };
    // the bar moves inside a long step too (the lift, the compile)
    let stop = Arc::new(AtomicBool::new(false));
    let ticker = {
        let (sink, stop) = (sink.clone(), stop.clone());
        thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                for _ in 0..50 {
                    if stop.load(Ordering::SeqCst) {
                        return;
                    }
                    thread::sleep(std::time::Duration::from_millis(100));
                }
                sink.tick();
            }
        })
    };
    let code = rx.recv().unwrap_or(-1);
    let _ = ended.join();
    stop.store(true, Ordering::SeqCst);
    let _ = ticker.join();

    let mut st = sink.st.lock().unwrap();
    let ok = code == 0;
    st.steps.finish(ok);
    let rep = std::mem::take(&mut st.rep);
    if ok {
        return (rep, Ok(()));
    }
    let stage = st.steps.stage();
    let tail: Vec<String> = st.tail.iter().cloned().collect();
    let why = st.errors.last().cloned().unwrap_or_else(|| format!("{} failed (code {code}).", (plan.text)(stage)));
    (rep, Err((stage, tail, why)))
}

/// What the last setup's online part ended with, for `--play-online` to say
/// when the online version is not there: <data>/online-build.json.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct OnlineBuildStatus {
    pub ok: bool,
    /// Its exit code (Exit::OnlineDownload, Exit::OnlineBuild; 0 when ok).
    pub code: i32,
    pub why: String,
    /// Seconds since 1970.
    pub at: u64,
}

fn online_status_path() -> PathBuf {
    data_root().join("online-build.json")
}

fn write_online_status(st: &OnlineBuildStatus) {
    if let Ok(j) = serde_json::to_string_pretty(st) {
        let _ = fs::write(online_status_path(), j);
    }
}

fn read_online_status() -> Option<OnlineBuildStatus> {
    fs::read_to_string(online_status_path()).ok().and_then(|t| serde_json::from_str(&t).ok())
}

/// The online version, built in the campaign's port folder `at` by
/// tools/build_online.sh (prepare's second part), its outcome written to
/// online-build.json. Err: its exit code for `--play-online` and why.
fn prepare_online(at: &Path, mut rep: Reporter) -> (Reporter, Result<(), (Exit, String)>) {
    let now = now_secs();
    rep.say(ONLINE.overall(0.0), "The campaign is built. Now its online version (the game's 01.60 update, for PS Rewired)");
    let result = match online_install_job(at) {
        None => Err((Exit::OnlineBuild, "these sources have no tools/build_online.sh".to_string())),
        Some(job) => {
            let (r, res) = run_job(job, &ONLINE, rep);
            rep = r;
            match res {
                Ok(()) => {
                    let o = resolve_online(at);
                    if o.build_ok && o.update_ok {
                        Ok(())
                    } else {
                        Err((Exit::OnlineBuild, format!("the online build ended, but {} is missing.", o.missing.join(", "))))
                    }
                }
                Err((stage, tail, why)) => {
                    say(&format!("The online build's log: {}", online_log_path().display()));
                    Err((classify_online(stage, &tail), why))
                }
            }
        }
    };
    let (ok, code, why) = match &result {
        Ok(()) => (true, 0, String::new()),
        Err((e, why)) => (false, e.code(), why.clone()),
    };
    write_online_status(&OnlineBuildStatus { ok, code, why, at: now });
    (rep, result)
}

/// `--game <copy>` before a direct start: checked, attached when it is an
/// image, and remembered as the disc.
fn use_game(game: Option<&Path>) -> Result<(), (Exit, String)> {
    let Some(g) = game else { return Ok(()) };
    let (copy, _) = check_copy(g).map_err(|e| (Exit::Copy, e))?;
    if let GameCopy::Image(img) = &copy {
        attach_image(img).map_err(|e| (Exit::Copy, format!("the disc image wouldn't mount: {e}")))?;
    }
    remember_disc(g).map_err(|e| (Exit::Failed, e))?;
    Ok(())
}

/// `--play` under RecompLauncher: the copy given with `--game` remembered,
/// then the window's Play (play_run: the settings, the save backup, the
/// disc), and the launcher's process replaced by the game's (exec_game).
/// Returns an exit code only when the game could not start.
pub fn play_direct(game: Option<&Path>) -> i32 {
    let refuse = |e: Exit, why: &str| {
        eprintln!("[launcher] --play: {why}");
        e.code()
    };
    if let Err((e, why)) = use_game(game) {
        return refuse(e, &why);
    }
    if let Err(e) = require_supported_host() {
        return refuse(Exit::NeedsPlayer, &e);
    }
    if game_running() {
        return refuse(Exit::Running, "the game is already running");
    }
    let s = load_settings_file();
    // caffeinate is started beside the game rather than in front of it, so
    // the process that is exec'd is the window host itself
    let mut no_caffeinate = s.clone();
    no_caffeinate.keep_awake = false;
    let (run, facts) = match play_run(&no_caffeinate, false) {
        Ok(r) => r,
        Err(PlayRefusal::NotBuilt(e)) => return refuse(Exit::NotBuilt, &e),
        Err(PlayRefusal::Disc(e)) => return refuse(Exit::Copy, &e),
        Err(PlayRefusal::Other(e)) => return refuse(Exit::Failed, &e),
    };
    if let Some(e) = &facts.backup_error {
        eprintln!("[launcher] --play: the save backup failed: {e}");
    }
    exec_game(run, s.keep_awake, "--play")
}

/// The exit code `--play-online` ends with when online play cannot start:
/// for a missing online version, what the last setup said about it
/// (online-build.json).
pub(crate) fn online_exit(r: &OnlineRefusal, last: Option<&OnlineBuildStatus>) -> Exit {
    match r {
        OnlineRefusal::Id => Exit::OnlineId,
        OnlineRefusal::Build | OnlineRefusal::Update => match last.filter(|st| !st.ok).and_then(|st| Exit::from_code(st.code)) {
            Some(e @ (Exit::OnlineDownload | Exit::OnlineBuild)) => e,
            _ => Exit::OnlineNotBuilt,
        },
        OnlineRefusal::Install => Exit::OnlineInstall,
        OnlineRefusal::Disc(_) => Exit::Copy,
        OnlineRefusal::Other(_) => Exit::Failed,
    }
}

/// `--play-online` under RecompLauncher (its Online tile): the copy given
/// with `--game` remembered, then the window's Play online (online_run: the
/// online ID, the 01.60 build, the online folder, online_env: the game's own
/// 30 fps, every frame-rate and timing variable left out, those of the
/// launcher's own environment unset too), and the launcher's process
/// replaced by the game's (exec_game). Returns an exit code only when it
/// could not start.
pub fn play_online_direct(game: Option<&Path>) -> i32 {
    let refuse = |e: Exit, why: &str| {
        eprintln!("[launcher] --play-online: {why}");
        e.code()
    };
    if let Err((e, why)) = use_game(game) {
        return refuse(e, &why);
    }
    if let Err(e) = require_supported_host() {
        return refuse(Exit::NeedsPlayer, &e);
    }
    if game_running() {
        return refuse(Exit::Running, "the game is already running");
    }
    let s = load_settings_file();
    let mut no_caffeinate = s.clone();
    no_caffeinate.keep_awake = false;
    let (run, _) = match online_run(&no_caffeinate) {
        Ok(r) => r,
        Err(r) => {
            let last = read_online_status();
            let e = online_exit(&r, last.as_ref());
            let why = match (&r, &last) {
                (OnlineRefusal::Build | OnlineRefusal::Update, Some(st)) if !st.ok => format!("{} (the last setup: {})", r.describe(), st.why),
                _ => r.describe(),
            };
            return refuse(e, &why);
        }
    };
    exec_game(run, s.keep_awake, "--play-online")
}

/// The launcher's process replaced by the game's (exec): its output in
/// last.log of the folder it runs in (the previous one moved to logs/), and
/// `caffeinate` beside it when the settings keep the Mac awake. Returns an
/// exit code only when it could not start (on Windows: the game's own).
fn exec_game(run: GameRun, keep_awake: bool, what: &str) -> i32 {
    let refuse = |e: Exit, why: &str| {
        eprintln!("[launcher] {what}: {why}");
        e.code()
    };
    let (log_path, _) = match session_log(&run.dir, None) {
        Ok(l) => l,
        Err(e) => return refuse(Exit::Failed, &e),
    };
    let (out, err) = match fs::File::create(&log_path).and_then(|f| f.try_clone().map(|g| (f, g))) {
        Ok(f) => f,
        Err(e) => return refuse(Exit::Failed, &format!("{}: {e}", log_path.display())),
    };
    let mut cmd = game_command(&run.argv, &run.env, &run.unset, &run.dir);
    cmd.stdout(out).stderr(err);
    eprintln!("[launcher] {what}: starting the game, log {}", log_path.display());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        if keep_awake && Path::new("/usr/bin/caffeinate").is_file() {
            // waits for this process id, which the game keeps
            let _ = Command::new("/bin/sh")
                .args(["-c", "/usr/bin/caffeinate -i -w \"$1\" >/dev/null 2>&1 &", "sh", &std::process::id().to_string()])
                .stdin(Stdio::null())
                .status();
        }
        let e = cmd.exec();
        refuse(Exit::Failed, &format!("{}: {e}", run.argv[0]))
    }
    #[cfg(not(unix))]
    {
        let _ = keep_awake;
        match cmd.status() {
            Ok(st) => st.code().unwrap_or(Exit::Failed.code()),
            Err(e) => refuse(Exit::Failed, &format!("{}: {e}", run.argv[0])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("rl2-prep-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn args(v: &[&str]) -> Vec<String> {
        std::iter::once("launcher").chain(v.iter().copied()).map(String::from).collect()
    }

    #[test]
    fn command_line() {
        assert_eq!(mode(&args(&["--prepare", "--game", "/x/r2.iso"]), false), Mode::Prepare { game: Some("/x/r2.iso".into()) });
        assert_eq!(mode(&args(&["--game=/x/disc", "--prepare"]), true), Mode::Prepare { game: Some("/x/disc".into()) });
        assert_eq!(mode(&args(&["--prepare"]), false), Mode::Prepare { game: None });
        assert_eq!(mode(&args(&["--prepare", "--game"]), false), Mode::Prepare { game: None });
        assert_eq!(mode(&args(&["--prepare", "--game", ""]), false), Mode::Prepare { game: None });
        // --play opens the window, unless RecompLauncher started it
        assert_eq!(mode(&args(&["--play"]), false), Mode::Window { game: None });
        assert_eq!(mode(&args(&["--play", "--game", "/a b/r2.iso"]), true), Mode::PlayDirect { game: Some("/a b/r2.iso".into()) });
        assert_eq!(mode(&args(&["--game", "/x"]), true), Mode::Window { game: Some("/x".into()) });
        assert_eq!(mode(&args(&[]), true), Mode::Window { game: None });
        // the program's own name is never an argument
        assert_eq!(mode(&["--prepare".to_string()], false), Mode::Window { game: None });
        // --play-online: online without a window under RecompLauncher (it wins
        // over --play: the Online tile's arguments follow start.args), else
        // the window, which starts it once up
        assert_eq!(mode(&args(&["--play", "--game", "/x/r2.iso", "--play-online"]), true), Mode::PlayOnlineDirect { game: Some("/x/r2.iso".into()) });
        assert_eq!(mode(&args(&["--play-online"]), true), Mode::PlayOnlineDirect { game: None });
        assert_eq!(mode(&args(&["--play-online", "--game", "/x"]), false), Mode::Window { game: Some("/x".into()) });
        assert_eq!(mode(&args(&["--prepare", "--play-online"]), true), Mode::Prepare { game: None });
    }

    /// RecompLauncher's tiles: each one's arguments after start.args ask for
    /// the campaign (--play) and online play (--play-online) without a window.
    #[test]
    fn profile_tiles_start_what_they_say() {
        let profile: serde_json::Value = serde_json::from_str(include_str!("../../../recomplauncher.json")).unwrap();
        let strs = |v: &serde_json::Value| v.as_array().unwrap().iter().map(|a| a.as_str().unwrap().replace("{game}", "/x/r2.iso")).collect::<Vec<_>>();
        let start = strs(&profile["start"]["args"]);
        let games = profile["games"].as_array().unwrap();
        assert_eq!(games.len(), 2);
        let tile = |id: &str| games.iter().find(|g| g["id"] == id).unwrap_or_else(|| panic!("no {id} tile"));
        let argv = |g: &serde_json::Value| std::iter::once("launcher".to_string()).chain(start.iter().cloned()).chain(strs(&g["args"])).collect::<Vec<_>>();
        let campaign = tile("campaign");
        assert!(!strs(&campaign["args"]).is_empty(), "a tile without args opens the game's own menu");
        assert_eq!(mode(&argv(campaign), true), Mode::PlayDirect { game: Some("/x/r2.iso".into()) });
        let online = tile("online");
        assert_eq!(mode(&argv(online), true), Mode::PlayOnlineDirect { game: Some("/x/r2.iso".into()) });
        assert!(online["name"].as_str().unwrap().contains("Online"));
        // its saves are the online folder's own (online_dir: <play>/online)
        assert!(online["saves"][0].as_str().unwrap().ends_with("resistance-recomp-2/online/dev_hdd0/home/00000001/savedata"));
        // and the profile has no setting that could change the frame rate
        for s in profile["settings"].as_array().unwrap() {
            let keys: Vec<String> = s["choices"].as_array().into_iter().flatten().flat_map(|c| c["set"].as_object().unwrap().keys().cloned().collect::<Vec<_>>()).chain(s.get("slider").map(|sl| sl["key"].as_str().unwrap().to_string())).collect();
            for k in keys {
                assert!(!["frameClockUs", "extraEnv"].contains(&k.as_str()), "{k}");
            }
        }
    }

    #[test]
    fn online_exit_codes() {
        let failed = |e: Exit| OnlineBuildStatus { ok: false, code: e.code(), why: "x".into(), at: 1 };
        let ok = OnlineBuildStatus { ok: true, code: 0, why: String::new(), at: 1 };
        assert_eq!(online_exit(&OnlineRefusal::Id, None), Exit::OnlineId);
        assert_eq!(online_exit(&OnlineRefusal::Install, None), Exit::OnlineInstall);
        assert_eq!(online_exit(&OnlineRefusal::Disc("no-disc".into()), None), Exit::Copy);
        assert_eq!(online_exit(&OnlineRefusal::Other("x".into()), None), Exit::Failed);
        // a missing online version: what the last setup said about it
        assert_eq!(online_exit(&OnlineRefusal::Build, None), Exit::OnlineNotBuilt);
        assert_eq!(online_exit(&OnlineRefusal::Build, Some(&ok)), Exit::OnlineNotBuilt);
        assert_eq!(online_exit(&OnlineRefusal::Build, Some(&failed(Exit::OnlineDownload))), Exit::OnlineDownload);
        assert_eq!(online_exit(&OnlineRefusal::Update, Some(&failed(Exit::OnlineBuild))), Exit::OnlineBuild);
        assert_eq!(online_exit(&OnlineRefusal::Build, Some(&failed(Exit::Build))), Exit::OnlineNotBuilt, "only the online codes");
        assert_eq!(online_exit(&OnlineRefusal::Id, Some(&failed(Exit::OnlineBuild))), Exit::OnlineId, "the ID first");
        let st = failed(Exit::OnlineDownload);
        assert_eq!(serde_json::from_str::<OnlineBuildStatus>(&serde_json::to_string(&st).unwrap()).unwrap(), st);
        // why the online build failed
        assert_eq!(classify_online(Some("download"), &["curl: (22) The requested URL returned error: 503".into()]), Exit::OnlineDownload);
        assert_eq!(classify_online(Some("decrypt"), &["error: could not download the decryption keys from GitHub (timed out).".into()]), Exit::OnlineDownload);
        assert_eq!(classify_online(Some("extract"), &["curl: (6) Could not resolve host: b0.ww.np.dl.playstation.net".into()]), Exit::OnlineDownload);
        assert_eq!(
            classify_online(Some("decrypt"), &["ERROR: could not decrypt the 01.60 EBOOT.BIN: decrypting it without RPCS3 failed (its keys are downloaded once from RPCS3's source code on GitHub: check the internet connection), and RPCS3 is not installed to try instead.".into()]),
            Exit::OnlineBuild
        );
        assert_eq!(classify_online(Some("build"), &["ninja: build stopped: subcommand failed.".into()]), Exit::OnlineBuild);
        assert_eq!(classify_online(None, &[]), Exit::OnlineBuild);
        assert!(game_pattern().starts_with("BCUS98120(_v160)?/"));
    }

    #[test]
    fn exit_codes_are_distinct_and_in_the_profile() {
        let codes: std::collections::BTreeSet<i32> = Exit::ALL.iter().map(|e| e.code()).collect();
        assert_eq!(codes.len(), Exit::ALL.len());
        assert_eq!(Exit::Ok.code(), 0);
        // recomplauncher.json says each one to the player, word for word
        let profile: serde_json::Value = serde_json::from_str(include_str!("../../../recomplauncher.json")).unwrap();
        let said = profile["exit_codes"].as_object().unwrap();
        assert_eq!(said.len(), Exit::ALL.len());
        for e in Exit::ALL {
            assert_eq!(said[&e.code().to_string()].as_str(), Some(e.says()), "{e:?}");
        }
        assert_eq!(classify(Some("lift"), &["ERROR: another install is already running in /x".into()]), Exit::Running);
        assert_eq!(classify(Some("venv"), &["ERROR: could not install the toolkit's Python packages (pip). Check the internet connection and run this again.".into()]), Exit::Sources);
        assert_eq!(classify(Some("sources"), &["fatal: something".into()]), Exit::Sources);
        assert_eq!(classify(Some("decrypt"), &["ERROR: the disc is not mounted: mount your Resistance 2 (BCUS98120) disc image and run this again.".into()]), Exit::Copy);
        // the decrypt failure names the connection too, but is the build's
        assert_eq!(
            classify(Some("decrypt"), &["ERROR: could not decrypt the game's EBOOT.BIN from your disc. Check that it is Resistance 2 (BCUS98120), and the internet connection (the keys are downloaded once), then run this again.".into()]),
            Exit::Build
        );
        assert_eq!(classify(Some("build"), &["ninja: build stopped: subcommand failed.".into()]), Exit::Build);
    }

    /// RecompLauncher edits launcher.json with the profile's settings: every
    /// key is one of Settings' own, every value it writes reads back as that
    /// value, and the defaults are the launcher's.
    #[test]
    fn profile_settings_are_the_launchers() {
        let profile: serde_json::Value = serde_json::from_str(include_str!("../../../recomplauncher.json")).unwrap();
        let ours = serde_json::to_value(Settings::default()).unwrap();
        // numbers as numbers (an f32 setting comes back as 0.05000000074...)
        let same = |a: &serde_json::Value, b: &serde_json::Value| match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => (x - y).abs() < 1e-6,
            _ => a == b,
        };
        for s in profile["settings"].as_array().unwrap() {
            let id = s["id"].as_str().unwrap();
            for (k, v) in s["defaults"].as_object().unwrap() {
                let want: serde_json::Value = serde_json::from_str(v.as_str().unwrap()).unwrap();
                assert!(same(&ours[k], &want), "{id}: default of {k} is {} here, {want} in the profile", ours[k]);
            }
            let mut writes: Vec<(String, serde_json::Value)> = Vec::new();
            for c in s["choices"].as_array().into_iter().flatten() {
                for (k, v) in c["set"].as_object().unwrap() {
                    writes.push((k.clone(), serde_json::from_str(v.as_str().unwrap()).unwrap()));
                }
            }
            if let Some(sl) = s.get("slider") {
                writes.push((sl["key"].as_str().unwrap().into(), serde_json::json!(sl["max"])));
            }
            assert!(!writes.is_empty(), "{id}");
            for (k, v) in writes {
                assert!(ours.get(&k).is_some(), "{id}: {k} is not a launcher setting");
                let mut file = ours.clone();
                file[&k] = v.clone();
                let back = serde_json::to_value(serde_json::from_value::<Settings>(file).unwrap_or_else(|e| panic!("{id}: {k}={v}: {e}"))).unwrap();
                assert!(same(&back[&k], &v), "{id}: {k}={v} reads back as {}", back[&k]);
            }
        }
    }

    #[test]
    fn progress_lines() {
        assert_eq!(progress_line(0.25, "Translating  the\ngame's code"), "progress 0.250 Translating the game's code");
        assert_eq!(progress_line(1.7, "x"), "progress 1.000 x");
        assert_eq!(progress_line(-1.0, "x"), "progress 0.000 x");
        // RecompLauncher reads "progress <n> <text>": a script line must not pass for one
        assert_eq!(log_line("progress 0.5 fake"), "  progress 0.5 fake");
        assert_eq!(log_line("\x1b[1m==> lift\x1b[0m"), "==> lift");

        let mut r = Reporter::default();
        assert_eq!(r.update(0.1, "a").as_deref(), Some("progress 0.100 a"));
        assert_eq!(r.update(0.102, "a"), None, "under half a percent");
        assert_eq!(r.update(0.05, "b").as_deref(), Some("progress 0.102 b"), "never back");
        assert_eq!(r.update(0.2, "b").as_deref(), Some("progress 0.200 b"));
        assert_eq!(r.force(0.0, "failed"), "progress 0.200 failed");
        assert_eq!(r.update(1.0, "failed").as_deref(), Some("progress 1.000 failed"));
    }

    /// The output of tools/install.sh on a fresh Mac, step by step.
    const INSTALL: &[&str] = &[
        "==> sources (the port, from GitHub)",
        "==> deps (the build tools and libraries: tools/bootstrap_deps.sh)",
        "==> [1/8] CMake 3.31 (already installed)",
        "==> [5/8] SDL2 2.30",
        "\x1b[1m==> toolkit\x1b[0m",
        "==> venv (the Python environment)",
        "==> decrypt (your disc's EBOOT.BIN)",
        "\x1b[1m==> decrypt\x1b[0m",
        "==> analyse (done before with the same inputs, skipped)",
        "\x1b[1m==> lift\x1b[0m",
        "[12/24] lifting unit 12",
        "\x1b[1m==> spu (policy module, raw-SPU programs, job modules, relocatable modules, raw-SPU loader)\x1b[0m",
        "\x1b[1m==> build\x1b[0m",
        "[812/1490] Building C object",
        "\x1b[1m==> done\x1b[0m",
        "==> done",
    ];

    /// tools/build_online.sh's output on a fresh install, step by step.
    const ONLINE_RUN: &[&str] = &[
        "\x1b[1m==> download (the official 01.50 and 01.60 update packages)\x1b[0m",
        "  UP9000-BCUS98120_00-RESISTANCE200001-A0150-V0100-PE.pkg: downloaded, checked",
        "\x1b[1m==> extract (a few minutes)\x1b[0m",
        "\x1b[1m==> stage (the update's files for the online folder's dev_hdd0)\x1b[0m",
        "\x1b[1m==> decrypt (the 01.60 EBOOT.BIN)\x1b[0m",
        "\x1b[1m==> analyse\x1b[0m",
        "\x1b[1m==> lift\x1b[0m",
        "\x1b[1m==> spu (policy module, raw-SPU programs, job modules, relocatable modules, raw-SPU loader)\x1b[0m",
        "\x1b[1m==> build\x1b[0m",
        "\x1b[1m==> done\x1b[0m",
    ];

    /// The campaign's install, then the online version's build, through one
    /// reporter: the bar never goes back, from the first line to "Ready".
    #[test]
    fn online_build_continues_the_bar() {
        let mut rep = Reporter::default();
        let mut last = -1.0;
        let mut texts = Vec::new();
        for (plan, lines) in [(&CAMPAIGN, INSTALL), (&ONLINE, ONLINE_RUN)] {
            let mut steps = StepTracker::new(plan);
            for (i, l) in lines.iter().enumerate() {
                let t = i as f64 * 300.0;
                steps.feed(l, t);
                for dt in [0.0, 30.0, 299.0] {
                    let (f, text) = steps.bar(t + dt);
                    assert!((plan.from..=plan.to).contains(&f), "{l}: {f}");
                    if let Some(line) = rep.update(f, text) {
                        assert!(line.starts_with("progress "));
                    }
                    assert!(rep.last() >= last, "{l}: {} < {last}", rep.last());
                    last = rep.last();
                    if texts.last() != Some(&text) {
                        texts.push(text);
                    }
                }
            }
            steps.finish(true);
            assert!((steps.bar(1e9).0 - plan.to).abs() < 1e-9);
        }
        assert!(texts.contains(&"Online: downloading the game's official 01.60 update from Sony (340 MB)"));
        assert!(texts.contains(&"Online: compiling the online version"));
        assert_eq!(rep.force(1.0, "Ready to play, online too"), "progress 1.000 Ready to play, online too");
        // a re-run with everything done: the steps say "skipped" and the bar still only moves on
        let mut s = StepTracker::new(&ONLINE);
        for l in ["==> download (the update's files are staged already, skipped)", "==> lift (done before with the same inputs, skipped)", "==> build"] {
            s.feed(l, 0.0);
        }
        assert_eq!(s.stage(), Some("build"));
        assert!(s.fraction(0.0) > 0.5);
        // a failure keeps the online step it stopped at
        s.finish(false);
        assert_eq!(s.stage(), Some("build"));
    }

    #[test]
    fn install_steps_move_the_bar_forward() {
        let mut steps = StepTracker::default();
        let mut rep = Reporter::default();
        let mut last = -1.0;
        let mut stages = Vec::new();
        for (i, l) in INSTALL.iter().enumerate() {
            let t = i as f64 * 600.0;
            steps.feed(l, t);
            for dt in [0.0, 60.0, 599.0] {
                let f = CAMPAIGN.overall(steps.fraction(t + dt));
                assert!((0.0..=1.0).contains(&f));
                rep.update(f, stage_text(steps.stage()));
                assert!(rep.last() >= last, "{l}: {} < {last}", rep.last());
                last = rep.last();
            }
            if let Some(s) = steps.stage() {
                if stages.last() != Some(&s) {
                    stages.push(s);
                }
            }
        }
        assert_eq!(stages, ["sources", "deps", "toolkit", "venv", "decrypt", "analyse", "lift", "spu", "build"]);
        steps.finish(true);
        assert!((steps.fraction(1e9) - 1.0).abs() < 1e-9);
        assert!((CAMPAIGN.overall(1.0) - CAMPAIGN_TO).abs() < 1e-9 && CAMPAIGN.overall(0.0) == INSTALL_FROM);
        assert!(CAMPAIGN_TO == ONLINE.overall(0.0) && (ONLINE.overall(1.0) - INSTALL_TO).abs() < 1e-9 && INSTALL_TO < 1.0);

        // an existing checkout: no sources line, so that step counts as skipped
        let mut s2 = StepTracker::default();
        s2.feed("==> deps (the build tools)", 0.0);
        assert_eq!(s2.stage(), Some("deps"));
        assert!(s2.fraction(0.0) > 0.0);
        s2.feed("==> [3/5] FFmpeg", 1.0);
        let f = s2.fraction(1.0);
        assert!(f > 1.0 / 100.0 && f < 9.0 / 100.0, "{f}");
        // a failure keeps the step it stopped at
        s2.finish(false);
        assert_eq!(s2.stage(), Some("deps"));
        // lines that only look like steps change nothing
        let mut s3 = StepTracker::default();
        for l in ["==>lift", "==> LIFT", "  ==> lift", "==> lifting", "progress 0.5 lift"] {
            s3.feed(l, 0.0);
        }
        assert_eq!(s3.stage(), None);
    }

    fn sfb(id: &str) -> Vec<u8> {
        let mut d = vec![0u8; 0x600];
        d[..4].copy_from_slice(b".SFB");
        d[4..6].copy_from_slice(&[0, 1]);
        d[0x20..0x2B].copy_from_slice(b"HYBRID_FLAG");
        d[0x30..0x38].copy_from_slice(&[0, 0, 2, 0, 0, 0, 0, 0x20]);
        d[0x40..0x48].copy_from_slice(b"TITLE_ID");
        d[0x50..0x58].copy_from_slice(&[0, 0, 2, 0x20, 0, 0, 0, 0x10]);
        d[0x220..0x220 + id.len()].copy_from_slice(id.as_bytes());
        d
    }

    fn sfo(id: &str) -> Vec<u8> {
        saves::psf_write(&[
            saves::PsfEntry { key: "TITLE_ID".into(), fmt: saves::PSF_UTF8, data: format!("{id}\0").into_bytes(), max_len: 16 },
            saves::PsfEntry { key: "VERSION".into(), fmt: saves::PSF_UTF8, data: b"01.00\0".to_vec(), max_len: 8 },
        ])
    }

    /// A directory record: its extent, size, flags and name.
    fn rec(lba: u32, size: u32, dir: bool, name: &[u8]) -> Vec<u8> {
        let len = 33 + name.len() + (name.len() + 1) % 2;
        let mut r = vec![0u8; len];
        r[0] = len as u8;
        r[2..6].copy_from_slice(&lba.to_le_bytes());
        r[6..10].copy_from_slice(&lba.to_be_bytes());
        r[10..14].copy_from_slice(&size.to_le_bytes());
        r[14..18].copy_from_slice(&size.to_be_bytes());
        r[25] = if dir { 2 } else { 0 };
        r[32] = name.len() as u8;
        r[33..33 + name.len()].copy_from_slice(name);
        r
    }

    /// A small ISO 9660 image: the root (sector 18) with PS3_DISC.SFB
    /// (sector 19) and PS3_GAME/ (sector 20) holding PARAM.SFO (sector 21).
    fn fake_iso(sfb_id: &str, sfo_id: &str) -> Vec<u8> {
        let s = 2048usize;
        let mut img = vec![0u8; 22 * s + 2048];
        let (sfb, sfo) = (sfb(sfb_id), sfo(sfo_id));
        let pvd = &mut img[16 * s..17 * s];
        pvd[0] = 1;
        pvd[1..6].copy_from_slice(b"CD001");
        pvd[6] = 1;
        pvd[128..130].copy_from_slice(&2048u16.to_le_bytes());
        pvd[156..156 + 34].copy_from_slice(&rec(18, 2048, true, &[0]));
        let term = &mut img[17 * s..18 * s];
        term[0] = 255;
        term[1..6].copy_from_slice(b"CD001");
        let root: Vec<u8> = [rec(18, 2048, true, &[0]), rec(18, 2048, true, &[1]), rec(19, sfb.len() as u32, false, b"PS3_DISC.SFB;1"), rec(20, 2048, true, b"PS3_GAME")].concat();
        img[18 * s..18 * s + root.len()].copy_from_slice(&root);
        img[19 * s..19 * s + sfb.len()].copy_from_slice(&sfb);
        let game: Vec<u8> = [rec(20, 2048, true, &[0]), rec(18, 2048, true, &[1]), rec(21, sfo.len() as u32, false, b"PARAM.SFO;1")].concat();
        img[20 * s..20 * s + game.len()].copy_from_slice(&game);
        img[21 * s..21 * s + sfo.len()].copy_from_slice(&sfo);
        img
    }

    #[test]
    fn sfb_ids() {
        assert_eq!(sfb_title_id(&sfb("BCUS-98120")).as_deref(), Some("BCUS-98120"));
        assert_eq!(id_key("BCUS-98120"), TITLE_ID);
        assert_eq!(sfb_title_id(b"not an sfb at all, not at all, not at all, not at all, no"), None);
        // an empty slot before TITLE_ID is passed over
        let mut gap = sfb("BCUS-98120");
        gap[0x20..0x40].fill(0);
        assert_eq!(sfb_title_id(&gap).as_deref(), Some("BCUS-98120"));
        let mut cut = sfb("BCUS-98120");
        cut.truncate(0x100);
        assert_eq!(sfb_title_id(&cut), None, "the value lies past the end");
    }

    #[test]
    fn disc_images() {
        let d = tmp("iso");
        let good = d.join("Resistance 2 (USA).iso");
        fs::write(&good, fake_iso("BCUS-98120", "BCUS98120")).unwrap();
        assert_eq!(check_copy(&good).unwrap(), (GameCopy::Image(good.clone()), "01.00".to_string()));
        let other = d.join("backup-07.iso");
        fs::write(&other, fake_iso("BCES-00226", "BCES00226")).unwrap();
        assert!(check_copy(&other).unwrap_err().contains("BCES-00226"));
        let plain = d.join("notes.iso");
        fs::write(&plain, vec![0u8; 40000]).unwrap();
        assert!(check_copy(&plain).unwrap_err().contains("isn't a disc image"));
        let small = d.join("tiny.iso");
        fs::write(&small, b"x").unwrap();
        assert!(check_copy(&small).is_err());
        // an ISO 9660 image of something else: no PS3_DISC.SFB
        let mut cd = fake_iso("BCUS-98120", "BCUS98120");
        let at = cd.windows(12).position(|w| w == b"PS3_DISC.SFB").unwrap();
        cd[at..at + 12].copy_from_slice(b"README.TXT;1");
        let cd_path = d.join("cd.iso");
        fs::write(&cd_path, cd).unwrap();
        assert!(check_copy(&cd_path).unwrap_err().contains("no PS3_DISC.SFB"));
        assert!(check_copy(&d.join("missing.iso")).unwrap_err().contains("isn't there"));
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn disc_folders() {
        let d = tmp("folder");
        let disc = d.join("BCUS98120");
        assert!(check_copy(&disc).is_err());
        fs::create_dir_all(disc.join("PS3_GAME/USRDIR")).unwrap();
        assert!(check_copy(&disc).unwrap_err().contains("no PS3_GAME"));
        fs::write(disc.join("PS3_GAME/PARAM.SFO"), sfo("BCUS98120")).unwrap();
        assert!(check_copy(&disc).unwrap_err().contains("EBOOT.BIN"));
        fs::write(disc.join("PS3_GAME/USRDIR/EBOOT.BIN"), b"x").unwrap();
        fs::write(disc.join("PS3_DISC.SFB"), sfb("BCUS-98120")).unwrap();
        assert_eq!(check_copy(&disc).unwrap(), (GameCopy::Folder(disc.clone()), "01.00".to_string()));
        // PS3_GAME itself is taken for its disc
        assert_eq!(check_copy(&disc.join("PS3_GAME")).unwrap().0, GameCopy::Folder(disc.clone()));
        // another region, in either file
        fs::write(disc.join("PS3_DISC.SFB"), sfb("BCES-00226")).unwrap();
        assert!(check_copy(&disc).unwrap_err().contains("BCES-00226"));
        fs::remove_file(disc.join("PS3_DISC.SFB")).unwrap();
        assert!(check_copy(&disc).is_ok(), "PS3_DISC.SFB is not required in a folder");
        fs::write(disc.join("PS3_GAME/PARAM.SFO"), sfo("BCES00226")).unwrap();
        assert!(check_copy(&disc).unwrap_err().contains("BCES00226"));
        fs::remove_dir_all(&d).unwrap();
    }
}

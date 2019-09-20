use super::{MusicBackend, MusicControl, PlaybackInfo, SongInfo};
use format::data::Format;
use nom::IResult;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use widget::base::Sender;

named!(parse_playback_info<&[u8], PlaybackInfo>,
    do_parse!(
        tag!("[") >>
        playing: take_until!("]") >>
        take_until_and_consume!("#") >>
        playlist_index: take_until!("/") >>
        take!(1) >>
        playlist_total: take_until!(" ") >>
        take!(1) >>
        progress_m: ws!(take_until!(":")) >>
        take!(1) >>
        progress_s: take_until!("/") >>
        take!(1) >>
        total_m: take_until!(":") >>
        take!(1) >>
        total_s: take_until!(" ") >>

        ( PlaybackInfo {
            playing: (String::from_utf8_lossy(playing) == "playing"),

            progress: Duration::from_secs(
                String::from_utf8_lossy(progress_m).parse::<u64>().unwrap() * 60
                + String::from_utf8_lossy(progress_s).parse::<u64>().unwrap()),
            total: Duration::from_secs(
                String::from_utf8_lossy(total_m).parse::<u64>().unwrap() * 60
                + String::from_utf8_lossy(total_s).parse::<u64>().unwrap()),

            playlist_index:
                String::from_utf8_lossy(playlist_index).parse().unwrap(),
            playlist_total:
                String::from_utf8_lossy(playlist_total).parse().unwrap()
        })
    )
);

fn mpc_get_format(format: &'static str) -> Option<String> {
    let mpc = Command::new("mpc")
        .arg("-f")
        .arg(format)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Couldn't run `mpc -f`");

    BufReader::new(mpc.stdout.unwrap())
        .lines()
        .next()
        .and_then(|result| result.ok())
}

fn get_playback_info() -> Option<PlaybackInfo> {
    let mpc = Command::new("mpc")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Couldn't run `mpc` to get song info");
    for line in BufReader::new(mpc.stdout.unwrap()).lines() {
        let line = line.unwrap_or_default();
        match parse_playback_info(&line.into_bytes()) {
            IResult::Done(_, playback_info) => return Some(playback_info),
            _ => continue,
        }
    }
    None
}

pub struct MPDMusic {
    last_value: Arc<RwLock<Format>>,
}

impl MPDMusic {
    pub fn new() -> MPDMusic {
        MPDMusic {
            last_value: Arc::new(RwLock::new(Format::Str(String::new()))),
        }
    }

    fn call_cmd(&self, cmd: &str) {
        let _ = Command::new("mpc").arg(cmd).stdout(Stdio::null()).status();
    }
}

impl MusicControl for MPDMusic {
    fn play(&self) {
        self.call_cmd("play")
    }

    fn pause(&self) {
        self.call_cmd("pause")
    }

    fn play_pause(&self) {
        self.call_cmd("toggle")
    }

    fn stop(&self) {
        self.call_cmd("stop")
    }

    fn next(&self) {
        self.call_cmd("next")
    }

    fn prev(&self) {
        self.call_cmd("prev")
    }
}

impl<F> MusicBackend<F> for MPDMusic
where
    F: Fn(SongInfo) -> Format + Sync + Send + 'static,
{
    fn current_value(&self) -> Format {
        (*self.last_value).read().unwrap().clone()
    }

    fn spawn_notifier(&mut self, tx: Sender<()>, updater: Arc<Box<F>>) {
        let last_value = self.last_value.clone();
        thread::spawn(move || {
            loop {
                let state = SongInfo {
                    title: mpc_get_format("%title%").unwrap_or_default(),
                    artist: mpc_get_format("%artist%").unwrap_or_default(),
                    album: mpc_get_format("%album%").unwrap_or_default(),
                    filename: mpc_get_format("%file%").unwrap_or_default(),
                    musicbrainz_track: None,
                    musicbrainz_artist: None,
                    musicbrainz_album: None,
                    playback: get_playback_info(),
                };

                // Scope for taking the RwLock
                {
                    let mut writer = last_value.write().unwrap();
                    *writer = (*updater)(state);
                    tx.send(());
                }

                // Wait for event
                let _ = Command::new("mpc")
                    .arg("idle")
                    .arg("player")
                    .stdout(Stdio::null())
                    .status();
            }
        });
    }
}

impl Default for MPDMusic {
    fn default() -> Self {
        Self::new()
    }
}

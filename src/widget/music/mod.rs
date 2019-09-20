use format::data::Format;
use std::sync::Arc;
use std::time::Duration;
use widget::base::{Sender, Widget};

pub mod mpd;
pub use self::mpd::MPDMusic;

#[cfg(feature = "dbus")]
pub mod mpris;
#[cfg(feature = "dbus")]
pub use self::mpris::MPRISMusic;

#[derive(Debug, Clone)]
pub struct PlaybackInfo {
    /// playing or paused
    pub playing: bool,

    pub progress: Duration,
    pub total: Duration,

    pub playlist_index: u32,
    pub playlist_total: u32,
}

#[derive(Debug, Clone)]
pub struct SongInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub filename: String,
    pub musicbrainz_track: Option<String>,
    pub musicbrainz_artist: Option<String>,
    pub musicbrainz_album: Option<String>,
    pub playback: Option<PlaybackInfo>,
}

pub struct Music<F: Fn(SongInfo) -> Format, B: MusicBackend<F>> {
    updater: Arc<Box<F>>,
    backend: B,
}

impl<F, B> Widget for Music<F, B>
where
    F: Fn(SongInfo) -> Format + Sync + Send + 'static,
    B: MusicBackend<F>,
{
    fn current_value(&self) -> Format {
        self.backend.current_value()
    }

    fn spawn_notifier(&mut self, tx: Sender<()>) {
        self.backend.spawn_notifier(tx, self.updater.clone());
    }
}

impl<F, B> Music<F, B>
where
    F: Fn(SongInfo) -> Format,
    B: MusicBackend<F>,
{
    pub fn new(backend: B, updater: F) -> Box<Music<F, B>> {
        Box::new(Music {
            updater: Arc::new(Box::new(updater)),
            backend,
        })
    }
}

pub trait MusicBackend<F: Fn(SongInfo) -> Format> {
    fn current_value(&self) -> Format;
    fn spawn_notifier(&mut self, tx: Sender<()>, updater: Arc<Box<F>>);
}

pub trait MusicControl {
    fn play(&self);
    fn pause(&self);
    fn play_pause(&self);
    fn stop(&self);
    fn next(&self);
    fn prev(&self);
}

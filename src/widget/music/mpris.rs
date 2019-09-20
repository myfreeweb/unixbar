use super::{MusicBackend, MusicControl, PlaybackInfo, SongInfo};
use dbus::arg::Array;
use dbus::{BusType, Connection, Message, MessageItem, Props};
use format::data::Format;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use widget::base::Sender;

fn find_player(bus: &Connection) -> Option<String> {
    let m = Message::new_method_call(
        "org.freedesktop.DBus",
        "/",
        "org.freedesktop.DBus",
        "ListNames",
    )
    .unwrap();
    let r = bus.send_with_reply_and_block(m, 2000).unwrap();
    let mut arr: Array<&str, _> = r.get1().unwrap();
    arr.find(|s| s.starts_with("org.mpris.MediaPlayer2."))
        .map(|s| s.to_owned())
}

fn extract_i64(item: &MessageItem) -> Option<i64> {
    match *item {
        MessageItem::Int16(x) => Some(i64::from(x)),
        MessageItem::Int32(x) => Some(i64::from(x)),
        MessageItem::Int64(x) => Some(x),
        MessageItem::UInt16(x) => Some(i64::from(x)),
        MessageItem::UInt32(x) => Some(i64::from(x)),
        MessageItem::Variant(ref x) => extract_i64(x),
        _ => None,
    }
}

fn extract_str(item: &MessageItem) -> Option<String> {
    match *item {
        MessageItem::Str(ref x) => Some((*x).clone()),
        MessageItem::Array(ref x, _) => Some(
            x.iter()
                .map(|y| extract_str(y).unwrap_or_default())
                .collect::<Vec<_>>()
                .join(", "),
        ),
        MessageItem::Variant(ref x) => extract_str(x),
        _ => None,
    }
}

fn get_entry<'a>(entries: &'a [MessageItem], key: &str) -> Option<&'a MessageItem> {
    entries
        .iter()
        .find(|e| match *e {
            MessageItem::DictEntry(ref k, _) => match **k {
                MessageItem::Str(ref x) if x == key => true,
                _ => false,
            },
            _ => false,
        })
        .and_then(|e| match *e {
            MessageItem::DictEntry(_, ref v) => Some(&**v),
            _ => None,
        })
}

pub struct MPRISMusic {
    last_value: Arc<RwLock<Format>>,
}

impl MPRISMusic {
    pub fn new() -> MPRISMusic {
        MPRISMusic {
            last_value: Arc::new(RwLock::new(Format::Str(String::new()))),
        }
    }

    fn call_method(&self, method: &str) {
        let bus = Connection::get_private(BusType::Session)
            .expect("Could not connect to D-Bus session bus for music control");
        if let Some(player) = find_player(&bus) {
            let m = Message::new_method_call(
                player,
                "/org/mpris/MediaPlayer2",
                "org.mpris.MediaPlayer2.Player",
                method,
            )
            .unwrap();
            let _ = bus.send(m);
        }
    }
}

impl MusicControl for MPRISMusic {
    fn play(&self) {
        self.call_method("Play")
    }

    fn pause(&self) {
        self.call_method("Pause")
    }

    fn play_pause(&self) {
        self.call_method("PlayPause")
    }

    fn stop(&self) {
        self.call_method("Stop")
    }

    fn next(&self) {
        self.call_method("Next")
    }

    fn prev(&self) {
        self.call_method("Previous")
    }
}

impl<F> MusicBackend<F> for MPRISMusic
where
    F: Fn(SongInfo) -> Format + Sync + Send + 'static,
{
    fn current_value(&self) -> Format {
        (*self.last_value).read().unwrap().clone()
    }

    fn spawn_notifier(&mut self, tx: Sender<()>, updater: Arc<Box<F>>) {
        let last_value = self.last_value.clone();
        thread::spawn(move || {
            let bus = Connection::get_private(BusType::Session)
                .expect("Could not connect to D-Bus session bus for music info");
            loop {
                if let Some(player) = find_player(&bus) {
                    if let Ok(props) = Props::new(
                        &bus,
                        player,
                        "/org/mpris/MediaPlayer2",
                        "org.mpris.MediaPlayer2.Player",
                        500,
                    )
                    .get_all()
                    {
                        if let Some(&MessageItem::Array(ref metas, _)) = props.get("Metadata") {
                            let state = SongInfo {
                                title: get_entry(metas, "xesam:title")
                                    .and_then(|m| extract_str(m))
                                    .unwrap_or_default(),
                                artist: get_entry(metas, "xesam:artist")
                                    .and_then(|m| extract_str(m))
                                    .unwrap_or_default(),
                                album: get_entry(metas, "xesam:album")
                                    .and_then(|m| extract_str(m))
                                    .unwrap_or_default(),
                                filename: get_entry(metas, "xesam:url")
                                    .and_then(|m| extract_str(m))
                                    .unwrap_or_default(),
                                musicbrainz_track: get_entry(metas, "xesam:musicBrainzTrackID")
                                    .and_then(|m| extract_str(m)),
                                musicbrainz_artist: get_entry(metas, "xesam:musicBrainzArtistID")
                                    .and_then(|m| extract_str(m)),
                                musicbrainz_album: get_entry(metas, "xesam:musicBrainzAlbumID")
                                    .and_then(|m| extract_str(m)),
                                playback: match props.get("PlaybackStatus") {
                                    Some(&MessageItem::Str(ref status)) => Some(PlaybackInfo {
                                        playing: status == "Playing",
                                        progress: Duration::from_millis(
                                            (props
                                                .get("Position")
                                                .and_then(|m| extract_i64(m))
                                                .unwrap_or(-1)
                                                / 1000)
                                                as u64,
                                        ),
                                        total: Duration::from_millis(
                                            (get_entry(&metas, "mpris:length")
                                                .and_then(|m| extract_i64(&m))
                                                .unwrap_or(-1)
                                                / 1000)
                                                as u64,
                                        ),
                                        playlist_index: 0,
                                        playlist_total: 0,
                                    }),
                                    _ => None,
                                },
                            };

                            let mut writer = last_value.write().unwrap();
                            *writer = (*updater)(state);
                            tx.send(());
                        }
                    }
                } else {
                    let mut writer = last_value.write().unwrap();
                    *writer = (*updater)(SongInfo {
                        title: "".to_owned(),
                        artist: "".to_owned(),
                        album: "".to_owned(),
                        filename: "".to_owned(),
                        musicbrainz_track: None,
                        musicbrainz_artist: None,
                        musicbrainz_album: None,
                        playback: None,
                    });
                    tx.send(());
                    thread::sleep(Duration::from_millis(1000)); // more sleepy without player
                }

                // Ideally, this would be smarter than constantly looping...
                // But the only signal in the Player interface is Seeked, no signals for any other
                // state change? Also, would need to detect players disappearing/appearing.
                thread::sleep(Duration::from_millis(500));
            }
        });
    }
}

impl Default for MPRISMusic {
    fn default() -> Self {
        Self::new()
    }
}

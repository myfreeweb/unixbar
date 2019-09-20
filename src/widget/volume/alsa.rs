use alsa::ctl::Ctl;
use alsa::mixer::{Mixer, SelemChannelId, SelemId};
use alsa::PollDescriptors;

use format::data::Format;
use widget::base::Sender;

use std::ffi::CString;
use std::sync::{Arc, RwLock};
use std::thread;

use libc::{poll, pollfd}; // TODO use mio/epoll?

use super::{VolumeBackend, VolumeState};

pub struct ALSA {
    last_value: Arc<RwLock<Format>>,
}

impl ALSA {
    pub fn new() -> ALSA {
        ALSA {
            last_value: Arc::new(RwLock::new(Format::Str(String::new()))),
        }
    }

    fn get_volume_state() -> VolumeState {
        let mixer = Mixer::new("default", false).unwrap();
        let selem_id = SelemId::new("Master", 0);
        let selem = mixer.find_selem(&selem_id).unwrap();
        let (min, max) = selem.get_playback_volume_range();
        let volume = selem
            .get_playback_volume(SelemChannelId::FrontLeft)
            .unwrap();
        let switch = selem
            .get_playback_switch(SelemChannelId::FrontLeft)
            .unwrap();
        VolumeState {
            volume: (volume as f64 / (max - min) as f64) as f32,
            muted: switch == 0,
        }
    }
}

impl<F> VolumeBackend<F> for ALSA
where
    F: Fn(VolumeState) -> Format + Sync + Send + 'static,
{
    fn current_value(&self) -> Format {
        (*self.last_value).read().unwrap().clone()
    }

    fn spawn_notifier(&mut self, tx: Sender<()>, updater: Arc<Box<F>>) {
        let ctl = Ctl::open(CString::new("default").unwrap().as_ref(), false).unwrap();
        ctl.subscribe_events(true).unwrap();

        let mut fds = Vec::<pollfd>::with_capacity(ctl.count());
        fds.resize(
            ctl.count(),
            pollfd {
                fd: 0,
                events: 0,
                revents: 0,
            },
        );
        let filled = ctl.fill(&mut fds).unwrap();
        assert!(filled == ctl.count());

        let initial_state = ALSA::get_volume_state();
        {
            let mut writer = self.last_value.write().unwrap();
            *writer = (*updater)(initial_state);
        }

        let last_value = self.last_value.clone();
        thread::spawn(move || loop {
            unsafe {
                let err = poll(fds.as_mut_ptr(), fds.len() as u64, -1);
                if err == -1 {
                    libc::perror(std::ptr::null());
                }
            }

            match ctl.read() {
                Ok(Some(_)) => {
                    let state = ALSA::get_volume_state();

                    let mut writer = last_value.write().unwrap();
                    *writer = (*updater)(state);
                    tx.send(());
                }
                _ => continue,
            }
        });
    }
}

impl Default for ALSA {
    fn default() -> Self {
        Self::new()
    }
}

pub fn default_volume() -> ALSA {
    ALSA::new()
}

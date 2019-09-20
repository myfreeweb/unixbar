use super::base::{Sender, Widget};
use format::data::Format;
use std::sync::{Arc, RwLock};
use std::thread;
use xcb;
use xcb::xkb;

pub struct Xkb<F: Fn(u8) -> Format> {
    last_value: Arc<RwLock<Format>>,
    last_id: Arc<RwLock<u8>>,
    formatter: Arc<Box<F>>,
}

impl<F> Widget for Xkb<F>
where
    F: Fn(u8) -> Format + Sync + Send + 'static,
{
    fn current_value(&self) -> Format {
        (*self.last_value).read().unwrap().clone()
    }

    fn spawn_notifier(&mut self, tx: Sender<()>) {
        let formatter = self.formatter.clone();
        let last_id = self.last_id.clone();
        let last_value = self.last_value.clone();
        thread::spawn(move || {
            if let Ok((conn, _)) = xcb::Connection::connect(None) {
                {
                    let cookie = xkb::use_extension(&conn, 1, 0);
                    match cookie.get_reply() {
                        Ok(r) => {
                            if !r.supported() {
                                return;
                            }
                        }
                        Err(_) => return,
                    }
                }
                {
                    let map_parts = xcb::xkb::MAP_PART_MODIFIER_MAP;
                    let events = xcb::xkb::EVENT_TYPE_STATE_NOTIFY;
                    let cookie = xkb::select_events_checked(
                        &conn,
                        xkb::ID_USE_CORE_KBD as u16,
                        events as u16,
                        0,
                        events as u16,
                        map_parts as u16,
                        map_parts as u16,
                        None,
                    );
                    let _ = cookie.request_check();
                }
                loop {
                    let event = conn.wait_for_event();
                    match event {
                        None => {
                            break;
                        }
                        Some(event) => {
                            let evt: &xkb::StateNotifyEvent = unsafe { xcb::cast_event(&event) };
                            let new_id = evt.group();
                            let mut id_writer = last_id.write().unwrap();
                            if *id_writer != new_id {
                                *id_writer = new_id;
                                let mut writer = last_value.write().unwrap();
                                *writer = (*formatter)(new_id);
                                tx.send(());
                            }
                        }
                    }
                }
            } else {
                return;
            }
        });
    }
}

impl<F> Xkb<F>
where
    F: Fn(u8) -> Format,
{
    pub fn new(formatter: F) -> Box<Xkb<F>> {
        let v = formatter(0);
        Box::new(Xkb {
            formatter: Arc::new(Box::new(formatter)),
            last_id: Arc::new(RwLock::new(0)),
            last_value: Arc::new(RwLock::new(v)),
        })
    }
}

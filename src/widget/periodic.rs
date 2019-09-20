use super::base::{Sender, Widget};
use format::data::Format;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

pub struct Periodic<F: Fn() -> Format> {
    interval: Duration,
    updater: Arc<Box<F>>,
    last_value: Arc<RwLock<Format>>,
}

impl<F> Widget for Periodic<F>
where
    F: Fn() -> Format + Sync + Send + 'static,
{
    fn current_value(&self) -> Format {
        (*self.last_value).read().unwrap().clone()
    }

    fn spawn_notifier(&mut self, tx: Sender<()>) {
        let interval = self.interval;
        let updater = self.updater.clone();
        let last_value = self.last_value.clone();
        thread::spawn(move || loop {
            thread::sleep(interval);
            {
                let mut writer = last_value.write().unwrap();
                *writer = (*updater)();
            }
            tx.send(());
        });
    }
}

impl<F> Periodic<F>
where
    F: Fn() -> Format,
{
    pub fn new(interval: Duration, updater: F) -> Box<Periodic<F>> {
        let v = updater();
        Box::new(Periodic {
            interval,
            updater: Arc::new(Box::new(updater)),
            last_value: Arc::new(RwLock::new(v)),
        })
    }
}

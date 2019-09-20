use super::base::{Sender, Widget};
use format::data::Format;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{io, thread};
use systemstat::DelayedMeasurement;

pub struct Delayed<T, F: Fn(io::Result<T>) -> Format, U: Fn() -> DelayedMeasurement<T>> {
    interval: Duration,
    updater: Arc<Box<U>>,
    formatter: Arc<Box<F>>,
    last_value: Arc<RwLock<Format>>,
}

impl<T, F, U> Widget for Delayed<T, F, U>
where
    F: Fn(io::Result<T>) -> Format + Sync + Send + 'static,
    U: Fn() -> DelayedMeasurement<T> + Sync + Send + 'static,
{
    fn current_value(&self) -> Format {
        (*self.last_value).read().unwrap().clone()
    }

    fn spawn_notifier(&mut self, tx: Sender<()>) {
        let interval = self.interval;
        let formatter = self.formatter.clone();
        let updater = self.updater.clone();
        let last_value = self.last_value.clone();
        thread::spawn(move || {
            let mut measurement = (*updater)();
            loop {
                thread::sleep(interval);
                {
                    let mut writer = last_value.write().unwrap();
                    *writer = (*formatter)(measurement.done());
                    measurement = (*updater)();
                }
                tx.send(());
            }
        });
    }
}

impl<T, F, U> Delayed<T, F, U>
where
    F: Fn(io::Result<T>) -> Format,
    U: Fn() -> DelayedMeasurement<T>,
{
    pub fn new(interval: Duration, updater: U, formatter: F) -> Box<Delayed<T, F, U>> {
        Box::new(Delayed {
            interval,
            updater: Arc::new(Box::new(updater)),
            formatter: Arc::new(Box::new(formatter)),
            last_value: Arc::new(RwLock::new(Format::Str("".to_owned()))),
        })
    }
}

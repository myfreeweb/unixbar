use super::base::{Sender, Widget};
use chrono::Local;
use format::data::Format;
use std::thread;
use std::time::Duration;

pub struct DateTime {
    format: String,
}

impl Widget for DateTime {
    fn current_value(&self) -> Format {
        Format::Str(Local::now().format(&self.format).to_string())
    }

    fn spawn_notifier(&mut self, tx: Sender<()>) {
        let seconds = if self.format.contains("%S") { 1 } else { 60 };
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(seconds));
            tx.send(());
        });
    }
}

impl DateTime {
    pub fn new(format: &str) -> Box<DateTime> {
        Box::new(DateTime {
            format: format.to_owned(),
        })
    }
}

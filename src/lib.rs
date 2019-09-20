extern crate chrono;
#[macro_use]
extern crate nom;
#[cfg(target_os = "linux")]
extern crate alsa;
#[cfg(feature = "dbus")]
extern crate dbus;
extern crate libc;
#[cfg(feature = "systemstat")]
extern crate systemstat;
#[cfg(feature = "xkb")]
extern crate xcb;
#[macro_use]
extern crate chan;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

pub mod format;
pub mod widget;

pub use format::*;
use std::collections::BTreeMap;
pub use widget::*;

pub struct UnixBar<F: Formatter> {
    formatter: F,
    widgets: Vec<Box<Widget>>,
    fns: BTreeMap<String, Box<FnMut()>>,
}

impl<F: Formatter> UnixBar<F> {
    pub fn new(formatter: F) -> UnixBar<F> {
        UnixBar {
            formatter,
            widgets: Vec::new(),
            fns: BTreeMap::new(),
        }
    }

    pub fn register_fn<Fn>(&mut self, name: &str, func: Fn) -> &mut UnixBar<F>
    where
        Fn: FnMut() + 'static,
    {
        self.fns.insert(name.to_owned(), Box::new(func));
        self
    }

    pub fn add(&mut self, widget: Box<Widget>) -> &mut UnixBar<F> {
        self.widgets.push(widget);
        self
    }

    pub fn run(&mut self) {
        let (wid_tx, wid_rx) = chan::async();
        for widget in &mut self.widgets {
            widget.spawn_notifier(wid_tx.clone());
        }
        self.show();
        let (stdin_tx, stdin_rx) = chan::async();
        std::thread::spawn(move || {
            let stdin = std::io::stdin();
            let mut line = String::new();
            loop {
                line.clear();
                if stdin.read_line(&mut line).is_ok() {
                    stdin_tx.send(line.clone());
                }
            }
        });
        loop {
            chan_select! {
                wid_rx.recv() => self.show(),
                stdin_rx.recv() -> line => self.formatter.handle_stdin(line, &mut self.fns),
            }
        }
    }

    fn show(&mut self) {
        let vals: Vec<Format> = self.widgets.iter().map(|ref w| w.current_value()).collect();
        let line = self.formatter.format_all(&vals);
        println!("{}", line.replace("\n", ""));
    }
}

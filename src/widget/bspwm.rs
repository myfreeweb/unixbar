use super::base::{Sender, Widget};
use format::data::Format;
use nom::IResult;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum WindowMode {
    Tiled,
    PseudoTiled,
    Floating,
    FullScreen,
    /// No window is focused
    None,
}
impl WindowMode {
    fn from_byte(byte: Option<&[u8]>) -> WindowMode {
        let byte = byte.map(|inner| inner[0] as char);
        match byte {
            Some('T') => WindowMode::Tiled,
            Some('P') => WindowMode::PseudoTiled,
            Some('F') => WindowMode::Floating,
            Some('=') => WindowMode::FullScreen,
            _ => WindowMode::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BspwmState {
    pub desktops: Vec<BspwmDesktop>,
    pub monocle: bool,
    pub window_mode: WindowMode,
}

#[derive(Debug, Clone)]
pub struct BspwmDesktop {
    pub name: String,
    pub occupied: bool,
    pub focused: bool,
    pub urgent: bool,
}

named!(bspstr<&[u8], BspwmState>,
    do_parse!(
        tag!("WM") >>
        take_until_and_consume!(":") >>
        d: many0!(
            do_parse!(
                mode: one_of!(&"oOfFuU"[..]) >>
                name: take_until_and_consume!(":") >>
                (BspwmDesktop {
                    name: String::from_utf8_lossy(name).into_owned(),
                    occupied: mode == 'o' || mode == 'O',
                    focused: mode == 'F' || mode == 'O' || mode == 'U',
                    urgent: mode == 'u' || mode == 'U',
                })
            )
        ) >>
        tag!("L") >>
        layout: take!(1) >>

        wmode: opt!(
            complete!(
                do_parse!(
                    tag!(":T") >>
                    wmode: take!(1) >>
                    tag!(":G") >>
                    (wmode)
                )
            )
        ) >>
        (BspwmState {
            desktops: d,
            monocle: layout == b"M",
            window_mode: WindowMode::from_byte(wmode),
        })
    )
);

pub struct Bspwm<F: Fn(BspwmState) -> Format> {
    updater: Arc<Box<F>>,
    last_value: Arc<RwLock<Format>>,
}

impl<F> Widget for Bspwm<F>
where
    F: Fn(BspwmState) -> Format + Sync + Send + 'static,
{
    fn current_value(&self) -> Format {
        (*self.last_value).read().unwrap().clone()
    }

    fn spawn_notifier(&mut self, tx: Sender<()>) {
        let updater = self.updater.clone();
        let last_value = self.last_value.clone();
        thread::spawn(move || {
            loop {
                // Should be possible to use the socket directly...
                let bspc = Command::new("bspc")
                    .arg("subscribe")
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("Couldn't run bspc");
                for line in BufReader::new(bspc.stdout.unwrap()).lines() {
                    let mut writer = last_value.write().unwrap();
                    let line = line.unwrap_or_default();
                    if let IResult::Done(_, result) = bspstr(&line.into_bytes()) {
                        *writer = (*updater)(result);
                        tx.send(());
                    }
                }
                thread::sleep(Duration::from_millis(500));
            }
        });
    }
}

impl<F> Bspwm<F>
where
    F: Fn(BspwmState) -> Format,
{
    pub fn new(updater: F) -> Box<Bspwm<F>> {
        Box::new(Bspwm {
            updater: Arc::new(Box::new(updater)),
            last_value: Arc::new(RwLock::new(Format::Str(String::new()))),
        })
    }
}

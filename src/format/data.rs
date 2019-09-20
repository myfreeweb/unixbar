use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    /// Button used to navigate backwards in browsers
    NavBack,
    /// Button used to navigate forwards in browsers
    NavForward,
}

impl MouseButton {
    pub fn to_number(self) -> u8 {
        use MouseButton::*;
        match self {
            Left => 1,
            Middle => 2,
            Right => 3,
            ScrollUp => 4,
            ScrollDown => 5,
            ScrollLeft => 6,
            ScrollRight => 7,
            NavBack => 8,
            NavForward => 9,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone)]
pub enum ClickAction {
    Function(MouseButton, String),
    ShellCommand(MouseButton, String),
}

impl ClickAction {
    pub fn to_string(&self) -> String {
        match self {
            &ClickAction::Function(ref mb, ref name) => format!("f{:?}{}", mb, name),
            &ClickAction::ShellCommand(ref mb, ref cmd) => format!("s{:?}{}", mb, cmd),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Format {
    UnescapedStr(String),
    Str(String),
    Concat(Vec<Box<Format>>),
    Align(Alignment, Box<Format>),
    FgColor(String, Box<Format>),
    BgColor(String, Box<Format>),
    Clickable(ClickAction, Box<Format>),
    NoSeparator(Box<Format>),
    Padding(i32, Box<Format>),
}

pub trait Formatter {
    fn format(&mut self, data: &Format) -> String;

    fn format_all(&mut self, data: &[Format]) -> String {
        let mut line = String::new();
        for f in data {
            line.push_str(self.format(f).as_ref());
        }
        line
    }

    fn handle_stdin(&self, _line: Option<String>, _fns: &mut BTreeMap<String, Box<FnMut()>>) {}
}

#[macro_export]
macro_rules! bfmt {
    (raw[$str:expr]) => { Format::UnescapedStr($str.to_owned()) };
    (text[$str:expr]) => { Format::Str($str.to_owned()) };
    (fmt[$($rest:tt)*]) => { Format::Str(format!($($rest)*)) };
    (left $($rest:tt)*) => { Format::Align(Alignment::Left, Box::new(bfmt!($($rest)*))) };
    (center $($rest:tt)*) => { Format::Align(Alignment::Center, Box::new(bfmt!($($rest)*))) };
    (right $($rest:tt)*) => { Format::Align(Alignment::Right, Box::new(bfmt!($($rest)*))) };
    (fg[$color:expr] $($rest:tt)*) => { Format::FgColor($color.to_owned(), Box::new(bfmt!($($rest)*))) };
    (bg[$color:expr] $($rest:tt)*) => { Format::BgColor($color.to_owned(), Box::new(bfmt!($($rest)*))) };
    (click[$btn:expr => fn $act:expr] $($rest:tt)*) => {
        Format::Clickable(ClickAction::Function($btn, $act.to_owned()), Box::new(bfmt!($($rest)*)))
    };
    (click[$btn:expr => sh $act:expr] $($rest:tt)*) => {
        Format::Clickable(ClickAction::ShellCommand($btn, $act.to_owned()), Box::new(bfmt!($($rest)*)))
    };
    (no_sep $($rest:tt)*) => { Format::NoSeparator(Box::new(bfmt!($($rest)*))) };
    (pad[$pad:expr] $($rest:tt)*) => { Format::Padding($pad, Box::new(bfmt!($($rest)*))) };
    (multi[$(($($rest:tt)*)),*]) => { Format::Concat(vec![ $( Box::new(bfmt!( $($rest)* )) ),* ]) };
    ($e:expr) => { $e };
    () => { Format::UnescapedStr("".to_owned()) };
}

use super::data::*;

#[derive(Default)]
pub struct Dzen2Formatter;

impl Formatter for Dzen2Formatter {
    fn format(&mut self, data: &Format) -> String {
        match *data {
            Format::UnescapedStr(ref s) => s.clone(),
            Format::Str(ref s) => s.replace("^", "^^"),
            Format::Concat(ref fs) => fs
                .iter()
                .map(|f| self.format(f))
                .fold("".to_owned(), |a, b| a + &b),
            Format::Align(_, ref f) => self.format(f), // :-(
            Format::FgColor(ref c, ref f) => format!("^fg({}){}^fg()", c, self.format(f)),
            Format::BgColor(ref c, ref f) => format!("^bg({}){}^bg()", c, self.format(f)),
            Format::NoSeparator(ref f) => self.format(f),
            Format::Padding(_, ref f) => self.format(f),
            Format::Clickable(ref act, ref f) => match *act {
                ClickAction::ShellCommand(ref mb, ref a) => {
                    format!("^ca({}, {}){}^ca()", mb.to_number(), a, self.format(f))
                }
                _ => self.format(f), // TODO
            },
        }
    }
}

impl Dzen2Formatter {
    pub fn new() -> Dzen2Formatter {
        Dzen2Formatter
    }
}

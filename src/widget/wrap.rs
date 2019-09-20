use super::base::{Sender, Widget};
use format::data::Format;

pub struct Wrap<W, F> {
    widget: Box<W>,
    wrapper: Box<F>,
}

impl<W, F> Widget for Wrap<W, F>
where
    W: Widget,
    F: Fn(Format) -> Format,
{
    fn current_value(&self) -> Format {
        (*self.wrapper)(self.widget.current_value())
    }

    fn spawn_notifier(&mut self, tx: Sender<()>) {
        self.widget.spawn_notifier(tx);
    }
}

impl<W, F> Wrap<W, F> {
    pub fn new(wrapper: F, widget: Box<W>) -> Box<Wrap<W, F>> {
        Box::new(Wrap {
            widget,
            wrapper: Box::new(wrapper),
        })
    }
}

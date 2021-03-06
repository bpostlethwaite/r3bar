use conrod::position::Place;
use conrod::{self, widget, Positionable, Widget};

pub struct Simple {
    pub text_id: conrod::widget::Id,
}

impl Simple {
    pub fn new(mut id_generator: conrod::widget::id::Generator) -> Simple {
        Simple{
            text_id: id_generator.next()
        }
    }

    pub fn render(&self, text: &str, bar_id: conrod::widget::Id, mut ui_widgets: &mut conrod::UiCell) {
        widget::Text::new(text)
            .x_place_on(bar_id, Place::Middle)
            .set(self.text_id, &mut ui_widgets);
    }
}

use conrod::{self, widget, Place, Positionable, Widget};

// TODO these should be configurable
const FONT_SIZE: conrod::FontSize = 14;
const LINE_SPACING: f64 = 2.5;

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
            .x_place_on(bar_id, Place::End(Some(10.0)))
            .line_spacing(LINE_SPACING)
            .font_size(FONT_SIZE)
            .set(self.text_id, &mut ui_widgets);
    }
}

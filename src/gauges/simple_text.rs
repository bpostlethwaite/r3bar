use conrod::{self, color, widget, Colorable, Positionable, Sizeable, Widget};

// TODO these should be configurable
const FONT_SIZE: conrod::FontSize = 14;
const LINE_SPACING: f64 = 2.5;
const PAD: f64 = 20.0;

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
            .color(color::LIGHT_GREEN)
            .padded_w_of(bar_id, PAD)
            .middle_of(bar_id)
            .align_text_middle()
            .line_spacing(LINE_SPACING)
            .font_size(FONT_SIZE)
            .set(self.text_id, &mut ui_widgets);
    }
}

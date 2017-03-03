use conrod::position::Place;
use conrod::widget::{self, Id};
use conrod::{self, Positionable, Sizeable, UiCell, Widget};

#[derive(Debug, Clone, Copy)]
pub struct Icon {
    pub w: f64,
    pub h: f64,
    pub id: conrod::image::Id,
    pub padding: f64,
}

pub struct Opts<'a> {
    pub maybe_icon: Option<Icon>,
    pub maybe_text: Option<&'a str>,
}

pub struct IconText {
    pub text_id: Id,
    pub icon_id: Id,
}

const LEFT_PAD: f64 = 8.0;

impl IconText {
    pub fn new(mut id_generator: widget::id::Generator) -> Self {
        IconText{
            text_id: id_generator.next(),
            icon_id: id_generator.next()
        }
    }

    pub fn render(&self, opts: Opts, bar_id: Id, mut ui_widgets: &mut UiCell) {

        if let Some(icon) = opts.maybe_icon {
            widget::Image::new(icon.id)
                .w_h(icon.w, icon.h)
                .x_place_on(bar_id, Place::Start(Some(LEFT_PAD)))
                .set(self.icon_id, &mut ui_widgets);

            if let Some(text) = opts.maybe_text {
                widget::Text::new(text)
                    .parent(bar_id)
                    .x_relative(icon.w + LEFT_PAD)
                    .set(self.text_id, &mut ui_widgets);
            }

        } else if let Some(text) = opts.maybe_text {
            widget::Text::new(text)
                .x_place_on(bar_id, Place::Start(Some(LEFT_PAD)))
                .set(self.text_id, &mut ui_widgets);
        }
    }
}

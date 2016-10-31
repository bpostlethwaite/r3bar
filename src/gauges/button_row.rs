use conrod::{self, color, widget, Colorable, Positionable,
             Sizeable, Labelable, Widget, Place};
use std::cmp;

// TODO these should be configurable
const FONT_SIZE: conrod::FontSize = 14;
const LINE_SPACING: f64 = 2.5;
const PAD: f64 = 20.0;

// TODO Height should be detected by font height
const BUTTON_WIDTH: f64 = 40.;
const HEIGHT: u32 = 30; // needs to come in via rubar


pub struct ButtonRow {
    ids: Vec<conrod::widget::Id>
}

impl ButtonRow {

    pub fn new(mut id_generator: conrod::widget::id::Generator) -> ButtonRow {
        let mut ids = Vec::new();

        for _ in 0..9 {
            ids.push(id_generator.next());
        }

        ButtonRow{
            ids: ids,
        }
    }

    pub fn render(&self, buttons: Vec<(String, color::Color)>, bar_id: conrod::widget::Id, mut ui_widgets: &mut conrod::UiCell) -> Option<i64> {

        let basic_btn = || {
            widget::Button::new()
                .parent(bar_id)
                .w(BUTTON_WIDTH)
                .h(HEIGHT as f64)
                .align_label_middle()
        };

        let mut clicked_button: Option<i64> = None;

        // we have preallocated 9 ids but we only need buttons.len() of them
        let ids = self.ids.split_at(buttons.len()).0;

        // zip so we get min(len(ids), len(titles))
        let mut ids_titles = ids.iter().enumerate().zip(buttons);

        // place the first button at the start of the block
        if let Some(((i, &button_id), (title, color))) = ids_titles.next() {
            let btn = basic_btn();
            if btn.x_place_on(bar_id, Place::Start(None))
                .color(color)
                .label(&title)
                .set(button_id, &mut ui_widgets)
                .was_clicked() {
                    clicked_button = Some(i as i64);
                }
        }

        // and then line subsequent buttons up relative to first button
        for ((i, &button_id), (title, color)) in ids_titles {
            let btn = basic_btn();
            if btn.x_relative(BUTTON_WIDTH)
                .color(color)
                .label(&title)
                .set(button_id, &mut ui_widgets)
                .was_clicked() {
                    clicked_button = Some(i as i64);
                }
        }

        clicked_button
    }
}

use conrod::{self, color, widget, Colorable, Color, Positionable, Sizeable, Labelable, Widget, Place};
use std::cmp;


pub struct ButtonRow {
    ids: Vec<conrod::widget::Id>,
    height: u32,
    font_color: Color
}

impl ButtonRow {
    pub fn new(height: u32, color: Color, mut id_generator: conrod::widget::id::Generator) -> ButtonRow {
        let mut ids = Vec::new();

        for _ in 0..9 {
            ids.push(id_generator.next());
        }

        ButtonRow { ids: ids, height: height, font_color: color }
    }

    pub fn render(&self,
                  buttons: Vec<(String, color::Color)>,
                  bar_id: conrod::widget::Id,
                  mut ui_widgets: &mut conrod::UiCell)
                  -> Option<i64> {

        let basic_btn = || {
            widget::Button::new()
                .parent(bar_id)
                .label_color(self.font_color)
                .w(self.height as f64)
                .h(self.height as f64)
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
                .small_font(ui_widgets)
                .set(button_id, &mut ui_widgets)
                .was_clicked() {
                clicked_button = Some(i as i64);
            }
        }

        // and then line subsequent buttons up relative to first button
        for ((i, &button_id), (title, color)) in ids_titles {
            let btn = basic_btn();
            if btn.x_relative(self.height as f64)
                .color(color)
                .label(&title)
                .small_font(ui_widgets)
                .set(button_id, &mut ui_widgets)
                .was_clicked() {
                clicked_button = Some(i as i64);
            }
        }

        clicked_button
    }
}

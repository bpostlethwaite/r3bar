use conrod::position::Place;
use conrod::{self, color, widget, Colorable, Color, Positionable, Sizeable, Labelable, Widget};

pub struct ButtonRow {
    ids: Vec<conrod::widget::Id>,
    label_id: conrod::widget::Id,
    height: u32,
    label_color: Color,
    button_label_color: Color,
}

impl ButtonRow {
    pub fn new(height: u32,
               button_label_color: Color,
               label_color: Color,
               mut id_generator: conrod::widget::id::Generator)
               -> ButtonRow {
        let mut ids = Vec::new();

        for _ in 0..9 {
            ids.push(id_generator.next());
        }

        ButtonRow {
            ids: ids,
            height: height,
            button_label_color: button_label_color,
            label_color: label_color,
            label_id: id_generator.next(),
        }
    }

    pub fn render(&self,
                  buttons: Vec<(String, color::Color)>,
                  label: &str,
                  bar_id: conrod::widget::Id,
                  mut ui_widgets: &mut conrod::UiCell)
                  -> Option<i64> {

        let basic_btn = || {
            widget::Button::new()
                .parent(bar_id)
                .label_color(self.button_label_color)
                .w(self.height as f64)
                .h(self.height as f64)
                .center_justify_label()
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
            if btn.x_relative(self.height as f64)
                .color(color)
                .label(&title)
                .set(button_id, &mut ui_widgets)
                .was_clicked() {
                clicked_button = Some(i as i64);
            }
        }

        widget::Text::new(label)
            .x_place_on(bar_id, Place::End(Some(10.)))
            .color(self.label_color)
            .set(self.label_id, &mut ui_widgets);

        clicked_button
    }
}

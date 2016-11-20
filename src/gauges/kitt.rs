use conrod::{self, widget, Place, Positionable, Widget};
use conrod::color::{Color};

use bar::{Animate};

pub struct RedKitt {
    pub text_id: conrod::widget::Id,
}



const color_grad: Vec<Color> = vec![
    Color::Rgba(1., 0., 0., 1.00),
    Color::Rgba(1., 0., 0., 0.35),
    Color::Rgba(1., 0., 0., 0.15),
    Color::Rgba(1., 0., 0., 0.05),
    Color::Rgba(1., 0., 0., 0.02),
    Color::Rgba(1., 0., 0., 0.01),
    Color::Rgba(1., 0., 0., 0.00),
    Color::Rgba(1., 0., 0., 0.00),
];


impl RedKitt {
    pub fn new(mut id_generator: conrod::widget::id::Generator) -> Simple {
        Simple{
            text_id: id_generator.next()
        }
    }

    pub fn render(&self, animate: Animate, bar_id: conrod::widget::Id, mut ui_widgets: &mut conrod::UiCell) {
        widgets::kitt::Kitt::new()
            .color(conrod::color::Color::Rgba(0., 0.168627, 0.211764, 1.))
            .wh_of(bar_id)
            .gradient(color_grad)
            .animate(kitt_animator)
            .middle_of(slot_id)
            .set(kitt_id, ui_widgets);
    }
}

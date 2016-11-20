use conrod::{self, widget, Place, Positionable, Sizeable, Widget};
use conrod::color::{Color};
use conrod::widget::{Id};

use widgets;

use bar::{Animate};

pub struct RedKitt {
    pub kitt_id: conrod::widget::Id,
    color_grad: Vec<Color>,
}

impl RedKitt {
    pub fn new(mut id_generator: conrod::widget::id::Generator) -> Self {
        RedKitt{
            kitt_id: id_generator.next(),
            color_grad: vec![
                Color::Rgba(1., 0., 0., 1.00),
                Color::Rgba(1., 0., 0., 0.35),
                Color::Rgba(1., 0., 0., 0.15),
                Color::Rgba(1., 0., 0., 0.05),
                Color::Rgba(1., 0., 0., 0.02),
                Color::Rgba(1., 0., 0., 0.01),
                Color::Rgba(1., 0., 0., 0.00),
                Color::Rgba(1., 0., 0., 0.00),
            ]
        }
    }

    pub fn render(&self, animator: Option<Animate>, slot_id: Id, mut ui: &mut conrod::UiCell) -> Option<()> {
        widgets::kitt::Kitt::new(&self.color_grad)
            .wh_of(slot_id)
            .padding(4.0)
            .animate(animator)
            .middle_of(slot_id)
            .set(self.kitt_id, ui)
    }
}

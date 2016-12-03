use conrod::{self, Positionable, Sizeable, Widget};
use conrod::color::{Color};
use conrod::widget::{Id};

use widgets;

use animate::{Animate};

pub struct RedKitt {
    pub kitt_id: conrod::widget::Id,
    color_grad: Vec<Color>,
    animate: Animate
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
            ],
            animate: Animate::new(),
        }
    }

    pub fn render(&self, do_animate: bool, slot_id: Id, mut ui: &mut conrod::UiCell) -> Option<()> {

        {
            let mut animator = self.animate;
            animator.next_frame();
        }

        let animator = match do_animate {
            true => Some(self.animate),
            false => None,
        };

        widgets::kitt::Kitt::new(&self.color_grad)
            .wh_of(slot_id)
            .padding(4.0)
            .animate(animator)
            .middle_of(slot_id)
            .set(self.kitt_id, ui)
    }
}

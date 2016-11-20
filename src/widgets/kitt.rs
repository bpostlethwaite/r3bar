use conrod::{self, widget, Colorable, Place, Positionable, Scalar, Widget};

use animate::Animate;

const FRAMES_PER_TICK: i64 = 8;

/// The type upon which we'll implement the `Widget` trait.
pub struct Kitt<'a> {
    common: widget::CommonBuilder,
    padding: Scalar,
    style: Style,
    animate: Option<Animate>,
    gradient: &'a Vec<conrod::color::Color>,
    enabled: bool // respond to user input?

}

widget_style!{
    /// Represents the unique styling for our Kitt widget.
    style Style {
        /// Color of the button.
        - background_color: conrod::Color { theme.background_color }
        - color: conrod::Color { theme.shape_color }
    }
}

/// Represents the unique, cached state for our Kitt widget.
pub struct State {
    ids: Vec<conrod::widget::Id>,
    rect_id: conrod::widget::Id,
    iter: i64
}

impl<'a> Kitt<'a>{
    /// Create a button context to be built upon.
    pub fn new(gradient: &'a Vec<conrod::color::Color>) -> Kitt<'a> {
        Kitt {
            common: widget::CommonBuilder::new(),
            padding: 2.0,
            style: Style::new(),
            animate: None,
            gradient: gradient,
            enabled: true,
        }
    }

    #[allow(dead_code)]
    pub fn padding(mut self, padding: Scalar) -> Self {
        self.padding = padding;
        self
    }

    pub fn animate(mut self, animate: Option<Animate>) -> Self {
        self.animate = animate;
        self
    }

    #[allow(dead_code)]
    pub fn gradient(mut self, grad: &'a Vec<conrod::color::Color>) -> Self {
        self.gradient = grad;
        self
    }

    #[allow(dead_code)]
    pub fn enabled(mut self, flag: bool) -> Self {
        self.enabled = flag;
            self
    }
}

impl<'a> Widget for Kitt<'a> {

    type State = State;

    type Style = Style;

    type Event = Option<()>;

    fn common(&self) -> &widget::CommonBuilder {
        &self.common
    }

    fn common_mut(&mut self) -> &mut widget::CommonBuilder {
        &mut self.common
    }

    fn init_state(&self, mut id_gen: widget::id::Generator) -> Self::State {
        let mut ids = Vec::new();

        for _ in 0..8 {
            ids.push(id_gen.next());
        }

        State { ids: ids, rect_id: id_gen.next(), iter: 0}
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Option<()> {
        let widget::UpdateArgs { id, state, rect, mut ui, style, .. } = args;

        let n = state.ids.len() as f64;
        let cell_width = rect.w() / n;
        let radius = cell_width / 2. - self.padding;

        // set default animate index out of range
        let ni = n as i64;
        let mut animate_index = 0;
        if let Some(animate) = self.animate {
            animate_index = state.iter;

            if animate_index >= ni {
                animate_index = 2 * ni - animate_index - 2;
            }

            // only update our animation loop every N frames
            if animate.tick(FRAMES_PER_TICK) {
                state.update(|state| {
                    state.iter = (state.iter + 1) % (2 * ni - 2);
                });
            }
        }

        let color = style.background_color(&ui.theme);
        let mut ids = state.ids.iter();
        let mut circ_index = 0;
        // draw the backing plate and the first circle
        if let Some(&circ_id) = ids.next() {
            widget::Rectangle::fill_with(rect.dim(), color)
                .middle_of(id)
                .graphics_for(id)
                .set(state.rect_id, ui);

            let mut c_color = color;
            if let Some(_) = self.animate {
                let c_index = (circ_index - animate_index).abs();
                c_color = self.gradient[c_index as usize];
            }

            circ_index += 1;
            widget::Circle::fill(radius)
                .x_place_on(state.rect_id, Place::Start(Some(self.padding)))
                .graphics_for(id)
                .color(c_color)
                .set(circ_id, ui);
        }

        for &circ_id in ids {

            let mut c_color = color;
            if let Some(_) = self.animate {
                let c_index = (circ_index - animate_index).abs();
                c_color = self.gradient[c_index as usize];
            }

            widget::Circle::fill(radius)
                .x_relative(cell_width)
                .graphics_for(id)
                .color(c_color)
                .set(circ_id, ui);

            circ_index += 1;
        }

        let input = ui.widget_input(id);
        // If the button was clicked, produce `Some` event.
        input.clicks().left().next().map(|_| ())

    }
}

/// Provide the chainable color() configuration method.
impl<'a> Colorable for Kitt<'a> {
    fn color(mut self, color: conrod::Color) -> Self {
        self.style.color = Some(color);
        self
    }
}

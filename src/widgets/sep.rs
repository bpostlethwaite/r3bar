use conrod::{self, widget, Borderable, Colorable, Positionable, Scalar, Widget};
use conrod::position::{Place};

/// The type upon which we'll implement the `Widget` trait.
pub struct Sep {
    common: widget::CommonBuilder,
    padding: Scalar,
    style: Style,
}

const DEFAULT_PADDING: f64 = 2.0;
const DEFAULT_SEP_WIDTH: f64 = 2.0;

widget_style!{
    /// Represents the unique styling for our Sep widget.
    style Style {
        /// Color of the button.
        - background_color: conrod::Color { theme.background_color }
        - border_color: conrod::Color { theme.border_color }
        - border_width: conrod::Scalar { theme.border_width }
    }
}

/// Represents the unique, cached state for our Sep widget.
pub struct State {
    rect_id: conrod::widget::Id,
    line_id: conrod::widget::Id,
}

impl Sep{
    /// Create a button context to be built upon.
    pub fn new() -> Sep {
        Sep {
            common: widget::CommonBuilder::new(),
            padding: DEFAULT_PADDING,
            style: Style::new(),
        }
    }

    #[allow(dead_code)]
    pub fn padding(mut self, padding: Scalar) -> Self {
        self.padding = padding;
        self
    }
}

impl Widget for Sep {

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
        State { line_id: id_gen.next(), rect_id: id_gen.next() }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Option<()> {
        let widget::UpdateArgs { id, state, rect, mut ui, style, .. } = args;

        let r_color = style.background_color(&ui.theme);
        let l_color = style.border_color(&ui.theme);

        let dims = rect.dim();
        let handle_dim = [DEFAULT_SEP_WIDTH, dims[1]];

        widget::Rectangle::fill_with(dims, r_color)
            .graphics_for(id)
            .middle_of(id)
            .place_on_kid_area(true)
            .set(state.rect_id, ui);

        widget::Rectangle::fill_with(handle_dim, l_color)
            .graphics_for(id)
            .middle_of(state.rect_id)
            .place_on_kid_area(true)
            .set(state.line_id, ui);

        let input = ui.widget_input(id);

        // If the seperator was clicked, produce `Some` event.
        input.clicks().left().next().map(|_| ())
    }
}

/// Provide the chainable color() configuration method.
impl Borderable for Sep {
    fn border(mut self, width: f64) -> Self {
        self.style.border_width = Some(width);
        self
    }

    fn border_color(mut self, color: conrod::Color) -> Self {
        self.style.border_color = Some(color);
        self
    }
}

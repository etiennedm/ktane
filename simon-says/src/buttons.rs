//! Physical I/O for the module: the four push-buttons.
//!
//! | Color  | Button GPIO | LED index |
//! |--------|-------------|-----------|
//! | GREEN  | GPIO11      | 0         |
//! | RED    | GPIO10      | 1         |
//! | BLUE   | GPIO6       | 2         |
//! | YELLOW | GPIO7       | 3         |

use esp_hal::gpio::{Input, InputConfig, InputPin, Pull};

/// The four push-buttons, each on its own GPIO with an internal pull-up.
/// Buttons are active-low: a press pulls the pin to GND.
pub struct Buttons<'d> {
    green: Input<'d>,
    red: Input<'d>,
    blue: Input<'d>,
    yellow: Input<'d>,
}

impl<'d> Buttons<'d> {
    pub fn new(
        green: impl InputPin + 'd,
        red: impl InputPin + 'd,
        blue: impl InputPin + 'd,
        yellow: impl InputPin + 'd,
    ) -> Self {
        let cfg = InputConfig::default().with_pull(Pull::Up);
        Self {
            green: Input::new(green, cfg),
            red: Input::new(red, cfg),
            blue: Input::new(blue, cfg),
            yellow: Input::new(yellow, cfg),
        }
    }

    /// Sample every button, returning whether each is currently pressed.
    /// The array is in LED-index order (green, red, blue, yellow), matching
    /// the table above. Buttons are active-low, so a low pin means pressed.
    pub fn pressed(&self) -> [bool; 4] {
        [
            self.green.is_low(),
            self.red.is_low(),
            self.blue.is_low(),
            self.yellow.is_low(),
        ]
    }
}

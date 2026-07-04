use embedded_hal_async::i2c::I2c;
use log::info;

#[allow(dead_code)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum BrightnessLevel {
    // Flip bits since LSB first is expected by the TM1637 but I2C is sending MSB first
    Level0 = 0, // 0b000 -> 0b000
    Level1 = 4, // 0b001 -> 0b100
    Level2 = 2, // 0b010 -> 0b010
    Level3 = 6, // 0b011 -> 0b110
    Level4 = 1, // 0b100 -> 0b001
    Level5 = 5, // 0b101 -> 0b101
    Level6 = 3, // 0b110 -> 0b011
    Level7 = 7, // 0b111 -> 0b111
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum SevenSegmentChar {
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Blank,
}

impl From<u8> for SevenSegmentChar {
    fn from(value: u8) -> Self {
        match value {
            0 => SevenSegmentChar::Digit0,
            1 => SevenSegmentChar::Digit1,
            2 => SevenSegmentChar::Digit2,
            3 => SevenSegmentChar::Digit3,
            4 => SevenSegmentChar::Digit4,
            5 => SevenSegmentChar::Digit5,
            6 => SevenSegmentChar::Digit6,
            7 => SevenSegmentChar::Digit7,
            8 => SevenSegmentChar::Digit8,
            9 => SevenSegmentChar::Digit9,
            _ => SevenSegmentChar::Blank
        }
    }
}

impl From<SevenSegmentChar> for u8 {
    fn from(value: SevenSegmentChar) -> Self {
        match value {
            SevenSegmentChar::Digit0 => 0b1111_1100,
            SevenSegmentChar::Digit1 => 0b0110_0000,
            SevenSegmentChar::Digit2 => 0b1101_1010,
            SevenSegmentChar::Digit3 => 0b1111_0010,
            SevenSegmentChar::Digit4 => 0b0110_0110,
            SevenSegmentChar::Digit5 => 0b1011_0110,
            SevenSegmentChar::Digit6 => 0b1011_1110,
            SevenSegmentChar::Digit7 => 0b1110_0000,
            SevenSegmentChar::Digit8 => 0b1111_1110,
            SevenSegmentChar::Digit9 => 0b1111_0110,
            SevenSegmentChar::Blank => 0b0000_0000,
        }
    }
}

pub struct TM1637<I2C> {
    i2c: I2C,
    brightness_level: BrightnessLevel,
    display_on: bool,
    colon_on: bool,
    digits: [SevenSegmentChar; 4],
}

impl<I2C> TM1637<I2C>
where
    I2C: I2c<Error: core::fmt::Display>,
{
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            brightness_level: BrightnessLevel::Level0,
            display_on: false,
            colon_on: false,
            digits: [SevenSegmentChar::Blank; 4],
        }
    }

    pub fn with_brightness_level(&mut self, brightness_level: BrightnessLevel) -> &mut Self {
        self.brightness_level = brightness_level;
        self
    }

    pub fn with_display_on(&mut self) -> &mut Self {
        self.display_on = true;
        self
    }

    #[allow(dead_code)]
    pub fn with_display_off(&mut self) -> &mut Self {
        self.display_on = false;
        self
    }

    pub fn with_colon_on(&mut self, colon_on: bool) -> &mut Self {
        self.colon_on = colon_on;
        self
    }

    pub fn with_digits(&mut self, digits: [SevenSegmentChar; 4]) -> &mut Self {
        self.digits = digits;
        self
    }

    pub async fn refresh(&mut self) {
        if let Err(e) = self.i2c.write(0x01, &[]).await {
            info!("I2C write error for command: {}", e);
        }

        // Invalid command (LSB = 00) followed by data for C0H-C3H, adding a dummy 0xFF last makes it
        // somehow loop around to accept the next update.
        let mut digit_data = [0xFF; 5];
        for (i, digit) in self.digits.iter().enumerate() {
            digit_data[i] = (*digit).into();
        }

        if self.colon_on {
            digit_data[1] += 1;
        }

        if let Err(e) = self
            .i2c
            .write(
                0x00, /* 0xFF on bus */
                &digit_data,
            )
            .await
        {
            info!("I2C write error for command: {}", e);
        }

        // Send display control command to enable display and set brightness level
        let display_cmd = (self.display_on as u8) << 3 | (self.brightness_level as u8) << 4;
        if let Err(e) = self.i2c.read(display_cmd, &mut [0; 1]).await {
            info!("I2C write error for display: {}", e);
        }
    }
}

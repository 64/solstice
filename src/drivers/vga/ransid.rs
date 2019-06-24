const ESC: u8 = b'\x1B';

pub enum State {
    Esc,
    Bracket,
    Parse,
    BgColor,
    FgColor,
    Equals,
    EndVal,
}

pub struct RansidState {
    pub state: State,
    pub style: u8,
    pub next_style: u8,
}

pub struct ColorChar {
    pub style: u8,
    pub ascii: u8,
}

fn convert_color(color: u8) -> u8 {
    let lookup_table: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];
    lookup_table[color as usize]
}

impl RansidState {
    pub fn new() -> Self {
        Self {
            state: State::Esc,
            style: 0x02,
            next_style: 0x02,
        }
    }

    pub fn ransid_process(&mut self, x: u8) -> ColorChar {
        let mut rv = ColorChar {
            style: self.style,
            ascii: b'\0',
        };
        match self.state {
            State::Esc => {
                if x == ESC {
                    self.state = State::Bracket;
                } else {
                    rv.ascii = x;
                }
            }
            State::Bracket => {
                if x == b'[' {
                    self.state = State::Parse;
                } else {
                    self.state = State::Esc;
                    rv.ascii = x;
                }
            }
            State::Parse => {
                if x == b'3' {
                    self.state = State::FgColor;
                } else if x == b'4' {
                    self.state = State::BgColor;
                } else if x == b'0' {
                    self.state = State::EndVal;
                    self.next_style = 0x0F;
                } else if x == b'1' {
                    self.state = State::EndVal;
                    self.next_style |= 1 << 3;
                } else if x == b'=' {
                    self.state = State::Equals;
                } else {
                    self.state = State::Esc;
                    self.next_style = self.style;
                    rv.ascii = x;
                }
            }
            State::BgColor => {
                if x >= b'0' && x <= b'7' {
                    self.state = State::EndVal;
                    self.next_style &= 0x1F;
                    self.next_style |= convert_color(x - b'0') << 4;
                } else {
                    self.state = State::Esc;
                    self.next_style = self.style;
                    rv.ascii = x;
                }
            }
            State::FgColor => {
                if x >= b'0' && x <= b'7' {
                    self.state = State::EndVal;
                    self.next_style &= 0xF8;
                    self.next_style |= convert_color(x - b'0');
                } else {
                    self.state = State::Esc;
                    self.next_style = self.style;
                    rv.ascii = x;
                }
            }
            State::Equals => {
                if x == b'1' {
                    self.state = State::EndVal;
                    self.next_style &= !(1 << 3);
                } else {
                    self.state = State::Esc;
                    self.next_style = self.style;
                    rv.ascii = x;
                }
            }
            State::EndVal => {
                if x == b';' {
                    self.state = State::Parse;
                } else if x == b'm' {
                    self.state = State::Esc;
                    self.style = self.next_style;
                } else {
                    self.state = State::Esc;
                    self.next_style = self.style;
                }
            }
        };

        rv
    }
}

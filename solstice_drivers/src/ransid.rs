use crate::{ransid, vga_text};

pub enum State {
    RANSID_ESC,
    RANSID_BRACKET,
    RANSID_PARSE,
    RANSID_BGCOLOR,
    RANSID_FGCOLOR,
    RANSID_EQUALS,
    RANSID_ENDVAL,
}

const ESC: u8 = b'\x1B';

pub struct Ransid_State {
    pub state: State,
    pub style: u8,
    pub next_style: u8,

}

pub struct Color_Char {
    pub style: u8,
    pub ascii: u8,
}

fn ransid_convert_color(color: u8) -> u8 {
    let lookup_table: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];
    lookup_table[color as usize]
}

impl Ransid_State {
    pub fn ransid_process(&mut self, x: u8) -> Color_Char {
        let mut rv = Color_Char {
            style: self.style,
            ascii: '\0' as u8,
        };
        match self.state {
            State::RANSID_ESC => {
                if x == ESC {
                    self.state = State::RANSID_BRACKET;
                } else {
                    rv.ascii = x;
                }
            }
            State::RANSID_BRACKET => {
                if x == b'[' {
                    self.state = State::RANSID_PARSE;
                } else {
                    self.state = State::RANSID_ESC;
                    rv.ascii = x;
                }
            }
            State::RANSID_PARSE => {
                if x == b'3' {
                    self.state = State::RANSID_FGCOLOR;
                } else if x == b'4' {
                    self.state = State::RANSID_BGCOLOR;
                } else if x == b'0' {
                    self.state = State::RANSID_ENDVAL;
                    self.next_style = 0x0F;
                } else if x == b'1' {
                    self.state = State::RANSID_ENDVAL;
                    self.next_style |= 1 << 3;
                } else if x == b'=' {
                    self.state = State::RANSID_EQUALS;
                } else {
                    self.state = State::RANSID_ESC;
                    self.next_style = self.style;
                    rv.ascii = x;
                }
            }
            State::RANSID_BGCOLOR => {
                if x >= b'0' && x <= b'7' {
                    self.state = State::RANSID_ENDVAL;
                    self.next_style &= 0x1F;
                    self.next_style |= ransid_convert_color(x - b'0') << 4;
                } else {
                    self.state = State::RANSID_ESC;
                    self.next_style = self.style;
                    rv.ascii = x;
                }
            }
            State::RANSID_FGCOLOR => {
                if x >= b'0' && x <= b'7' {
                    self.state = State::RANSID_ENDVAL;
                    self.next_style &= 0xF8;
                    self.next_style |= ransid_convert_color(x - b'0');
                } else {
                    self.state = State::RANSID_ESC;
                    self.next_style = self.style;
                    rv.ascii = x;
                }
            }
            State::RANSID_EQUALS => {
                if x == b'1' {
                    self.state = State::RANSID_ENDVAL;
                    self.next_style &= !(1 << 3);
                } else {
                    self.state = State::RANSID_ESC;
                    self.next_style = self.style;
                    rv.ascii = x;
                }
            }
            State::RANSID_ENDVAL => {
                if x == b';' {
                    self.state = State::RANSID_PARSE;
                } else if x == b'm' {
                    self.state = State::RANSID_ESC;
                    self.style = self.next_style;
                } else {
                    self.state = State::RANSID_ESC;
                    self.next_style = self.style;
                }
            }
        };
        rv
    }
}

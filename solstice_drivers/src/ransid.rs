use crate::{ransid, vga_text};

pub enum State {
    RANSIDESC,
    RANSIDBRACKET,
    RANSIDPARSE,
    RANSIDBGCOLOR,
    RANSIDFGCOLOR,
    RANSIDEQUALS,
    RANSIDENDVAL,
}

const ESC: u8 = b'\x1B';

pub struct Ransid_State {
    pub state: State,
    pub style: u8,
    pub nextstyle: u8,
}

pub struct Color_Char {
    pub style: u8,
    pub ascii: u8,
}

fn convert_color(color: u8) -> u8 {
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
            State::RANSIDESC => {
                if x == ESC {
                    self.state = State::RANSIDBRACKET;
                } else {
                    rv.ascii = x;
                }
            }
            State::RANSIDBRACKET => {
                if x == b'[' {
                    self.state = State::RANSIDPARSE;
                } else {
                    self.state = State::RANSIDESC;
                    rv.ascii = x;
                }
            }
            State::RANSIDPARSE => {
                if x == b'3' {
                    self.state = State::RANSIDFGCOLOR;
                } else if x == b'4' {
                    self.state = State::RANSIDBGCOLOR;
                } else if x == b'0' {
                    self.state = State::RANSIDENDVAL;
                    self.nextstyle = 0x0F;
                } else if x == b'1' {
                    self.state = State::RANSIDENDVAL;
                    self.nextstyle |= 1 << 3;
                } else if x == b'=' {
                    self.state = State::RANSIDEQUALS;
                } else {
                    self.state = State::RANSIDESC;
                    self.nextstyle = self.style;
                    rv.ascii = x;
                }
            }
            State::RANSIDBGCOLOR => {
                if x >= b'0' && x <= b'7' {
                    self.state = State::RANSIDENDVAL;
                    self.nextstyle &= 0x1F;
                    self.nextstyle |= convert_color(x - b'0') << 4;
                } else {
                    self.state = State::RANSIDESC;
                    self.nextstyle = self.style;
                    rv.ascii = x;
                }
            }
            State::RANSIDFGCOLOR => {
                if x >= b'0' && x <= b'7' {
                    self.state = State::RANSIDENDVAL;
                    self.nextstyle &= 0xF8;
                    self.nextstyle |= convert_color(x - b'0');
                } else {
                    self.state = State::RANSIDESC;
                    self.nextstyle = self.style;
                    rv.ascii = x;
                }
            }
            State::RANSIDEQUALS => {
                if x == b'1' {
                    self.state = State::RANSIDENDVAL;
                    self.nextstyle &= !(1 << 3);
                } else {
                    self.state = State::RANSIDESC;
                    self.nextstyle = self.style;
                    rv.ascii = x;
                }
            }
            State::RANSIDENDVAL => {
                if x == b';' {
                    self.state = State::RANSIDPARSE;
                } else if x == b'm' {
                    self.state = State::RANSIDESC;
                    self.style = self.nextstyle;
                } else {
                    self.state = State::RANSIDESC;
                    self.nextstyle = self.style;
                }
            }
        };
        rv
    }
}

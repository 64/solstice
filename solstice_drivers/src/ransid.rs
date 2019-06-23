use crate::vga_text::Color;

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
	state: State,
	style: u8,
	next_style: u8,
}

pub struct Color_Char {
	style: u8,
	ascii: u8
}
impl Ransid_State {
	pub fn new() -> Ransid_State {
		Ransid_State {
			state: State::RANSID_ESC,
			style: 0x0F,
			next_style: 0x0F,
		}
	}
}

fn ransid_convert_color(color: u8) -> u8 {
	let mut lookup_table: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];
	lookup_table[color as usize]
}

pub fn ransid_process(state: &mut Ransid_State, x: u8) -> Color_Char {
	let mut rv = Color_Char {
		style: (*state).style,
		ascii: '\0' as u8,
	};
	match (*state).state {
		State::RANSID_ESC => {
			if x == ESC {
				(*state).state = State::RANSID_BRACKET;
			} else {
				rv.ascii = x;
			}
		},
		State::RANSID_BRACKET => {
			if x == b'[' {
				(*state).state = State::RANSID_PARSE;
			} else {
				(*state).state = State::RANSID_ESC;
				rv.ascii = x;
			}
		},
		State::RANSID_PARSE => {
			if x == b'3' {
				(*state).state = State::RANSID_FGCOLOR;
			} else if x == b'4' {
				(*state).state = State::RANSID_BGCOLOR;
			} else if x == b'0' {
				(*state).state = State::RANSID_ENDVAL;
				(*state).next_style = 0x0F;
			} else if x == b'1' {
				(*state).state = State::RANSID_ENDVAL;
				(*state).next_style |= (1 << 3);
			} else if x == b'=' {
				(*state).state = State::RANSID_EQUALS;
			} else {
				(*state).state = State::RANSID_ESC;
				(*state).next_style = (*state).style;
				rv.ascii = x;
			}
		},
		State::RANSID_BGCOLOR => {
			if x >= b'0' && x <= b'7' {
				(*state).state = State::RANSID_ENDVAL;
				(*state).next_style &= 0x1F;
				(*state).next_style |= cansid_convert_color(x - b'0') << 4;
			} else {
				(*state).state = State::RANSID_ESC;
				(*state).next_style = (*state).style;
				rv.ascii = x;
			}
		},
		State::RANSID_FGCOLOR => {
			if x >= b'0' && x <= b'7' {
				(*state).state = State::RANSID_ENDVAL;
				(*state).next_style &= 0xF8;
				(*state).next_style |= cansid_convert_color(x - b'0');
			} else {
				(*state).state = State::RANSID_ESC;
				(*state).next_style = (*state).style;
				rv.ascii = x;
			}
		},
		State::RANSID_EQUALS => {
			if x == b'1' {
				(*state).state = State::RANSID_ENDVAL;
				(*state).next_style &= !(1 << 3);
			} else {
				(*state).state = State::RANSID_ESC;
				(*state).next_style = (*state).style;
				rv.ascii = x;
			}
		},
		State::RANSID_ENDVAL => {
			if x == b';' {
				(*state).state = State::RANSID_PARSE;
			} else if x == b'm' {
				(*state).state = State::RANSID_ESC;
				(*state).style = (*state).next_style;
			} else {
				(*state).state = State::RANSID_ESC;
				(*state).next_style = (*state).style;
			}
		}
	};
	rv
}
/// Data structures and methods for creating and shuffling keyboard layouts.

extern crate rand;

use std::fmt;
use self::rand::random;

/* ----- *
 * TYPES *
 * ----- */

// KeyMap format:
//    LEFT HAND | RIGHT HAND
//     0  1  2  | 3  4  5
//     6  7  8  | 9  10 11
// 	12 13 14 15 | 16 17 18 19
//  20 21 22 23 | 24 25 26 27
//           28 | 29 (thumb keys)
//     30 31 32 | 33 34 35

pub type KeyMap<T> =  [T; NUM_OF_KEYS];


#[derive(Clone)]
pub struct Layer(KeyMap<char>);

#[derive(Clone)]
pub struct Layout(Layer, Layer);



pub struct LayoutPosMap([Option<KeyPress>; 128]);

#[derive(Clone)]
pub struct LayoutShuffleMask(KeyMap<bool>);

#[derive(Clone, Copy, PartialEq)]
pub enum Finger 
{
	Thumb,
	Index,
	Middle,
	Ring,
	Pinky,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Hand
{
	Left,
	Right,
	Thumb
}

#[derive(Clone, Copy, PartialEq)]
pub enum Row
{
	Top,
	MiddleTop,
	MiddleBottom,
	Bottom,
	Thumb,
}

#[derive(Clone, Copy)]
pub struct KeyPress
{
	pub kc:     char,
	pub pos:    usize,
	pub finger: Finger,
	pub hand:   Hand,
	pub row:    Row,
	pub center: bool,
}

/* ------- *
* STATICS *
* ------- */

const NUM_OF_KEYS: usize = 36;

// pub static _: Layout = Layout(
//     Layer(['', '', '',  '', '',     '', '', '', '', '', '',
//            '', '', '',  '', '',     '', '', '', '', '', '',
//            '', '', '',  '', '',     '', '', '', '', '',
//                             '',     '']),
//     Layer(['', '', '',  '', '',     '', '', '', '', '', '',
//            '', '', '',  '', '',     '', '', '', '', '', '',
//            '', '', '',  '', '',     '', '', '', '', '',
//                             '',     ''])
// );

// pub static RSTHD: Layout = Layout(
// 	Layer(['j', 'c', 'y', 'f', 'k',   'z', 'l', ',', 'u', 'q', '=',
// 		   'r', 's', 't', 'h', 'd',   'm', 'n', 'a', 'i', 'o',  '\'',
// 		   '/', 'v', 'g', 'p', 'b',   'x', 'w', '.', ';', '-',
//                                'e',   ' ']),
// 	Layer(['J', 'C', 'Y', 'F', 'K',   'Z', 'L', '<', 'U', 'Q', '+',
// 		   'R', 'S', 'T', 'H', 'D',   'M', 'N', 'A', 'I', 'O', '"',
// 		   '?', 'V', 'G', 'P', 'B',   'X', 'W', '>', ':', '_',
// 							   'E',   ' ']));

// pub static DABEST: Layout = Layout(
// 	Layer(['b', 'y', 'o', 'u', '/',   'f', 'g', 'd', 'l', 'V', '-',
// 		   'h', 'i', 'e', 'a', ',',   'd', 't', 's', 'n', 'r', '\'',
// 		   'q', 'x', 'z', '.', ';',   'k', 'w', 'c', 'm', 'j',
//                                '\0',  ' ']),
// 	Layer(['B', 'Y', 'O', 'U', '?',   'F', 'G', 'D', 'L', 'V', '_',
// 		   'H', 'I', 'E', 'A', '<',   'D', 'T', 'S', 'N', 'R', '"',
// 		   'Q', 'X', 'Z', '>', ':',   'K', 'W', 'C', 'M', 'J',
//                                '\0',  '\n']));

// pub static X1: Layout = Layout(
// 	Layer(['k', 'y', 'o', 'u', '/',   'f', 'c', 'l', 'p', 'v', '-',
// 		   'h', 'i', 'e', 'a', ',',   'd', 's', 't', 'n', 'r', '\'',
// 		   'q', 'x', 'z', '.', ';',   'w', 'g', 'm', 'b', 'j',
// 							   '\0',  ' ']),
// 	Layer(['K', 'Y', 'O', 'U', '?',   'F', 'C', 'L', 'P', 'V', '_',
// 		   'H', 'I', 'E', 'A', '<',   'D', 'S', 'T', 'N', 'R', '"',
// 		   'Q', 'X', 'Z', '>', ':',   'W', 'G', 'M', 'B', 'J',
// 							   '\0',  '\n']));

// q w e r t y u i o p [ { ] } \ |
// a s d f g h j k l ; : ' "
// z x c v b n m , < . > / ?
// ! @ $ % ^ & * ( ) - _ = +

//   e r t  y u i
//   d f g  h j k
// q w x c  n m o p
// a s z v  b , . l


// q w e r t y u i o p
// a s d f g h j k l
// z x c v b n m


pub static BASE: Layout = Layout(
	Layer([          'e', 'r', 't',   'y', 'u', 'i', 
	                 'd', 'f', 'g',   'h', 'j', 'k',
		        'q', 'w', 'x', 'c',   'n', 'm', 'o', 'p', 
				'a', 's', 'z', 'v',   'b', ',', '.', 'l', 
							  '\0',   '\0',
				     ' ','\0','\0',   '\0', '\0', '\n']),
	Layer([          'E', 'R', 'T',   'Y', 'U', 'I',
		             'D', 'F', 'G',   'H', 'J', 'K',
		        'Q', 'W', 'X', 'C',   'N', 'M', 'O', 'P',
		        'A', 'S', 'Z', 'V',   'B', '<', '>', 'L',
                              '\0',   '\0',
					 ' ','\0','\0',   '\0','\0','\n']));

// pub static DVORAK_LAYOUT: Layout = Layout(
// 	Layer(['\'', ',', '.', 'p', 'y',  'f', 'g', 'c', 'r', 'l', '/',
// 		   'a', 'o', 'e', 'u', 'i',   'd', 'h', 't', 'n', 's', '-',
// 		   ';', 'q', 'j', 'k', 'x',   'b', 'm', 'w', 'v', 'z',
//                                '\0',  ' ']),
// 	Layer(['"', ',', '.', 'P', 'Y',   'F', 'G', 'C', 'R', 'L', '?',
// 		   'A', 'O', 'E', 'U', 'I',   'D', 'H', 'T', 'N', 'S', '_',
// 		   ':', 'Q', 'J', 'K', 'X',   'B', 'M', 'W', 'V', 'Z',
//                                '\0',  '\n']));

// pub static COLEMAK_LAYOUT: Layout = Layout(
// 	Layer(['q', 'w', 'f', 'p', 'g',   'j', 'l', 'u', 'y', ';', '-',
// 		   'a', 'r', 's', 't', 'd',   'h', 'n', 'e', 'i', 'o', '\'',
//            'z', 'x', 'c', 'v', 'b',   'k', 'm', ',', '.', '/',
// 							   '\0',  ' ']),
// 	Layer(['Q', 'W', 'F', 'P', 'G',   'J', 'L', 'U', 'Y', ':', '_',
// 		   'A', 'R', 'S', 'T', 'D',   'H', 'N', 'E', 'I', 'O', '"',
// 		   'Z', 'X', 'C', 'V', 'B',   'K', 'M', '<', '>', '?',
// 							   '\0',  '\n']));

// pub static RSTHD: Layout = Layout(
// 	Layer(['j', 'c', 'y', 'f', 'k',   'z', 'l', ',', 'u', 'q', '=',
// 		   'r', 's', 't', 'h', 'd',   'm', 'n', 'a', 'i', 'o',  '\'',
// 		   '/', 'v', 'g', 'p', 'b',   'x', 'w', '.', ';', '-',
//                                'e',   ' ']),
// 	Layer(['J', 'C', 'Y', 'F', 'K',   'Z', 'L', '<', 'U', 'Q', '+',
// 		   'R', 'S', 'T', 'H', 'D',   'M', 'N', 'A', 'I', 'O', '"',
// 		   '?', 'V', 'G', 'P', 'B',   'X', 'W', '>', ':', '_',
//                                'E',   ' ']));

// pub static QGMLWY_LAYOUT: Layout = Layout(
// 	Layer(['q', 'g', 'm', 'l', 'w',   'y', 'f', 'u', 'b', ';', '-',
// 		   'd', 's', 't', 'n', 'r',   'i', 'a', 'e', 'o', 'h', '\'',
// 		   'z', 'x', 'c', 'v', 'j',   'k', 'p', ',', '.', '/',
//                                '\0',  ' ']),
// 	Layer(['Q', 'G', 'M', 'L', 'W',   'Y', 'F', 'U', 'B', ':', '_',
// 		   'D', 'S', 'T', 'N', 'R',   'I', 'A', 'E', 'O', 'H', '"',
// 		   'Z', 'X', 'C', 'V', 'J',   'K', 'P', '<', '>', '?',
//                                '\0',  ' ']));

// pub static WORKMAN_LAYOUT: Layout = Layout(
// 	Layer(['q', 'd', 'r', 'w', 'b',   'j', 'f', 'u', 'p', ';', '-',
// 		   'a', 's', 'h', 't', 'g',   'y', 'n', 'e', 'o', 'i', '\'',
// 		   'z', 'x', 'm', 'c', 'v',   'k', 'l', ',', '.', '/',
//                                '\0',  ' ']),
// 	Layer(['Q', 'D', 'R', 'W', 'B',   'J', 'F', 'U', 'P', ':', '_',
// 		   'A', 'S', 'H', 'T', 'G',   'Y', 'N', 'E', 'O', 'I', '"',
// 		   'Z', 'X', 'M', 'C', 'V',   'K', 'L', '<', '>', '?',
//                                '\0',  ' ']));

// pub static MALTRON_LAYOUT: Layout = Layout(
// 	Layer(['q', 'p', 'y', 'c', 'b',   'v', 'm', 'u', 'z', 'l', '=',
// 		   'a', 'n', 'i', 's', 'f',   'd', 't', 'h', 'o', 'r', '\'',
// 		   ',', '.', 'j', 'g', '/',   ';', 'w', 'k', '-', 'x',
//                                'e',   ' ']),
// 	Layer(['Q', 'P', 'Y', 'C', 'B',   'V', 'M', 'U', 'Z', 'L', '+',
// 		   'A', 'N', 'I', 'S', 'F',   'D', 'T', 'H', 'O', 'R', '"',
// 		   '<', '>', 'J', 'G', '?',   ':', 'W', 'K', '_', 'X',
//                                'E',   ' ']));

// pub static MTGAP_LAYOUT: Layout = Layout(
// 	Layer(['y', 'p', 'o', 'u', '-',    'b', 'd', 'l', 'c', 'k', 'j',
// 		   'i', 'n', 'e', 'a', ',',    'm', 'h', 't', 's', 'r', 'v',
// 		   '(', '"', '\'', '.', '_',   ')', 'f', 'w', 'g', 'x',
//                                 'z',   ' ']),
// 	Layer(['Y', 'P', 'O', 'U', ':',   'B', 'D', 'L', 'C', 'K', 'J',
//            'I', 'N', 'E', 'A', ';',   'M', 'H', 'T', 'S', 'R', 'V',
//            '&', '?', '*', '=', '<',   '>', 'F', 'W', 'G', 'X',
//                                'Z',   '\n']));

// pub static CAPEWELL_LAYOUT: Layout = Layout(
// 	Layer(['.', 'y', 'w', 'd', 'f',   'j', 'p', 'l', 'u', 'q', '/',
// 		   'a', 'e', 'r', 's', 'g',   'b', 't', 'n', 'i', 'o', '-',
// 		   'x', 'z', 'c', 'v', ';',   'k', 'w', 'h', ',', '\'',
//                                '\0',  ' ']),
// 	Layer(['>', 'Y', 'W', 'D', 'F',   'J', 'P', 'L', 'U', 'Q', '?',
// 		   'A', 'E', 'R', 'S', 'G',   'B', 'T', 'N', 'I', 'O', '_',
// 		   'X', 'Z', 'C', 'V', ':',   'K', 'W', 'H', '<', '"',
//                                '\0',  ' ']));

// pub static ARENSITO_LAYOUT: Layout = Layout(
// 	Layer(['q', 'l', ',', 'p', '\0',  '\0', 'f', 'u', 'd', 'k', '\0',
// 		   'a', 'r', 'e', 'n', 'b',   'g', 's', 'i', 't', 'o', '\0',
// 		   'z', 'w', '.', 'h', 'j',   'v', 'c', 'y', 'm', 'x',
//                                '\0',  ' ']),
// 	Layer(['Q', 'L', '<', 'P', '\0',  '\0', 'F', 'U', 'D', 'K', '\0',
// 		   'A', 'R', 'E', 'N', 'B',   'G', 'S', 'I', 'T', 'O', '\0',
// 		   'Z', 'W', '>', 'H', 'J',   'V', 'C', 'Y', 'M', 'X',
//                                '\0',  ' ']));

// pub static THE_ONE: Layout = Layout(
// 	Layer(['k', 'm', 'l', 'u', '!',   'v', 'd', 'r', '\'', 'q', '\\',
//            'a', 't', 'h', 'e', '.',   'c', 's', 'n', 'o', 'i', '_',
// 		   'z', 'p', 'f', 'j', ',',   'b', 'g', 'w', 'x', 'y',
//                                '\0',  ' ']),
// 	Layer(['K', 'M', 'L', 'U', '?',   'V', 'D', 'R', '"', 'Q', '|',
// 		   'A', 'T', 'H', 'E', '>',   'C', 'S', 'N', 'O', 'I', '-',
// 		   'Z', 'P', 'F', 'J', '<',   'B', 'G', 'W', 'X', 'Y',
//                                '\0',  '\n']));

// pub static TEST: Layout = Layout(
// 	Layer([	'b', 'y', 'o', 'u', 'j',   'f', 'g', 'd', 'l', 'v', '-',
// 			'h', 'i', 'e', 'a', '\'',  'p', 't', 's', 'n', 'r', ';',
// 			'z', 'x', 'q', ',', '.',   'k', 'c', 'w', 'm', '/',
// 								'(',   ' ']),
// 	Layer([	'B', 'Y', 'O', 'U', 'J',   'F', 'G', 'D', 'L', 'V', '_',
// 			'H', 'i', 'E', 'A', '"',   'P', 'T', 'S', 'N', 'R', ':',
// 			'Z', 'X', 'Q', '<', '>',   'K', 'C', 'W', 'M', '?',
//                                 ')',   '\n']));

/*
pub static SWAPPABLE_MAP: KeyMap<bool>= [
	false,  false,  false,  false,  true,       false,  false,  false,  false,  false,  false,
	false,  false,  false,  false,  false,       false,  false,  false,  false,  false,  false,
	true,  true,  true,  false,  false,       false,  true,  true,  false,  true,
								false, 		true
];
*/
pub static SWAPPABLE_MAP: KeyMap<bool>= [
	       true,  true,  true,    true,  true,  true,  
	       true,  true,  true,    true,  true,  true,  
	true,  true,  true,  true,    true,  true,  true,  true,  
	true,  true,  true,  true,    true,  true,  true,  true,  
	                    false,    false,
		false,  false,  false,    false,  false,  false,  
];
 

static KEY_FINGERS: KeyMap<Finger> = [
					Finger::Ring, Finger::Middle, Finger::Index, 	Finger::Index, Finger::Middle, Finger::Ring,
					Finger::Ring, Finger::Middle, Finger::Index,	Finger::Index, Finger::Middle, Finger::Ring,
	Finger::Pinky, Finger::Ring, Finger::Middle, Finger::Index,		Finger::Index, Finger::Middle, Finger::Ring, Finger::Pinky,
	Finger::Pinky, Finger::Ring, Finger::Middle, Finger::Index,		Finger::Index, Finger::Middle, Finger::Ring, Finger::Pinky,
												 Finger::Thumb, 	Finger::Thumb,
				   Finger::Thumb, Finger::Thumb, Finger::Thumb, 	Finger::Thumb, Finger::Thumb, Finger::Thumb
];

static KEY_HANDS: KeyMap<Hand> = [
				Hand::Left, Hand::Left, Hand::Left,     Hand::Right, Hand::Right, Hand::Right, 
				Hand::Left, Hand::Left, Hand::Left,    	Hand::Right, Hand::Right, Hand::Right, 
	Hand::Left, Hand::Left, Hand::Left, Hand::Left,     Hand::Right, Hand::Right, Hand::Right, Hand::Right, 
	Hand::Left, Hand::Left, Hand::Left, Hand::Left,     Hand::Right, Hand::Right, Hand::Right, Hand::Right, 
									   Hand::Thumb, 	Hand::Thumb,
			 Hand::Thumb, Hand::Thumb, Hand::Thumb, 	Hand::Thumb, Hand::Thumb, Hand::Thumb
];

static KEY_ROWS: KeyMap<Row> = [
												  Row::Top, Row::Top, Row::Top, 			Row::Top, Row::Top, Row::Top,
	  							Row::MiddleTop, Row::MiddleTop, Row::MiddleTop, 			Row::MiddleTop, Row::MiddleTop, Row::MiddleTop,
	Row::MiddleBottom, Row::MiddleBottom, Row::MiddleBottom, Row::MiddleBottom, 			Row::MiddleBottom, Row::MiddleBottom, Row::MiddleBottom, Row::MiddleBottom,  
							Row::Bottom, Row::Bottom, Row::Bottom, Row::Bottom, 			Row::Bottom, Row::Bottom, Row::Bottom, Row::Bottom, 
																	Row::Thumb, 			Row::Thumb,
											Row::Thumb, Row::Thumb, Row::Thumb, 			Row::Thumb, Row::Thumb, Row::Thumb
];

static KEY_CENTER_COLUMN: KeyMap<bool> = [
			false, false, true,    true, false, false,
			false, false, true,    true, false, false,
	 false, false, false, true,    true, false, false, false,
	 false, false, false, true,    true, false, false, false,
						 false,    false,
		   false, false, false,    false, false, false
];

pub static KP_NONE: Option<KeyPress> = None;

static LAYOUT_FILE_IDXS: KeyMap<usize> = [
		 0,  1,  2,    3,  4,  5, 
		 6,  7,  8,    9,  10, 11,
	12, 13, 14, 15,    16, 17, 18, 19, 
	20, 21, 22, 23,    24, 25, 26, 27, 
				28,    29,    
		30, 31, 32,    33, 34, 35];

/* ----- *
 * IMPLS *
 * ----- */

impl Layout
{
	pub fn from_string(s: &str)
	-> Layout
	{
		let s: Vec<char> = s.chars().collect();
		let mut lower: [char; 36] = ['\0'; 36];
		let mut upper: [char; 36] = ['\0'; 36];
		
		for i in 0..36 {
			let file_i = LAYOUT_FILE_IDXS[i];
			lower[i] = *s.get(file_i).unwrap_or(&'\0');
			upper[i] = *s.get(file_i + 40).unwrap_or(&'\0');
		}

		Layout(Layer(lower), Layer(upper))
	}

	pub fn shuffle(&mut self, times: usize)
	{
		for _ in 0..times {
			let (i, j) = Layout::shuffle_position();
			let Layout(ref mut lower, ref mut upper) = *self;
			lower.swap(i, j);
			upper.swap(i, j);
		}
	}

	pub fn get_position_map(&self) -> LayoutPosMap
	{
		let Layout(ref lower, ref upper) = *self;
		let mut map = [None; 128];
		lower.fill_position_map(&mut map);
		upper.fill_position_map(&mut map);

		LayoutPosMap(map)
	}

	fn  shuffle_position() -> (usize, usize)
	{
		let mut i = random::<usize>() % NUM_OF_KEYS;
		let mut j = random::<usize>() % NUM_OF_KEYS;

		while !SWAPPABLE_MAP[i] {
			i = random::<usize>() % NUM_OF_KEYS;
		}
		while !SWAPPABLE_MAP[j] {
			j = random::<usize>() % NUM_OF_KEYS;
		}
		(i,j)
	}
}

impl Layer
{
	fn swap(&mut self, i: usize, j: usize)
	{
		let Layer(ref mut layer) = *self;
		let temp = layer[i];
		layer[i] = layer[j];
		layer[j] = temp;
	}

	fn fill_position_map(&self, map: &mut [Option<KeyPress>; 128])
	{
		let Layer(ref layer) = *self;
		for (i, c) in layer.into_iter().enumerate() {
			if *c < (128 as char) {
				map[*c as usize] = Some(KeyPress {
					kc: *c,
					pos: i,
					finger: KEY_FINGERS[i],
					hand: KEY_HANDS[i],
					row: KEY_ROWS[i],
					center: KEY_CENTER_COLUMN[i],
				});
			}
		}
	}
}

impl LayoutPosMap
{
	pub fn get_key_position(&self, kc: char)
	-> &Option<KeyPress>
	{
		let LayoutPosMap(ref map) = *self;
		if kc < (128 as char) {
			&map[kc as usize]
		} else {
			&KP_NONE
		}
	}
}


impl fmt::Display for Layout
{
	fn fmt(&self, f: &mut fmt::Formatter)
	-> fmt::Result
	{
		let Layout(ref lower, _) = *self;
		lower.fmt(f)
	}
}

impl fmt::Display for Layer
{
	fn fmt(&self, f: &mut fmt::Formatter)
	-> fmt::Result
	{
		let Layer(ref layer) = *self;
		write!(f, "{} {} {} | {} {} {}
{} {} {} | {} {} {}
{} {} {} {} | {} {} {} {}
{} {} {} {} | {} {} {} {}
        {} | {}
{} {} {} | {} {} {}",
			layer[0], layer[1], layer[2], 		layer[3], layer[4], layer[5], 
			layer[6], layer[7], layer[8], 		layer[9], layer[10], layer[11], 
 layer[12], layer[13], layer[14], layer[15], 	layer[16], layer[17], layer[18], layer[19], 
layer[20], layer[21], layer[22], layer[23], 	layer[24], layer[25], layer[26], layer[27], 
								 layer[28], 	layer[29], 
			layer[30], layer[31],layer[32], 	layer[33],layer[34], layer[35])
	}
}

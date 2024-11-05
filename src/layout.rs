/// Data structures and methods for creating and shuffling keyboard layouts.

extern crate rand;

use std::{fmt, ops::{Index, IndexMut}};
use self::rand::random;

use serde::{Deserialize, Serialize};

extern crate serde;

use crate::layout;

use serde_big_array::big_array;

big_array! {
    BigArray;
    layout::NUM_OF_KEYS,
}

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

pub type KeyMap = [char; NUM_OF_KEYS];
pub type KeyIndexMap = [u32; NUM_OF_KEYS];
pub type MaskMap = [bool; NUM_OF_KEYS];
pub type SwapMap = [bool; NUM_OF_KEYS];
pub type FingerMap = [Finger; NUM_OF_KEYS];
pub type HandMap = [Hand; NUM_OF_KEYS];
pub type RowMap = [Row; NUM_OF_KEYS];
pub type CenterMap = [bool; NUM_OF_KEYS];
pub type PositionMap = [EmptyKeyPress; NUM_OF_KEYS];

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct LayerKeys{
	#[serde(with = "BigArray")]
    keys: KeyMap,
}

impl LayerKeys {
	pub const fn new(values: KeyMap) -> Self {
        LayerKeys {
            keys: values
        }
    }
}

impl Index<usize> for LayerKeys {
    type Output = char;

    fn index(&self, index: usize) -> &Self::Output {
        &self.keys[index]
    }
}

impl IndexMut<usize> for LayerKeys {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		&mut self.keys[index]
    }
}


#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct Layer(LayerKeys);

impl Layer {
	pub fn new(keys: LayerKeys) -> Layer {
		Layer(keys)
	}

	pub fn to_string(self) -> String {
		return String::from_iter(self.0.keys)
	}
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct Layout(pub Layer, pub Layer);

impl Layout {
	pub fn new(lower: Layer, upper: Layer) -> Layout {
		Layout(lower, upper)
	}
}

pub struct LayoutPosMap([Option<KeyPress>; 128]);
pub struct EmptyLayoutPosMap([EmptyKeyPress; NUM_OF_KEYS]);

#[derive(Clone)]
pub struct LayoutShuffleMask(MaskMap);

#[derive(Debug,Clone, Copy, PartialEq)]
pub enum Finger 
{
	Index,
	Middle,
	Ring,
	Pinky,
	Thumb,
	ThumbBottom,
}

impl Finger {
	pub fn index(&self)
	-> usize
	{
		match self {
			Finger::Index => 1,
			Finger::Middle => 2,
			Finger::Ring => 3,
			Finger::Pinky => 4,
			_ => 0
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hand
{
	Left,
	Right
}

#[derive(Debug,Clone, Copy, PartialEq, PartialOrd)]
pub enum Row
{
	Top,
	MiddleTop,
	MiddleBottom,
	Bottom,
	Thumb,
}

impl Row {
	pub fn index(&self)
	-> usize
	{
		match self {
			Row::Top => 0,
			Row::MiddleTop => 1,
			Row::MiddleBottom => 2,
			Row::Bottom => 3,
			Row::Thumb => 4
		}
	}

	pub fn difference(&self, prev_row:Row)
	-> usize
	{
		match (self, prev_row) {
			(Row::Bottom, Row::Top) => 3,
			(Row::Bottom, Row::MiddleTop) => 2,
			(Row::Bottom, Row::MiddleBottom) => 1,
			(Row::Bottom, Row::Bottom) => 0,

			(Row::MiddleBottom, Row::Top) => 2,
			(Row::MiddleBottom, Row::MiddleTop) => 1,
			(Row::MiddleBottom, Row::MiddleBottom) => 0,
			(Row::MiddleBottom, Row::Bottom) => 1,

			(Row::MiddleTop, Row::Top) => 1,
			(Row::MiddleTop, Row::MiddleTop) => 0,
			(Row::MiddleTop, Row::MiddleBottom) => 1,
			(Row::MiddleTop, Row::Bottom) => 2,

			(Row::Top, Row::Top) => 0,
			(Row::Top, Row::MiddleTop) => 1,
			(Row::Top, Row::MiddleBottom) => 2,
			(Row::Top, Row::Bottom) => 3,

			_ => 0
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub struct KeyPress
{
	pub kc:     char,
	pub pos:    usize,
	pub finger: Finger,
	pub hand:   Hand,
	pub row:    Row,
	pub center: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct EmptyKeyPress
{
	pub pos:    usize,
	pub finger: Finger,
	pub hand:   Hand,
	pub row:    Row,
	pub center: bool,
}

/* ------- *
* STATICS *
* ------- */

pub const NUM_OF_KEYS: usize = 36;

//pub const NUM_OF_KEYS: usize = 28;

// q w e r t y u i o p [ { ] } \ |
// a s d f g h j k l ; : ' "
// z x c v b n m , < . > / ?
// ! @ $ % ^ & * ( ) - _ = +

// pub const firstLayer: LayerKeys = LayerKeys::new([          'e', 'r', 't',   'y', 'u', 'i', 
// 'd', 'f', 'g',   'h', 'j', 'k',
// 'q', 'w', 'x', 'c',   'n', 'm', 'o', 'p', 
// 'a', 's', 'z', 'v',   'b', ',', '.', 'l', 
// 		 '\0',   '\0',
// ' ','\0','\0',   '\0', '\0', '\n']);

// pub const secondLayer: LayerKeys = LayerKeys::new([          'e', 'r', 't',   'y', 'u', 'i', 
// 'd', 'f', 'g',   'h', 'j', 'k',
// 'q', 'w', 'x', 'c',   'n', 'm', 'o', 'p', 
// 'a', 's', 'z', 'v',   'b', ',', '.', 'l', 
// 		 '\0',   '\0',
// ' ','\0','\0',   '\0', '\0', '\n']);

// v  z     |    q  j
// c  l  w  | u  o  f
// n  p  s  t  | a  e  h  d
// m  b     g  | x     y  k
// 		|
//    r    |   i

// [
// 	"",  "",  "",   "",  "",  "", 
//    "r", "h", "c",   "x", "l", "s", 
// "k", "t", "o", "u",   "b", "i", "n", "v", 
// "z", "d", "m", "p",   "",  "g", "y",  "", 
// 			 "j",   "", 
//    "f", "e", "q",   "", "a", "w"
// ];

#[rustfmt::skip]
pub static BASE: Layout = Layout(
	Layer(LayerKeys::new([          
					 'f', 'j', 'x',   'r', 'h', 'y', 
	                 'e', 'o', 'u',   'c', 'n', 'a',
		        'd', 't', 'g', 'p',   'v', 's', 'i', 'w', 
				'k', 'l', ' ', 'z',   'm', ' ', 'b', 'q', 
							  ' ',   ' ',
				    ' ',' ',' ',   ' ', ' ', '_'
					])),
	Layer(LayerKeys::new([          
					'F', 'J', 'X',   'R', 'H', 'Y', 
					'E', 'O', 'U',   'C', 'N', 'A',
			   'D', 'T', 'G', 'P',   'V', 'S', 'I', 'W', 
			   'K', 'L', ' ', 'Z',   'M', ' ', 'B', 'Q', 
							 ' ',   ' ',
					' ',' ',' ',   ' ', ' ', '_'
					])),
					);

			// 		'w', 'v', 'm',   'e', 'g', 'o', 
			// 		' ', ' ', 'z',   'h', 'd', 'a',
			//    'k', ' ', ' ', ' ',   'r', 'i', 'y', 'c', 
			//    'p', 'x', 'q', ' ',   't', 'n', ' ', 'u', 
			// 				 's',   'l',
			// 		'b',' ','f',   ' ', ' ', 'j'])),

					// #[rustfmt::skip]
					// static LAYOUT_FILE_IDXS: KeyIndexMap = [
					// 		 0,  1,  2,    3,  4,  5, 
					// 		 6,  7,  8,    9,  10, 11,
					// 	12, 13, 14, 15,    16, 17, 18, 19, 
					// 	20, 21, 22, 23,    24, 25, 26, 27, 
					// 				28,    29,    
					// 		30, 31, 32,    33, 34, 35];

#[rustfmt::skip]
pub static SWAPPABLE_MAP: SwapMap= [
	       true,  true,  true,   true,  true,  true,  
	       true,  true,  true,    true,  true,  true,  
	true,  true,  true,  true,    true,  true,  true,  true,  
	true,  true,  true,  true,    true,  true,  true,  true,  
	                    false,    false,
		false,  false,  false,    false,  false,  false,  
];
 
#[rustfmt::skip]
pub static KEY_FINGERS: FingerMap = [
				   Finger::Ring, Finger::Middle, Finger::Index, 	Finger::Index, Finger::Middle, Finger::Ring,
				   Finger::Ring, Finger::Middle, Finger::Index,		Finger::Index, Finger::Middle, Finger::Ring,
	Finger::Pinky, Finger::Ring, Finger::Middle, Finger::Index,		Finger::Index, Finger::Middle, Finger::Ring, Finger::Pinky,
	Finger::Pinky, Finger::Ring, Finger::Middle, Finger::Index,		Finger::Index, Finger::Middle, Finger::Ring, Finger::Pinky,
												 Finger::Thumb, 	Finger::Thumb,
	   Finger::ThumbBottom, Finger::ThumbBottom, Finger::Thumb, 	Finger::Thumb, Finger::ThumbBottom, Finger::ThumbBottom
];

#[rustfmt::skip]
pub static KEY_HANDS: HandMap = [
				Hand::Left, Hand::Left, Hand::Left,     Hand::Right, Hand::Right, Hand::Right, 
				Hand::Left, Hand::Left, Hand::Left,    	Hand::Right, Hand::Right, Hand::Right, 
	Hand::Left, Hand::Left, Hand::Left, Hand::Left,     Hand::Right, Hand::Right, Hand::Right, Hand::Right, 
	Hand::Left, Hand::Left, Hand::Left, Hand::Left,     Hand::Right, Hand::Right, Hand::Right, Hand::Right, 
									    Hand::Left, 	Hand::Right,
			 	Hand::Left, Hand::Left, Hand::Left, 	Hand::Right, Hand::Right, Hand::Right
];

#[rustfmt::skip]
pub static KEY_ROWS: RowMap = [
												  Row::Top, Row::Top, Row::Top, 			Row::Top, Row::Top, Row::Top,
	  							Row::MiddleTop, Row::MiddleTop, Row::MiddleTop, 			Row::MiddleTop, Row::MiddleTop, Row::MiddleTop,
	Row::MiddleBottom, Row::MiddleBottom, Row::MiddleBottom, Row::MiddleBottom, 			Row::MiddleBottom, Row::MiddleBottom, Row::MiddleBottom, Row::MiddleBottom,  
							Row::Bottom, Row::Bottom, Row::Bottom, Row::Bottom, 			Row::Bottom, Row::Bottom, Row::Bottom, Row::Bottom, 
																	Row::Thumb, 			Row::Thumb,
											Row::Thumb, Row::Thumb, Row::Thumb, 			Row::Thumb, Row::Thumb, Row::Thumb
];

#[rustfmt::skip]
static KEY_CENTER_COLUMN: CenterMap = [
			false, false, true,    true, false, false,
			false, false, true,    true, false, false,
	 false, false, false, true,    true, false, false, false,
	 false, false, false, true,    true, false, false, false,
						 false,    false,
		   false, false, false,    false, false, false
];

pub static KP_NONE: Option<KeyPress> = None;

#[rustfmt::skip]
static LAYOUT_FILE_IDXS: KeyIndexMap = [
		 0,  1,  2,    3,  4,  5, 
		 6,  7,  8,    9,  10, 11,
	12, 13, 14, 15,    16, 17, 18, 19, 
	20, 21, 22, 23,    24, 25, 26, 27, 
				28,    29,    
		30, 31, 32,    33, 34, 35
		];

/* ----- *
 * IMPLS *
 * ----- */

impl Layout
{
	pub fn from_string(s: &str)
	-> Layout
	{
		let s: Vec<char> = s.chars().collect();
		let mut lower: [char; layout::NUM_OF_KEYS] = ['\0'; layout::NUM_OF_KEYS];
		let mut upper: [char; layout::NUM_OF_KEYS] = ['\0'; layout::NUM_OF_KEYS];
		
		for i in 0..layout::NUM_OF_KEYS {
			let file_i = LAYOUT_FILE_IDXS[i];
			lower[i] = *s.get(file_i as usize).unwrap_or(&'\0');
			upper[i] = *s.get(file_i as usize + 40).unwrap_or(&'\0');
		}

		Layout(Layer(LayerKeys::new(lower)), Layer(LayerKeys::new(upper)))
	}

	pub fn from_lower_string(s: &str)
	-> Layout
	{
		let s: Vec<char> = s.chars().collect();
		let mut lower: [char; layout::NUM_OF_KEYS] = ['\0'; layout::NUM_OF_KEYS];
		let mut upper: [char; layout::NUM_OF_KEYS] = ['\0'; layout::NUM_OF_KEYS];
		
		for i in 0..layout::NUM_OF_KEYS {
			let file_i = LAYOUT_FILE_IDXS[i];
			let key = if let Some(entry) = s.get(file_i as usize) {
					let keychar = *entry;
					let upperkeychar = keychar.to_uppercase().collect::<Vec<_>>()[0];
					lower[i] = keychar;
					upper[i] = upperkeychar;
				};

			// if key == &'\0' {
			// 	lower[i] = '\0';
			// 	upper[i] = '\0';
			// }
			// else {
			// 	let keychar = *key;
			// 	let upperkeychar = keychar.to_uppercase().collect::<Vec<_>>()[0];
			// 	lower[i] = keychar;
			// 	upper[i] = upperkeychar;
			// }
		}

		Layout(Layer(LayerKeys::new(lower)), Layer(LayerKeys::new(upper)))
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

	pub fn get_character_positions(&self) -> KeyMap
	{
		let Layout(ref lower, ref upper) = *self;
		let Layer(ref layerKeys) = *lower;

		layerKeys.keys
	}

	pub fn shuffle_position() -> (usize, usize)
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

pub fn get_empty_position_map() -> EmptyLayoutPosMap
{
	let mut map: PositionMap = [(); NUM_OF_KEYS].map(|_| {
		EmptyKeyPress {
			pos: 0,
			finger: Finger::Index,
			hand: Hand::Left,
			row: Row::Bottom,
			center: false,
		}
	});

	for i in 0..NUM_OF_KEYS {
		map[i] = EmptyKeyPress {
			pos: i,
			finger: KEY_FINGERS[i],
			hand: KEY_HANDS[i],
			row: KEY_ROWS[i],
			center: KEY_CENTER_COLUMN[i],
		};
	}

	EmptyLayoutPosMap(map)
}

impl Layer
{
	pub fn swap(&mut self, i: usize, j: usize)
	{
		let Layer(ref mut layer) = *self;
		let temp = layer[i];
		layer[i] = layer[j];
		layer[j] = temp;
	}

	fn fill_position_map(&self, map: &mut [Option<KeyPress>; 128])
	{
		let Layer(ref layer) = *self;
		for (i, c) in layer.keys.into_iter().enumerate() {
			if (0 as char) < c && c < (128 as char) {
				map[c as usize] = Some(KeyPress {
					kc: c,
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

impl EmptyLayoutPosMap
{
	pub fn get_key_position(&self, pos: usize)
	-> &EmptyKeyPress
	{
		let EmptyLayoutPosMap(ref map) = *self;
		&map[pos]
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
		write!(f, "{}\n{}\n{}\n{}\n{}\n{}",
format!(
	"{:<2} {:<2} {:<2} {:<2} | {:<2} {:<2} {:<2} {:<2}",
	"", layer[0], layer[1], layer[2], 		layer[3], layer[4], layer[5],""
), 
format!(
	"{:<2} {:<2} {:<2} {:<2} | {:<2} {:<2} {:<2} {:<2}",
	"",layer[6], layer[7], layer[8], 		layer[9], layer[10], layer[11],""
), 
format!(
	"{:<2} {:<2} {:<2} {:<2} | {:<2} {:<2} {:<2} {:<2}",
	layer[12], layer[13], layer[14], layer[15], 	layer[16], layer[17], layer[18], layer[19], 
), 
format!(
	"{:<2} {:<2} {:<2} {:<2} | {:<2} {:<2} {:<2} {:<2}",
	layer[20], layer[21], layer[22], layer[23], 	layer[24], layer[25], layer[26], layer[27], 
), 
format!(
	"{:<2} {:<2} {:<2} {:<2} | {:<2} {:<2} {:<2} {:<2}",
	"","","",layer[28], 	layer[29], "","",""
), 
format!(
	"{:<2} {:<2} {:<2} {:<2} | {:<2} {:<2} {:<2} {:<2}",
	"",layer[30], layer[31],layer[32], 	layer[33],layer[34], layer[35],""
), 
		)
	}
}

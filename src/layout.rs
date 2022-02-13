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

#[derive(Clone, Copy, Serialize, Deserialize)]
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


#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Layer(LayerKeys);

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Layout(Layer, Layer);



pub struct LayoutPosMap([Option<KeyPress>; 128]);

#[derive(Clone)]
pub struct LayoutShuffleMask(MaskMap);

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

pub const NUM_OF_KEYS: usize = 36;

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

#[rustfmt::skip]
pub static BASE: Layout = Layout(
	Layer(LayerKeys::new([          'e', 'r', 't',   'y', 'u', 'i', 
	                 'd', 'f', 'g',   'h', 'j', 'k',
		        'q', 'w', 'x', 'c',   'n', 'm', 'o', 'p', 
				'a', 's', 'z', 'v',   'b', ',', '.', 'l', 
							  '\0',   '\0',
				     ' ','\0','\0',   '\0', '\0', '\n'])),
	Layer(LayerKeys::new([          'E', 'R', 'T',   'Y', 'U', 'I',
		             'D', 'F', 'G',   'H', 'J', 'K',
		        'Q', 'W', 'X', 'C',   'N', 'M', 'O', 'P',
		        'A', 'S', 'Z', 'V',   'B', '<', '>', 'L',
                              '\0',   '\0',
					 ' ','\0','\0',   '\0','\0','\n'])));

					 #[rustfmt::skip]
pub static SWAPPABLE_MAP: SwapMap= [
	       true,  true,  true,    true,  true,  true,  
	       true,  true,  true,    true,  true,  true,  
	true,  true,  true,  true,    true,  true,  true,  true,  
	true,  true,  true,  true,    true,  true,  true,  true,  
	                    false,    false,
		false,  false,  false,    false,  false,  false,  
];
 
#[rustfmt::skip]
static KEY_FINGERS: FingerMap = [
					Finger::Ring, Finger::Middle, Finger::Index, 	Finger::Index, Finger::Middle, Finger::Ring,
					Finger::Ring, Finger::Middle, Finger::Index,	Finger::Index, Finger::Middle, Finger::Ring,
	Finger::Pinky, Finger::Ring, Finger::Middle, Finger::Index,		Finger::Index, Finger::Middle, Finger::Ring, Finger::Pinky,
	Finger::Pinky, Finger::Ring, Finger::Middle, Finger::Index,		Finger::Index, Finger::Middle, Finger::Ring, Finger::Pinky,
												 Finger::Thumb, 	Finger::Thumb,
				   Finger::Thumb, Finger::Thumb, Finger::Thumb, 	Finger::Thumb, Finger::Thumb, Finger::Thumb
];

#[rustfmt::skip]
static KEY_HANDS: HandMap = [
				Hand::Left, Hand::Left, Hand::Left,     Hand::Right, Hand::Right, Hand::Right, 
				Hand::Left, Hand::Left, Hand::Left,    	Hand::Right, Hand::Right, Hand::Right, 
	Hand::Left, Hand::Left, Hand::Left, Hand::Left,     Hand::Right, Hand::Right, Hand::Right, Hand::Right, 
	Hand::Left, Hand::Left, Hand::Left, Hand::Left,     Hand::Right, Hand::Right, Hand::Right, Hand::Right, 
									   Hand::Thumb, 	Hand::Thumb,
			 Hand::Thumb, Hand::Thumb, Hand::Thumb, 	Hand::Thumb, Hand::Thumb, Hand::Thumb
];

#[rustfmt::skip]
static KEY_ROWS: RowMap = [
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
			lower[i] = *s.get(file_i as usize).unwrap_or(&'\0');
			upper[i] = *s.get(file_i as usize + 40).unwrap_or(&'\0');
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

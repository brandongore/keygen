/// Methods for calculating the penalty of a keyboard layout given an input
/// corpus string.

use std::vec::Vec;
use std::ops::Range;
use std::collections::HashMap;
use std::fmt;
use std;

use layout::Layout;
use layout::LayoutPosMap;
use layout::KeyMap;
use layout::KeyPress;
use layout::Finger;
use layout::Row;
use layout::KP_NONE;
use layout::Hand;

pub struct KeyPenalty<'a>
{
	name:      &'a str,
	show: 		   bool
}

#[derive(Clone)]
pub struct KeyPenaltyResult<'a>
{
	pub name:  &'a str,
	pub times:     f64,
	pub total:     f64, 
	pub show: 	   bool,
}

pub struct QuartadList<'a>{
	pub map :HashMap<&'a str, usize>
}
impl <'a> fmt::Display for KeyPenaltyResult<'a>
{
	fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}: {}", self.name, self.total)
	}
}

static BASE_PENALTY: KeyMap<f64> = [
	5.0,  0.5, 0.5, 1.5, 2.5,    2.5, 1.5, 0.5, 0.5, 5.0, 5.0,
	2.5,  0.0, 0.0, 0.0, 1.0,    1.5, 0.0, 0.0, 0.0, 1.5, 5.0,
	20.0, 2.0, 1.5, 1.0, 5.0,    5.0, 1.0, 1.5, 2.0, 20.0,
						 0.0,    0.0
];

pub fn init<'a>()-> Vec<KeyPenalty<'a>>
{
	let mut penalties = Vec::new();

	// Base penalty.
	penalties.push(KeyPenalty {
		name: "Base",
		show : true
	});

	// Penalise 5 points for using the same finger twice on different keys.
	// An extra 5 points for using the centre column.
	penalties.push(KeyPenalty {
		name: "Same finger",
		show : true
	});

	// Penalise 1 point for jumping from top to bottom row or from bottom to
	// top row on the same hand.
	penalties.push(KeyPenalty {
		name: "Long jump hand",
		show : false
	});

	// Penalise 10 points for jumping from top to bottom row or from bottom to
	// top row on the same finger.
	penalties.push(KeyPenalty {
		name: "Long jump",
		show : false
	});

	// Penalise 5 points for jumping from top to bottom row or from bottom to
	// top row on consecutive fingers, except for middle finger-top row ->
	// index finger-bottom row.
	penalties.push(KeyPenalty {
		name: "Long jump consecutive",
		show : false
	});

	// Penalise 10 points for awkward pinky/ring combination where the pinky
	// reaches above the ring finger, e.g. QA/AQ, PL/LP, ZX/XZ, ;./.; on Qwerty.
	penalties.push(KeyPenalty {
		name: "Rinky/ring twist",
		show : false
	});

	// Penalise 20 points for reversing a roll at the end of the hand, i.e.
	// using the ring, pinky, then middle finger of the same hand, or the
	// middle, pinky, then ring of the same hand.
	penalties.push(KeyPenalty {
		name: "Roll reversal",
		show : false
	});

	penalties.push(KeyPenalty {
		name: "Roll into alternation",
		show : true
	});

	penalties.push(KeyPenalty {
		name: "Alternation",
		show : true
	});

	// Penalise 0.125 points for rolling outwards.
	penalties.push(KeyPenalty {
		name: "Roll out",
		show : true
	});

	// Award 0.125 points for rolling inwards.
	penalties.push(KeyPenalty {
		name: "Roll in",
		show : true
	});

	// Penalise 3 points for jumping from top to bottom row or from bottom to
	// top row on the same finger with a keystroke in between.
	penalties.push(KeyPenalty {
		name: "long jump sandwich",
		show : false
	});

	// Penalise 10 points for three consecutive keystrokes going up or down the
	// three rows of the keyboard in a roll.
	penalties.push(KeyPenalty {
		name: "twist",
		show : false
	});
	//13
	penalties.push(KeyPenalty {
		name: "4 times no alternation",
		show : true
	});
	//14
	penalties.push(KeyPenalty {
		name: "4 alternations in a row",
		show : true
	});
	//15
	penalties.push(KeyPenalty {
		name: "right hand",
		show : true
	});
	//16
	penalties.push(KeyPenalty {
		name: "left hand",
		show : true
	});
	penalties
}

pub fn prepare_quartad_list<'a>(
	string:       &'a str,
	position_map: &'a LayoutPosMap)
-> QuartadList<'a>
{
	let mut quartads: HashMap<&str, usize> = HashMap::new();
	
	for i in 0..string.chars().count()-4{
		let slice = &string[i..i+4];
		if slice.chars().all(|c| position_map.get_key_position(c).is_some()){
			let entry = quartads.entry(slice).or_insert(0);
			*entry += 1;
		}
	}
	QuartadList{map: quartads}
}

pub fn calculate_penalty<'a>(
	quartads:  &   QuartadList<'a>,
	layout:    &   Layout,
	penalties: &'a Vec<KeyPenalty>,
	detailed:      bool)
->  (f64, Vec<KeyPenaltyResult<'a>>)
{
	let mut result: Vec<KeyPenaltyResult> = Vec::new();
	let mut total = 0.0;

	if detailed {
		for penalty in penalties {
			result.push(KeyPenaltyResult {
				name: penalty.name,
				show:penalty.show,
				total: 0.0,
				times:0.0
			});
		}
	}

	let position_map = layout.get_position_map();
	for (string, count) in &quartads.map {
		total += penalty_for_quartad(string, *count, &position_map, &mut result, detailed);
	}

	(total,  result)//total / (len as f64),
}

fn penalty_for_quartad<'a, 'b>(
	string:       &'a str,
	count:            usize,
	position_map: &'b LayoutPosMap,
	result:       &'b mut Vec<KeyPenaltyResult<'a>>,
	detailed:         bool)
-> f64
{
	let mut chars = string.chars().into_iter();
	let opt_old2 = chars.next();
	let opt_old1 = chars.next();
	let opt_curr = chars.next();
	let next = chars.next();
	
	let curr = match opt_curr {
		Some(c) => match position_map.get_key_position(c) {
			&Some(ref kp) => kp,
			&None =>  return 0.0 
		},
		None => panic!("unreachable")
	};
	let old1 = match opt_old1 {
		Some(c) => position_map.get_key_position(c),
		None => &KP_NONE
	};
	let old2 = match opt_old2 {
		Some(c) => position_map.get_key_position(c),
		None => &KP_NONE
	};
	let next = match next {
		Some(c) => position_map.get_key_position(c),
		None => &KP_NONE
	};


	
	penalize(string, count, &curr, old1, old2, next, result, detailed)
}

fn penalize<'a, 'b>(
	string: &'a     str,
	count:          usize,
	curr:   &              KeyPress,
	old1:   &       Option<KeyPress>,
	old2:   &       Option<KeyPress>,
	next:   &       Option<KeyPress>,
	result: &'b mut Vec<KeyPenaltyResult<'a>>,
	detailed:       bool)
-> f64
{
	let len = string.len();
	let count = count as f64;
	let mut total = 0.0;

	if curr.hand == Hand::Right{
		result[15].times += count;
	}
	if curr.hand == Hand::Left{
		result[16].times += count;
	}
	

	// 0: Base penalty.
	let base = BASE_PENALTY[curr.pos] * count /4.0;
	if detailed {
		result[0].times += count;
		result[0].total += base;
	}
	total += base;

	let old1 = match *old1 {
		Some(ref o) => o,
		None => { return total }
	};

	if curr.hand == old1.hand && curr.hand != Hand::Thumb{
		// 1: Same finger.
		if curr.finger == old1.finger && curr.pos != old1.pos {
			let penalty = 8.0 + if curr.center { 5.0 } else { 0.0 }
			                  + if old1.center { 5.0 } else { 0.0 };
			let penalty = penalty * 2.5;
			if detailed {
				result[1].total += penalty;
				result[1].times += count;
			}
			total += penalty * count;
		}

		// 2: Long jump hand.
		if curr.row == Row::Top && old1.row == Row::Bottom ||
		   curr.row == Row::Bottom && old1.row == Row::Top {
			let penalty = 2.0* count;
			if detailed {
				result[2].total += penalty;
				result[2].times += count;
			}
			total += penalty;
		}

		// 3: Long jump.
		if curr.finger == old1.finger {
			if curr.row == Row::Top && old1.row == Row::Bottom ||
			   curr.row == Row::Bottom && old1.row == Row::Top {
				let penalty = 10.0 * count;
				if detailed {
					result[3].total += penalty;
					result[3].times += count;
				}
				total += penalty;
			}
		}

		// 4: Long jump consecutive.
		if curr.row == Row::Top && old1.row == Row::Bottom ||
		   curr.row == Row::Bottom && old1.row == Row::Top {
			if curr.finger == Finger::Ring   && old1.finger == Finger::Pinky  ||
			   curr.finger == Finger::Pinky  && old1.finger == Finger::Ring   ||
			   curr.finger == Finger::Middle && old1.finger == Finger::Ring   ||
			   curr.finger == Finger::Ring   && old1.finger == Finger::Middle ||
			  (curr.finger == Finger::Index  && (old1.finger == Finger::Middle ||
			                                     old1.finger == Finger::Ring) &&
			   curr.row == Row::Top && old1.row == Row::Bottom) {
				let penalty = 5.0 * count;
				if detailed {
					result[4].total += penalty;
					result[4].times += count;
				}
				total += penalty;
			}
		}

		// 5: Pinky/ring twist.
		if (curr.finger == Finger::Ring && old1.finger == Finger::Pinky &&
		    (curr.row == Row::Home && old1.row == Row::Top ||
		     curr.row == Row::Bottom && old1.row == Row::Top)) ||
		   (curr.finger == Finger::Pinky && old1.finger == Finger::Ring &&
		    (curr.row == Row::Top && old1.row == Row::Home ||
		     curr.row == Row::Top && old1.row == Row::Bottom)) {
			let penalty = 10.0 * count;
			if detailed {
				result[5].total += penalty;
				result[5].times += count;
			}
			total += penalty;
		}

		// 9: Roll out.
		if is_roll_out(curr.finger, old1.finger) {
			let penalty = 1.5 * count;
			if detailed {
				result[9].total += penalty;
				//result[9].times += count;
			}
			total += penalty;
		}

		// 10: Roll in.
		if is_roll_in(curr.finger, old1.finger) {
			let penalty = -1.0 * count;
			if detailed {
				result[10].total += penalty;
				//result[10].times += count;
			}
			total += penalty;
			
			//alternation after roll
			if let Some(ref nxt) = *next {
				if curr.hand != nxt.hand && nxt.hand != Hand::Thumb {
					let penalty = -0.05 * count;
					if detailed {
						result[7].total += penalty;
						result[7].times += count;
					}
					total += penalty;
				}
			}
		}
		if is_roll_in2(curr.finger, old1.finger){
			result[10].times+=count;

		}
		if is_roll_out2(curr.finger, old1.finger){
			result[9].times+=count;
			
		}
	}
	// Three key penalties.
	let old2 = match *old2 {
		Some(ref o) => o,
		None => { return total },
	};
		
	if let Some(ref nxt) = *next {
			// 8: Alternation 
			if curr.hand != old1.hand && curr.hand != Hand::Thumb && old1.hand != Hand::Thumb{
				let penalty = -0.2 * count;
				if detailed {
					result[8].total += penalty;
					result[8].times += count;
				}
				total += penalty;
			}
			if curr.hand == old1.hand && old1.hand == old2.hand && old2.hand == nxt.hand {
				// 13: 4 no alternation
				let penalty = 0.2 * count;
				if detailed {
					result[13].total += penalty;
					result[13].times += count;
				}
				total += penalty;
			} else if curr.hand != old1.hand && old1.hand != old2.hand && curr.hand != nxt.hand 
			&& curr.hand != Hand::Thumb && old1.hand != Hand::Thumb&& old2.hand != Hand::Thumb&& nxt.hand != Hand::Thumb{
				// 14: 4 alternations in a row.
				let penalty = 0.01 * count;
				if detailed {
					result[14].total += penalty;
					result[14].times += count;
				}
				total += penalty;
			}
		}

	

	if curr.hand == old1.hand && old1.hand == old2.hand {
		// 6: Roll reversal.
		if (curr.finger == Finger::Middle && old1.finger == Finger::Pinky && old2.finger == Finger::Ring) ||
		    curr.finger == Finger::Ring && old1.finger == Finger::Pinky && old2.finger == Finger::Middle {
			let slice3 = &string[(len - 3)..len];
			let penalty = 10.0 * count;
			if detailed {
				result[6].total += penalty;
				result[6].times += count;
			}
			total += penalty;
		}

		// 12: Twist.
		if ((curr.row == Row::Top && old1.row == Row::Home && old2.row == Row::Bottom) ||
		    (curr.row == Row::Bottom && old1.row == Row::Home && old2.row == Row::Top)) &&
		   ((is_roll_out(curr.finger, old1.finger) && is_roll_out(old1.finger, old2.finger)) ||
		   	(is_roll_in(curr.finger, old1.finger) && is_roll_in(old1.finger, old2.finger))) {
			let slice3 = &string[(len - 3)..len];
			let penalty = 5.0 * count;
			if detailed {
				result[12].total += penalty;
				result[12].times += count;
			}
			total += penalty;
		}
	}

	// 11: Long jump sandwich.
	if curr.hand == old2.hand && curr.finger == old2.finger {
		if curr.row == Row::Top && old2.row == Row::Bottom ||
		   curr.row == Row::Bottom && old2.row == Row::Top {
			let penalty = 3.0 * count;
			if detailed {
				result[11].total += penalty;
				result[11].times += count;
			}
			total += penalty;
		}
	}
	
	total
}

fn is_roll_out(curr: Finger, prev: Finger) -> bool {
	match curr {
		Finger::Middle => prev == Finger::Index,
		Finger::Ring   => prev != Finger::Pinky && prev != Finger::Ring && prev != Finger::Thumb,
		Finger::Pinky  => prev != Finger::Pinky && prev != Finger::Thumb,
		_  => false
	}
}
// my restricted roll-in, as not all inward rolls feel good
fn is_roll_in(curr: Finger, prev: Finger) -> bool {
	match curr {
		Finger::Index  => prev != Finger::Thumb && prev != Finger::Index,
		Finger::Middle => prev == Finger::Pinky || prev == Finger::Ring,
		_  => false
	}
}
fn is_roll_out2(curr: Finger, prev: Finger) -> bool {
	match curr {
		Finger::Thumb  => false,
		Finger::Index  => prev == Finger::Thumb,
		Finger::Middle => prev == Finger::Thumb || prev == Finger::Index,
		Finger::Ring   => prev != Finger::Pinky && prev != Finger::Ring,
		Finger::Pinky  => prev != Finger::Pinky
	}
}
// all roll-ins
fn is_roll_in2(curr: Finger, prev: Finger) -> bool {
	match curr {
		Finger::Thumb  => prev != Finger::Thumb,
		Finger::Index  => prev != Finger::Thumb && prev != Finger::Index,
		Finger::Middle => prev == Finger::Pinky || prev == Finger::Ring,
		Finger::Ring   => prev == Finger::Pinky,
		Finger::Pinky  => false,
	}
}

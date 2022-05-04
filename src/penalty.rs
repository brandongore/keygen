use crate::{
    corpus_manager::NgramList,
    layout,
    timer::{Timer, TimerState},
};
use std;
use std::collections::HashMap;
use std::fmt;
/// Methods for calculating the penalty of a keyboard layout given an input
/// corpus string.
//use layout;
use std::vec::Vec;

use layout::*;

use quanta::Clock;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use serde_big_array::big_array;

big_array! {
    BigArray;
    layout::NUM_OF_KEYS,
}

pub type PenaltyMap = [f64; layout::NUM_OF_KEYS];

// impl fmt::Display for 
// {
// 	fn fmt(&self, f: &mut fmt::Formatter)
// 	-> fmt::Result
// 	{
// 		let penalties = *self;
// 		format!(f, "{}\n{}\n{}\n{}\n{}\n{}",
// format!(
// 	"{:<2} {:<2} {:<2} {:<2} | {:<2} {:<2} {:<2} {:<2}",
// 	"", penalties[0], penalties[1], penalties[2], 		penalties[3], penalties[4], penalties[5],""
// ), 
// format!(
// 	"{:<2} {:<2} {:<2} {:<2} | {:<2} {:<2} {:<2} {:<2}",
// 	"",penalties[6], penalties[7], penalties[8], 		penalties[9], penalties[10], penalties[11],""
// ), 
// format!(
// 	"{:<2} {:<2} {:<2} {:<2} | {:<2} {:<2} {:<2} {:<2}",
// 	penalties[12], penalties[13], penalties[14], penalties[15], 	penalties[16], penalties[17], penalties[18], penalties[19], 
// ), 
// format!(
// 	"{:<2} {:<2} {:<2} {:<2} | {:<2} {:<2} {:<2} {:<2}",
// 	penalties[20], penalties[21], penalties[22], penalties[23], 	penalties[24], penalties[25], penalties[26], penalties[27], 
// ), 
// format!(
// 	"{:<2} {:<2} {:<2} {:<2} | {:<2} {:<2} {:<2} {:<2}",
// 	"","","",penalties[28], 	penalties[29], "","",""
// ), 
// format!(
// 	"{:<2} {:<2} {:<2} {:<2} | {:<2} {:<2} {:<2} {:<2}",
// 	"",penalties[30], penalties[31],penalties[32], 	penalties[33],penalties[34], penalties[35],""
// ), 
// 		)
// 	}
// }

// Penalise 30 points for using the same finger twice on different keys.
// An extra penalty of the same amount for each usage of the center row.
const SAME_FINGER_PENALTY: Option<f64> = Some(30.0);

// Penalise 30 points for jumping from top to bottom row or from bottom to
// top row on the same finger.
const LONG_JUMP_PENALTY: Option<f64> = Some(30.0);

// Penalise 1 point for jumping from top to bottom row or from bottom to
// top row on the same hand.
const LONG_JUMP_HAND_PENALTY: Option<f64> = Some(1.0);

// Penalise 5 points for jumping from top to bottom row or from bottom to
// top row on consecutive fingers, except for middle finger-top row ->
// index finger-bottom row.
const LONG_JUMP_CONSECUTIVE_PENALTY: Option<f64> = Some(5.0);

// Penalise 10 points if the ring finger is sandwiched in between
// the pinky and middle finger but streched out farther than those
// two (for example AWD and KO; on Qwerty)
const RING_STRETCH_PENALTY: Option<f64> = Some(10.0);

// Penalise 1 point if the pinky follows the ring finger (inprecise movement)
const PINKY_RING_PENALTY: Option<f64> = Some(1.0);

// Penalise 10 points for awkward pinky/ring combination where the pinky
// reaches above the ring finger, e.g. QA/AQ, PL/LP, ZX/XZ, ;./.; on Qwerty.
const PINKY_RING_TWIST_PENALTY: Option<f64> = Some(10.0);

// Penalise 20 points for reversing a roll at the end of the hand, i.e.
// using the ring, pinky, then middle finger of the same hand, or the
// middle, pinky, then ring of the same hand.
const ROLL_REVERSAL_PENALTY: Option<f64> = Some(10.0);

// Penalise 0.5 points for using the same hand four times in a row.
const SAME_HAND_PENALTY: Option<f64> = Some(0.5);

// Penalise 0.5 points for alternating hands three times in a row.
const ALTERNATING_HAND_PENALTY: Option<f64> = Some(0.5);

// Penalise 0.125 points for rolling outwards.
const ROLL_OUT_PENALTY: Option<f64> = Some(0.125);

// Award 0.125 points for rolling inwards.
const ROLL_IN_PENALTY: Option<f64> = Some(-0.125);

// Penalise 5 points for using the same finger on different keys
// with one key in between ("detached same finger bigram").
// An extra penalty of the same amount for each usage of the center row.
const SFB_SANDWICH_PENALTY: Option<f64> = Some(5.0);

// Penalise 10 points for jumping from top to bottom row or from bottom to
// top row on the same finger with a keystroke in between.
const LONG_JUMP_SANDWICH_PENALTY: Option<f64> = Some(10.0);

// Penalise 10 points for three consecutive keystrokes going up or down
//  (currently only down)the three rows of the keyboard in a roll.
const TWIST_PENALTY: Option<f64> = Some(10.0);

#[derive(Clone, Copy)]
pub struct KeyPenaltyDescription {
    name: &'static str,
    show: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyPenalty {
    pub name: String,
    pub times: usize,
    pub total: f64,
    pub show: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Penalty {
    pub penalties: Vec<KeyPenalty>,
    #[serde(with = "BigArray")]
    pub pos: [usize; layout::NUM_OF_KEYS],
    #[serde(with = "BigArray")]
    pub pos_pen: [f64; layout::NUM_OF_KEYS],
    pub fingers: [usize; 10],
    pub hands: [usize; 2],
    pub total: f64,
    pub len: usize,
}
impl Penalty {
    pub fn new() -> Penalty {
        let mut penalties = Vec::new();
        for desc in PenaltyDescriptions.into_iter() {
            penalties.push(KeyPenalty {
                name: desc.name.to_string(),
                show: desc.show,
                total: 0.0,
                times: 0,
            });
        }
        Penalty {
            penalties: penalties,
            pos: [0; layout::NUM_OF_KEYS],
            pos_pen: [0.0; layout::NUM_OF_KEYS],
            fingers: [0; 10],
            hands: [0; 2],
            total: 0.0,
            len: 0,
        }
    }
}

impl fmt::Display for Penalty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let penalty = self;
        write!(
            f,
            "{:?} {:?} {:?}",
            penalty.fingers, penalty.hands, penalty.total
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BestLayoutsEntry {
    pub layout: Layout,
    pub penalty: Penalty,
}
impl Ord for BestLayoutsEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.penalty.total.partial_cmp(&other.penalty.total) {
            Some(ord) => ord,
            None => std::cmp::Ordering::Equal,
        }
    }
}
impl PartialEq for BestLayoutsEntry {
    fn eq(&self, other: &Self) -> bool {
        self.penalty.total == other.penalty.total
    }
}
impl Eq for BestLayoutsEntry {}
impl PartialOrd for BestLayoutsEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.penalty.total.partial_cmp(&other.penalty.total)
    }
}

impl fmt::Display for KeyPenalty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.total)
    }
}

#[rustfmt::skip]
pub static BASE_PENALTY: PenaltyMap = [
        8.0, 8.0, 10.0,     10.0, 8.0, 8.0,
         1.0, 1.0, 2.5,     2.5, 1.0, 1.0,
    6.0, 0.5, 0.5, 1.5,     1.5, 0.5, 0.5, 6.0,
    6.0, 2.0, 2.0, 2.5,     2.5, 2.0, 2.0, 6.0,
                   5.0,     5.0,
         4.0, 4.0, 5.0,     5.0, 4.0, 4.0,
];

static PenaltyDescriptions: [KeyPenaltyDescription; 16] = [
    // Base penalty.
    KeyPenaltyDescription {
        name: "Base",
        show: true,
    },
    // Penalise 5 points for using the same finger twice on different keys.
    // An extra 5 points for using the centre column.
    KeyPenaltyDescription {
        name: "Same finger",
        show: true,
    },
    // Penalise 1 point for jumping from top to bottom row or from bottom to
    // top row on the same hand.
    KeyPenaltyDescription {
        name: "Long jump hand",
        show: true,
    },
    // Penalise 10 points for jumping from top to bottom row or from bottom to
    // top row on the same finger.
    KeyPenaltyDescription {
        name: "Long jump",
        show: true,
    },
    // Penalise 5 points for jumping from top to bottom row or from bottom to
    // top row on consecutive fingers, except for middle finger-top row ->
    // index finger-bottom row.
    KeyPenaltyDescription {
        name: "Long jump consecutive",
        show: true,
    },
    // Penalise 10 points for awkward pinky/ring combination where the pinky
    // reaches above the ring finger, e.g. QA/AQ, PL/LP, ZX/XZ, ;./.; on Qwerty.
    KeyPenaltyDescription {
        name: "Rinky/ring twist",
        show: true,
    },
    // Penalise 20 points for reversing a roll at the end of the hand, i.e.
    // using the ring, pinky, then middle finger of the same hand, or the
    // middle, pinky, then ring of the same hand.
    KeyPenaltyDescription {
        name: "Roll reversal",
        show: true,
    },
    //7
    KeyPenaltyDescription {
        name: "Long roll out",
        show: true,
    },
    KeyPenaltyDescription {
        name: "Alternation",
        show: true,
    },
    // Penalise 0.125 points for rolling outwards.
    KeyPenaltyDescription {
        name: "Roll out",
        show: true,
    },
    // Award 0.125 points for rolling inwards.
    KeyPenaltyDescription {
        name: "Roll in",
        show: true,
    },
    // Penalise 3 points for jumping from top to bottom row or from bottom to
    // top row on the same finger with a keystroke in between.
    KeyPenaltyDescription {
        name: "long jump sandwich",
        show: true,
    },
    // Penalise 10 points for three consecutive keystrokes going up or down the
    // three rows of the keyboard in a roll.
    KeyPenaltyDescription {
        name: "twist",
        show: true,
    },
    //13
    KeyPenaltyDescription {
        name: "4 times no alternation",
        show: false,
    },
    //14
    KeyPenaltyDescription {
        name: "4 alternations in a row",
        show: false,
    },
    //15
    KeyPenaltyDescription {
        name: "same finger trigram",
        show: true,
    },
];

pub fn calculate_penalty<'a>(
    quartads: &Vec<(Vec<char>, usize)>,
    layout: &Layout,
) -> BestLayoutsEntry {
    let mut result = Penalty::new();

    let position_map = layout.get_position_map();

    quartads.iter().for_each(|(string, count)| {
        //let test = Clock::new();
        //let start = test.start();
        //timer.start(String::from("splitmap"));

        // let trigram1 = &string[0..3];
        //let trigram2 = &string[1..3];
        //timer.stop(String::from("splitmap"));
        //timer.start(String::from("trigrams"));

        //[string].iter().for_each(|trigram| {

        let old2 = position_map.get_key_position(string[0]).unwrap();
        let old1 = position_map.get_key_position(string[1]).unwrap();
        let curr = position_map.get_key_position(string[2]).unwrap();

        // let old2 = match *position_map.get_key_position(trigram.chars().nth(0).expect("broken ngram")) {
        //     Some(ref o) => o,
        //     _ => continue,
        // };

        // let old1 = match *position_map.get_key_position(trigram[1]) {
        //     Some(ref o) => o,
        //     _ => (),
        // };

        // let curr = match *position_map.get_key_position(trigram[2]) {
        //     Some(ref o) => o,
        //     _ => (),
        // };

        result.len += count;
        result.len += count;
        result.len += count;

        update_hand(&old2, count, &mut result);
        update_hand(&old1, count, &mut result);
        update_hand(&curr, count, &mut result);

        update_position(&old2, count, &mut result);
        update_position(&old1, count, &mut result);
        update_position(&curr, count, &mut result);

        // 0: Base penalty.
        log_base_penalty(&old2, count, &mut result);
        log_base_penalty(&old1, count, &mut result);
        log_base_penalty(&curr, count, &mut result);

        evaluate_same_hand_penalties(&old2, &old1, count, &mut result);
        evaluate_same_hand_penalties(&old1, &curr, count, &mut result);

        //8: Alternation

        evaluate_different_hand_penalties(&old2, &old1, count, &mut result);
        evaluate_different_hand_penalties(&old1, &curr, count, &mut result);

        evaluate_trigram_penalties(&old2, &old1, &curr, count, &mut result);
        //});

        //timer.stop(String::from("trigrams"));
        // let end = test.end();
        // let long = test.delta(start, end).as_nanos();
        // if long > 300 {
        // println!("{:?} {:?} {:?} {:?}", long, old2, old1, curr);
        // }
    });

    // for (string, count) in &quartads.map {
    //     //timer.start(String::from("map"));
    //     timer.start(String::from("splitmap"));

    //     let trigram1 = &string[0..2];
    //     let trigram2 = &string[1..3];

    //     // let mut trigrams: Vec<Vec<char>> = Vec::new();

    //     // if string.len() > 3 {
    //     //     for i in 0..string.chars().count() - 3 {
    //     //         let slice = &string[i..i + 3];
    //     //         if slice.chars().all(|c| (c as i32) <= 128) {
    //     //             let letters: Vec<char> = slice.chars().collect();
    //     //             trigrams.push(letters);
    //     //         }
    //     //     }
    //     // } else {
    //     //     let letters: Vec<char> = string.chars().collect();
    //     //     trigrams.push(letters);
    //     // }
    //     timer.stop(String::from("splitmap"));
    //     timer.start(String::from("trigrams"));

    //     //let test = String::from("test");
    //     // [trigram1,trigram2].iter().enumerate().for_each(|(i, v)| {

    //     // });

    //     //for trigram in [trigram1,trigram2].iter() {
    //         // let old2 = match *position_map.get_key_position(trigram[0]) {
    //         //     Some(ref o) => o,
    //         //     None => continue,
    //         // };

    //         // let old1 = match *position_map.get_key_position(trigram[1]) {
    //         //     Some(ref o) => o,
    //         //     None => continue,
    //         // };

    //         // let curr = match *position_map.get_key_position(trigram[2]) {
    //         //     Some(ref o) => o,
    //         //     None => continue,
    //         // };

    //         //result.len += count;

    //         // update_hand(old2, count, &mut result);
    //         // update_hand(old1, count, &mut result);
    //         // update_hand(curr, count, &mut result);

    //         // // 0: Base penalty.
    //         // log_penalty(0, BASE_PENALTY[curr.pos] / 5.0, count, &mut result);

    //         // evaluate_same_hand_penalties(old2, old1, count, &mut result);
    //         // evaluate_same_hand_penalties(old1, curr, count, &mut result);

    //         // //8: Alternation

    //         // evaluate_different_hand_penalties(old2, old1, count, &mut result);
    //         // evaluate_different_hand_penalties(old1, curr, count, &mut result);

    //         // evaluate_trigram_penalties(old2, old1, curr, count, &mut result);
    //     //}
    //     timer.stop(String::from("trigrams"));
    //timer.stop(String::from("map"));
    //}

    let ret = BestLayoutsEntry {
        layout: layout.clone(),
        penalty: result,
    };

    return ret;
}

fn run_penalty_calculation(
    string: &String,
    count: &usize,
    result: Penalty,
    position_map: LayoutPosMap,
    timer: &mut HashMap<String, TimerState>,
) -> Penalty {
    let trigram1 = &string[0..2];
    let trigram2 = &string[1..3];
    [trigram1, trigram2]
        .iter()
        .enumerate()
        .for_each(|(i, v)| {});
    result
}

fn use_finger(kp: &KeyPress, count: &usize, result: &mut Penalty, i: usize) {
    match kp.finger {
        Finger::Pinky => result.fingers[i] += count,
        Finger::Ring => result.fingers[i + 1] += count,
        Finger::Middle => result.fingers[i + 2] += count,
        Finger::Index => result.fingers[i + 3] += count,
        Finger::Thumb => result.fingers[i + 4] += count,
        Finger::ThumbBottom => result.fingers[i + 4] += count,
    };
}

pub fn update_hand(kp: &KeyPress, count: &usize, result: &mut Penalty) {
    match kp.hand {
        Hand::Left => {
            result.hands[0] += count;
            use_finger(kp, count, result, 0);
        }
        Hand::Right => {
            result.hands[1] += count;
            use_finger(kp, count, result, 5);
        }
    }
}

pub fn update_position_penalty(kp: &KeyPress, penalty: f64, result: &mut Penalty) {
    result.pos_pen[kp.pos] += penalty;
}

pub fn update_position(kp: &KeyPress, count: &usize, result: &mut Penalty) {
    result.pos[kp.pos] += count;
}

pub fn log_base_penalty(
    curr: &KeyPress,
    count: &usize,
    result: &mut Penalty,
) {
    let penalty = BASE_PENALTY[curr.pos] / 5.0;
    log_penalty(0, penalty, count, result);
    update_position_penalty(curr, penalty, result);
}

pub fn log_same_finger_penalty(
    prev: &KeyPress,
    curr: &KeyPress,
    count: &usize,
    result: &mut Penalty,
) {
    //sfb
    let penalty = match (
        curr.finger.index() == prev.finger.index(),
        curr.pos != prev.pos,
    ) {
        (true, true) => {
            15.0
        }
        _ => 0.0,
        //let penalty = 15.0; //+ if curr.center { 5.0 } else { 0.0 } ;
        // log(1, penalty);
    };
    log_penalty(1, penalty, count, result);
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
}

pub fn log_long_jump_hand(prev: &KeyPress, curr: &KeyPress, count: &usize, result: &mut Penalty) {
    let penalty = match (prev.row, curr.row) {
        (Row::Bottom, Row::Top) => {
            5.0
        }
        (Row::Top, Row::Bottom) => {
            5.0
        }
        _ => 0.0,
    };

    log_penalty(2, penalty, count, result);
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
}

pub fn log_long_jump(prev: &KeyPress, curr: &KeyPress, count: &usize, result: &mut Penalty) {
    let penalty = match (prev.row, curr.row, curr.finger == prev.finger) {
        (Row::Bottom, Row::Top, true) => {
            20.0
        }
        (Row::Top, Row::Bottom, true) => {
            20.0
        }
        _ => 0.0,
    };
    log_penalty(3, penalty, count, result);
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
}

pub fn evaluate_trigram_penalties(
    first: &KeyPress,
    second: &KeyPress,
    third: &KeyPress,
    count: &usize,
    result: &mut Penalty,
) {
    if first.hand == second.hand && second.hand == third.hand {
        // 6: Roll reversal.
        let penalty_reversal = match (
            third.finger.index() < first.finger.index(),
            first.finger.index() < second.finger.index(),
            third.finger.index() < second.finger.index(),
            third.finger == Finger::Pinky,
            third.finger == Finger::Thumb || third.finger == Finger::ThumbBottom,
            first.finger == third.finger,
        ) {
            //rollback out in
            (true, true, true, false, false, false) => {
                12.0
            }
            //rollback in out wide pinky
            (false, false, false, true, false, false) => {
                8.0
            }
            //rollback in out wide thumb
            (false, false, false, false, true, false) => {
                8.0
            }
            //rollback in out wide ring
            (false, false, false, false, false, false) => {
                8.0
            }
            //roll in out thin
            (true, false, false, false, false, false) => {
                8.0
            }

            //rollback to same
            //ring or index rollback smaller
            (false, false, false, false, false, true) => {
                4.0
            }
            //ring or index rollback bigger
            (false, true, true, false, false, true) => {
                4.0
            }
            //pinky rollback smaller
            (false, false, false, true, false, true) => {
                4.0
            }
            //pinky rollback bigger
            (false, true, true, true, false, true) => {
                6.0
            }
            //thumb rollback
            (false, false, false, false, true, true) => {
                4.0
            }
            _ => 0.0,
        };

        log_penalty(6, penalty_reversal, count, result);

        // 12: Twist.
        let penalty_twist = match (
            first.row.index() > second.row.index(),
            second.row.index() > third.row.index(),
            first.row.index() != second.row.index(),
            second.row.index() != third.row.index(),
            first.finger.index() != second.finger.index(),
            second.finger.index() != third.finger.index(),
            first.finger.index() != third.finger.index(),
            second.row.difference(first.row) > 1 || third.row.difference(second.row) > 1,
        ) {
            (true, true, true, true, true, true, true, false) => {
                6.0
            }
            (false, false, true, true, true, true, true, false) => {
                6.0
            }
            (true, true, true, true, true, true, true, true) => {
                12.0
            }
            (false, false, true, true, true, true, true, true) => {
                12.0
            }
            _ => 0.0,
        };
        log_penalty(12, penalty_twist, count, result);

        //15 same finger trigram
        let penalty_trigram = match (
            first.finger == second.finger,
            second.finger == third.finger,
            first.pos != second.pos,
            second.pos != third.pos,
            third.pos != first.pos,
        ) {
            (true, true, true, true, true) => {
                10.0
            }
            (true, true, true, true, false) => {
                10.0
            }
            (true, true, true, false, true) => {
                10.0
            }
            (true, true, false, true, true) => {
                10.0
            }
            _ => 0.0,
        };

        log_penalty(15, penalty_trigram, count, result);
        update_position_penalty(first, penalty_reversal + penalty_twist + penalty_trigram, result);
        update_position_penalty(second, penalty_reversal + penalty_twist + penalty_trigram, result);
        update_position_penalty(third, penalty_reversal + penalty_twist + penalty_trigram, result);
    }

    // 11: Long jump sandwich. dsfb
    let penalty_sandwich = match (
        third.hand == first.hand,
        third.finger == first.finger,
        third.row.index() > first.row.index(),
        third.row.index() < first.row.index(),
        third.row.difference(first.row) > 1,
    ) {
        (true, true, true, false, true) => {
            3.0
        }
        (true, true, false, true, true) => {
            3.0
        }
        _ => 0.0,
    };
    log_penalty(11, penalty_sandwich, count, result);

    update_position_penalty(first, penalty_sandwich, result);
    update_position_penalty(second, penalty_sandwich, result);
    update_position_penalty(third, penalty_sandwich, result);
}

pub fn evaluate_different_hand_penalties(
    prev: &KeyPress,
    curr: &KeyPress,
    count: &usize,
    result: &mut Penalty,
) {
    if prev.hand != curr.hand {
        //8: Alternation
        let penalty = -0.4;
        log_penalty(8, penalty, count, result);
        update_position_penalty(prev, penalty, result);
        update_position_penalty(curr, penalty, result);
    }
}

pub fn evaluate_same_hand_penalties(
    prev: &KeyPress,
    curr: &KeyPress,
    count: &usize,
    result: &mut Penalty,
) {
    match prev.hand == curr.hand {
        true => {
            log_same_finger_penalty(prev, curr, count, result);

            log_long_jump_hand(prev, curr, count, result);

            // 3: Long jump.

            log_long_jump(prev, curr, count, result);

            // 4: Long jump consecutive.
            log_long_jump_consecutive(prev, curr, count, result);

            // 5: Pinky/ring twist.
            log_pinky_ring_twist(prev, curr, count, result);

            // 9: Roll out.
            // 7: Long Roll out.
            log_roll_out(prev, curr, count, result);

            // 10: Roll in.
            log_roll_in(prev, curr, count, result);
        }

        _ => (),
    }

    // if prev.hand == curr.hand {
    //     // 1: Same finger.
    //     log_same_finger_penalty(prev, curr, count, result);

    //     // log_long_jump_hand(prev, curr, count, result);

    //     // // 3: Long jump.

    //     // log_long_jump(prev, curr, count, result);

    //     // // 4: Long jump consecutive.
    //     // log_long_jump_consecutive(prev, curr, count, result);

    //     // // 5: Pinky/ring twist.
    //     // log_pinky_ring_twist(prev, curr, count, result);

    //     // // 9: Roll out.
    //     // // 7: Long Roll out.
    //     // log_roll_out(prev, curr, count, result);

    //     // // 10: Roll in.
    //     // log_roll_in(prev, curr, count, result);
    // }
}

pub fn log_penalty(i: usize, penalty: f64, count: &usize, result: &mut Penalty) {
    if penalty.abs() > 0.0 {
    let p = penalty * *count as f64;
    //println!("{}; {}", i, penalty);
    result.penalties[i].times += count;
    result.penalties[i].total += p;
    result.total += p;
    }
}

pub fn log_long_jump_consecutive(
    prev: &KeyPress,
    curr: &KeyPress,
    count: &usize,
    result: &mut Penalty,
) {
    let penalty = match (prev.finger != curr.finger, curr.row.difference(prev.row)) {
        (true, 2) => {
            5.0
        }
        (true, 3) => {
            8.0
        }

        _ => 0.0,
    };
    log_penalty(4, penalty, count, result);
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
}

pub fn log_pinky_ring_twist(prev: &KeyPress, curr: &KeyPress, count: &usize, result: &mut Penalty) {
    let penalty = match (prev.finger, curr.finger, prev.row, curr.row) {
        //pinky twist into ring
        (Finger::Pinky, Finger::Ring, Row::MiddleBottom, Row::MiddleBottom) => {
            10.0
        }
        (Finger::Pinky, Finger::Ring, Row::Bottom, Row::Bottom) => {
            10.0
        }
        (Finger::Pinky, Finger::Ring, Row::MiddleBottom, Row::Bottom) => {
            10.0
        }

        //ring twist into pinky bottom
        (Finger::Ring, Finger::Pinky, Row::Top, Row::Bottom) => {
            10.0
        }
        (Finger::Ring, Finger::Pinky, Row::MiddleTop, Row::Bottom) => {
            10.0
        }
        (Finger::Ring, Finger::Pinky, Row::MiddleBottom, Row::Bottom) => {
            10.0
        }

        //ring twist into pinky middle bottom
        (Finger::Ring, Finger::Pinky, Row::Top, Row::MiddleBottom) => {
            10.0
        }
        (Finger::Ring, Finger::Pinky, Row::MiddleTop, Row::MiddleBottom) => {
            10.0
        }

        _ => 0.0,
    };
    log_penalty(5, penalty, count, result);
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
}

pub fn log_roll_out(prev: &KeyPress, curr: &KeyPress, count: &usize, result: &mut Penalty) {
    let penalty = match (
        prev.finger.index() < curr.finger.index(),
        curr.row.difference(prev.row),
    ) {
        (true, 0) => {
            1.0
        }
        (true, 1) => {
            2.0
        }
        (true, 2) => {
            12.0
        }
        (true, 3) => {
            16.0
        }

        _ => 0.0,
    };
    log_penalty(9, penalty, count, result);
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
}

pub fn log_roll_in(prev: &KeyPress, curr: &KeyPress, count: &usize, result: &mut Penalty) {
    let penalty = match (
        prev.finger.index() < curr.finger.index(),
        curr.row.difference(prev.row),
    ) {
        (true, 0) => {
            -4.0
        }
        (true, 1) => {
            -2.0
        }
        (true, 2) => {
            1.0
        }
        (true, 3) => {
            4.0
        }

        _ => 0.0,
    };
    log_penalty(10, penalty, count, result);
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
}

pub fn is_roll_out(prev: Finger, curr: Finger) -> bool {
    match curr {
        Finger::Middle => prev == Finger::Index,
        Finger::Ring => prev != Finger::Pinky && prev != Finger::Ring && prev != Finger::Thumb,
        Finger::Pinky => prev != Finger::Pinky && prev != Finger::Thumb,
        _ => false,
    }
}
// my restricted roll-in, as not all inward rolls feel good
pub fn is_roll_in(prev: Finger, curr: Finger) -> bool {
    match curr {
        Finger::Index => prev != Finger::Thumb && prev != Finger::Index,
        Finger::Middle => prev == Finger::Pinky || prev == Finger::Ring,
        _ => false,
    }
}
pub fn is_roll_out2(prev: Finger, curr: Finger) -> bool {
    match curr {
        Finger::Thumb => false,
        Finger::ThumbBottom => false,
        Finger::Index => prev == Finger::Thumb,
        Finger::Middle => prev == Finger::Thumb || prev == Finger::Index,
        Finger::Ring => prev != Finger::Pinky && prev != Finger::Ring,
        Finger::Pinky => prev != Finger::Pinky,
    }
}
// all roll-ins
pub fn is_roll_in2(prev: Finger, curr: Finger) -> bool {
    match curr {
        Finger::Thumb => prev != Finger::Thumb,
        Finger::ThumbBottom => prev != Finger::ThumbBottom,
        Finger::Index => prev != Finger::Thumb && prev != Finger::Index,
        Finger::Middle => prev == Finger::Pinky || prev == Finger::Ring,
        Finger::Ring => prev == Finger::Pinky,
        Finger::Pinky => false,
    }
}

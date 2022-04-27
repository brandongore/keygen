use crate::{corpus_manager::NgramList, layout};
use std;
use std::collections::HashMap;
use std::fmt;
/// Methods for calculating the penalty of a keyboard layout given an input
/// corpus string.
//use layout;
use std::vec::Vec;

use layout::*;

use serde::{Deserialize, Serialize};

pub type PenaltyMap = [f64; layout::NUM_OF_KEYS];

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
            fingers: [0; 10],
            hands: [0; 2],
            total: 0.0,
            len: 0,
        }
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
        show: false,
    },
    // Penalise 10 points for jumping from top to bottom row or from bottom to
    // top row on the same finger.
    KeyPenaltyDescription {
        name: "Long jump",
        show: false,
    },
    // Penalise 5 points for jumping from top to bottom row or from bottom to
    // top row on consecutive fingers, except for middle finger-top row ->
    // index finger-bottom row.
    KeyPenaltyDescription {
        name: "Long jump consecutive",
        show: false,
    },
    // Penalise 10 points for awkward pinky/ring combination where the pinky
    // reaches above the ring finger, e.g. QA/AQ, PL/LP, ZX/XZ, ;./.; on Qwerty.
    KeyPenaltyDescription {
        name: "Rinky/ring twist",
        show: false,
    },
    // Penalise 20 points for reversing a roll at the end of the hand, i.e.
    // using the ring, pinky, then middle finger of the same hand, or the
    // middle, pinky, then ring of the same hand.
    KeyPenaltyDescription {
        name: "Roll reversal",
        show: false,
    },
    //7
    KeyPenaltyDescription {
        name: "Long roll out",
        show: false,
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
        show: false,
    },
    // Penalise 10 points for three consecutive keystrokes going up or down the
    // three rows of the keyboard in a roll.
    KeyPenaltyDescription {
        name: "twist",
        show: false,
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
        show: false,
    },
];

pub fn calculate_penalty<'a>(quartads: &NgramList, layout: &Layout) -> BestLayoutsEntry {
    let mut result = Penalty::new();
    let position_map = layout.get_position_map();

    for (string, count) in &quartads.map {
        let mut chars = string.chars().into_iter();

        let mut trigrams: Vec<Vec<char>> = Vec::new();

        if string.len() > 3 {
            for i in 0..string.chars().count() - 3 {
                let slice = &string[i..i + 3];
                if slice.chars().all(|c| (c as i32) <= 128) {
                    let letters: Vec<char> = slice.chars().collect();
                    trigrams.push(letters);
                }
            }
        } else {
            let letters: Vec<char> = string.chars().collect();
            trigrams.push(letters);
        }

        for trigram in trigrams {
            let old2 = match *position_map.get_key_position(trigram[0]) {
                Some(ref o) => o,
                None => continue,
            };

            let old1 = match *position_map.get_key_position(trigram[1]) {
                Some(ref o) => o,
                None => continue,
            };

            let curr = match *position_map.get_key_position(trigram[2]) {
                Some(ref o) => o,
                None => continue,
            };

            result.len += count;

            update_hand(old2, count, &mut result);
            update_hand(old1, count, &mut result);
            update_hand(curr, count, &mut result);

            // 0: Base penalty.
            log_penalty(0, BASE_PENALTY[curr.pos] / 5.0, count, &mut result);

            evaluate_same_hand_penalties(old2, old1, count, &mut result);
            evaluate_same_hand_penalties(old1, curr, count, &mut result);

            //8: Alternation

            evaluate_different_hand_penalties(old2, old1, count, &mut result);
            evaluate_different_hand_penalties(old1, curr, count, &mut result);

            evaluate_trigram_penalties(old2, old1, curr, count, &mut result);
        }
    }
    BestLayoutsEntry {
        layout: layout.clone(),
        penalty: result,
    }
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

fn update_hand(kp: &KeyPress, count: &usize, result: &mut Penalty) {
    match kp.hand {
        Hand::Left => {
            result.hands[0] += count;
            use_finger(kp, count, result, 0);
        }
        Hand::Right => {
            result.hands[1] += count;
            use_finger(kp, count, result, 5);
        }
        _ => {}
    }
}

fn log_same_finger_penalty(prev: &KeyPress, curr: &KeyPress, count: &usize, result: &mut Penalty) {
    //sfb
    if curr.finger == prev.finger && curr.pos != prev.pos {
        let penalty = 15.0; //+ if curr.center { 5.0 } else { 0.0 } ;
                            // log(1, penalty);
        log_penalty(1, penalty, count, result);
    }
}

fn log_long_jump_hand(prev: &KeyPress, curr: &KeyPress, count: &usize, result: &mut Penalty) {
    match (prev.row, curr.row) {
        (Row::Bottom, Row::Top) => {
            log_penalty(2, 5.0, count, result);
        }
        (Row::Top, Row::Bottom) => {
            log_penalty(2, 5.0, count, result);
        }
        _ => (),
    }
}

fn log_long_jump(prev: &KeyPress, curr: &KeyPress, count: &usize, result: &mut Penalty) {
    match (prev.row, curr.row, curr.finger == prev.finger) {
        (Row::Bottom, Row::Top, true) => {
            log_penalty(3, 20.0, count, result);
        }
        (Row::Top, Row::Bottom, true) => {
            log_penalty(3, 20.0, count, result);
        }
        _ => (),
    }
}

fn evaluate_trigram_penalties(
    first: &KeyPress,
    second: &KeyPress,
    third: &KeyPress,
    count: &usize,
    result: &mut Penalty,
) {
    if first.hand == second.hand && second.hand == third.hand  {
        // 6: Roll reversal.
        match (
            third.finger.index() < first.finger.index(),
            first.finger.index() < second.finger.index(),
            third.finger.index() < second.finger.index(),
            third.finger == Finger::Pinky,
            third.finger == Finger::Thumb || third.finger == Finger::ThumbBottom,
            first.finger == third.finger
        ) {
            //rollback out in
            (true, true, true, false, false, false) => { log_penalty(6, 12.0, count, result); },
            //rollback in out wide pinky
            (false, false, false, true, false, false) => { log_penalty(6, 8.0, count, result); },
            //rollback in out wide thumb
            (false, false, false, false, true, false) => { log_penalty(6, 8.0, count, result); },
            //rollback in out wide ring
            (false, false, false, false, false, false) => { log_penalty(6, 8.0, count, result); },
            //roll in out thin
            (true, false, false, false, false, false) => { log_penalty(6, 8.0, count, result); },

            //rollback to same
            //ring or index rollback smaller
            (false, false, false, false, false, true) => { log_penalty(6, 4.0, count, result); },
            //ring or index rollback bigger
            (false, true, true, false, false, true) => { log_penalty(6, 4.0, count, result); },
            //pinky rollback smaller
            (false, false, false, true, false, true) => { log_penalty(6, 4.0, count, result); },
            //pinky rollback bigger
            (false, true, true, true, false, true) => { log_penalty(6, 6.0, count, result); },
            //thumb rollback
            (false, false, false, false, true, true) => { log_penalty(6, 4.0, count, result); },
            _ => (),
        }

        // 12: Twist.
        match (
            first.row.index() > second.row.index(),
            second.row.index() > third.row.index(),
            first.row.index() != second.row.index(),
            second.row.index() != third.row.index(),
            first.finger.index() != second.finger.index(),
            second.finger.index() != third.finger.index(),
            first.finger.index() != third.finger.index(),
            second.row.difference(first.row) > 1 || third.row.difference(second.row) > 1
        ) {
            (true, true, true, true, true, true, true, false) => { log_penalty(12, 6.0, count, result); },
            (false, false, true, true, true, true, true, false) => { log_penalty(12, 6.0, count, result); },
            (true, true, true, true, true, true, true, true) => { log_penalty(12, 12.0, count, result); },
            (false, false, true, true, true, true, true, true) => { log_penalty(12, 12.0, count, result); },
            _ => ()
        }

        //15 same finger trigram
        match (
            first.finger == second.finger,
            second.finger == third.finger,
            first.pos != second.pos,
            second.pos != third.pos,
            third.pos != first.pos
        ) {
            (true, true, true, true, true) => { log_penalty(15, 10.0, count, result); },
            (true, true, true, true, false) => { log_penalty(15, 10.0, count, result); },
            (true, true, true, false, true) => { log_penalty(15, 10.0, count, result); },
            (true, true, false, true, true) => { log_penalty(15, 10.0, count, result); },
            _ => ()
        }
    }

    // 11: Long jump sandwich. dsfb
    match (
        third.hand == first.hand,
        third.finger == first.finger,
        third.row.index() > first.row.index(),
        third.row.index() < first.row.index(),
        third.row.difference(first.row) > 1
    ) {
        (true, true, true, false, true) => { log_penalty(11, 3.0, count, result); },
        (true, true, false, true, true) => { log_penalty(11, 3.0, count, result); },
        _ => ()
    }
}

fn evaluate_different_hand_penalties(
    prev: &KeyPress,
    curr: &KeyPress,
    count: &usize,
    result: &mut Penalty,
) {
    if prev.hand != curr.hand {
        //8: Alternation
        log_penalty(8, -0.4, count, result);
    }
}

fn evaluate_same_hand_penalties(
    prev: &KeyPress,
    curr: &KeyPress,
    count: &usize,
    result: &mut Penalty,
) {
    if prev.hand == curr.hand {
        // 1: Same finger.
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
}

fn log_penalty(i: usize, penalty: f64, count: &usize, result: &mut Penalty) {
    let p = penalty * *count as f64;
    //println!("{}; {}", i, penalty);
    result.penalties[i].times += count;
    result.penalties[i].total += p;
    result.total += p;
}

fn log_long_jump_consecutive(
    prev: &KeyPress,
    curr: &KeyPress,
    count: &usize,
    result: &mut Penalty,
) {
    match (prev.finger != curr.finger, curr.row.difference(prev.row)) {
        (true, 2) => {
            log_penalty(4, 5.0, count, result);
        }
        (true, 3) => {
            log_penalty(4, 8.0, count, result);
        }

        _ => (),
    }
}

fn log_pinky_ring_twist(prev: &KeyPress, curr: &KeyPress, count: &usize, result: &mut Penalty) {
    match (prev.finger, curr.finger, prev.row, curr.row) {
        //pinky twist into ring
        (Finger::Pinky, Finger::Ring, Row::MiddleBottom, Row::MiddleBottom) => {
            log_penalty(5, 10.0, count, result);
        }
        (Finger::Pinky, Finger::Ring, Row::Bottom, Row::Bottom) => {
            log_penalty(5, 10.0, count, result);
        }
        (Finger::Pinky, Finger::Ring, Row::MiddleBottom, Row::Bottom) => {
            log_penalty(5, 10.0, count, result);
        }

        //ring twist into pinky bottom
        (Finger::Ring, Finger::Pinky, Row::Top, Row::Bottom) => {
            log_penalty(5, 10.0, count, result);
        }
        (Finger::Ring, Finger::Pinky, Row::MiddleTop, Row::Bottom) => {
            log_penalty(5, 10.0, count, result);
        }
        (Finger::Ring, Finger::Pinky, Row::MiddleBottom, Row::Bottom) => {
            log_penalty(5, 10.0, count, result);
        }

        //ring twist into pinky middle bottom
        (Finger::Ring, Finger::Pinky, Row::Top, Row::MiddleBottom) => {
            log_penalty(5, 10.0, count, result);
        }
        (Finger::Ring, Finger::Pinky, Row::MiddleTop, Row::MiddleBottom) => {
            log_penalty(5, 10.0, count, result);
        }

        _ => (),
    }
}

fn log_roll_out(prev: &KeyPress, curr: &KeyPress, count: &usize, result: &mut Penalty) {
    match (
        prev.finger.index() < curr.finger.index(),
        curr.row.difference(prev.row),
    ) {
        (true, 0) => {
            log_penalty(9, 1.0, count, result);
        }
        (true, 1) => {
            log_penalty(9, 2.0, count, result);
        }
        (true, 2) => {
            log_penalty(7, 12.0, count, result);
        }
        (true, 3) => {
            log_penalty(7, 16.0, count, result);
        }

        _ => (),
    }
}

fn log_roll_in(prev: &KeyPress, curr: &KeyPress, count: &usize, result: &mut Penalty) {
    match (
        prev.finger.index() < curr.finger.index(),
        curr.row.difference(prev.row),
    ) {
        (true, 0) => {
            log_penalty(10, -4.0, count, result);
        }
        (true, 1) => {
            log_penalty(10, -2.0, count, result);
        }
        (true, 2) => {
            log_penalty(10, 1.0, count, result);
        }
        (true, 3) => {
            log_penalty(10, 4.0, count, result);
        }

        _ => (),
    }
}

fn is_roll_out(prev: Finger, curr: Finger) -> bool {
    match curr {
        Finger::Middle => prev == Finger::Index,
        Finger::Ring => prev != Finger::Pinky && prev != Finger::Ring && prev != Finger::Thumb,
        Finger::Pinky => prev != Finger::Pinky && prev != Finger::Thumb,
        _ => false,
    }
}
// my restricted roll-in, as not all inward rolls feel good
fn is_roll_in(prev: Finger, curr: Finger) -> bool {
    match curr {
        Finger::Index => prev != Finger::Thumb && prev != Finger::Index,
        Finger::Middle => prev == Finger::Pinky || prev == Finger::Ring,
        _ => false,
    }
}
fn is_roll_out2(prev: Finger, curr: Finger) -> bool {
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
fn is_roll_in2(prev: Finger, curr: Finger) -> bool {
    match curr {
        Finger::Thumb => prev != Finger::Thumb,
        Finger::ThumbBottom => prev != Finger::ThumbBottom,
        Finger::Index => prev != Finger::Thumb && prev != Finger::Index,
        Finger::Middle => prev == Finger::Pinky || prev == Finger::Ring,
        Finger::Ring => prev == Finger::Pinky,
        Finger::Pinky => false,
    }
}

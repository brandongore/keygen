use crate::{ corpus_manager::NgramList, layout, timer::{ Timer, TimerState } };
use flurry::*;
use layout::*;
use lazy_static::lazy_static;
use std::{ cmp::Ordering, any::{ Any } };
use std::fmt;
/// Methods for calculating the penalty of a keyboard layout given an input
/// corpus string.
//use layout;
use std::vec::Vec;
use std::{ self, sync::Arc };

use quanta::Clock;
use rayon::iter::{ IntoParallelIterator, IntoParallelRefIterator, ParallelIterator };
use serde::{ Deserialize, Serialize };

use serde_big_array::big_array;

big_array! {
    BigArray;
    layout::NUM_OF_KEYS,
}

pub type PenaltyMap = [f64; layout::NUM_OF_KEYS];

pub type RelationMap = [f64; layout::NUM_OF_KEYS - 1];

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct PosRelation<const N: usize> {
    #[serde(with = "serde_arrays")]
    pub relation_map: [f64; N],
    #[serde(with = "serde_arrays")]
    pub penalty_types: [PenaltyType<N>; N],
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DisplayPosRelation {
    pub relation_map: Vec<f64>,
    pub penalty_types: Vec<Vec<usize>>,
}


#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct PenaltyType<const N: usize> {
    #[serde(with = "serde_arrays")]
    pub type_map: [usize; N],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct KeyFrequencyPenalty {
    pub key: u8,
    pub penalty: f64
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct DirectionalKeyPenalty<const N: usize> {
    #[serde(with = "serde_arrays")]
    pub key_penalty_map: [KeyFrequencyPenalty; N],
}

impl DirectionalKeyPenalty<{ layout::NUM_OF_KEYS }> {
    pub fn new() -> DirectionalKeyPenalty<{ layout::NUM_OF_KEYS }> {
        DirectionalKeyPenalty {
            key_penalty_map: [
                KeyFrequencyPenalty { key: 0, penalty: 0.0};
                layout::NUM_OF_KEYS
            ],
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct PosKeyPenalty<const N: usize> {
    #[serde(with = "serde_arrays")]
    pub pos_key_penalty: [DirectionalKeyPenalty<N>; N],
}

impl PosKeyPenalty<{ layout::NUM_OF_KEYS }> {
    pub fn new() -> PosKeyPenalty<{ layout::NUM_OF_KEYS }> {
        PosKeyPenalty {
            pos_key_penalty: [
                DirectionalKeyPenalty::new();
                layout::NUM_OF_KEYS
            ],
        }
    }
}

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyPenalty {
    pub name: String,
    pub times: usize,
    pub total: f64,
    pub show: bool,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Penalty<const N: usize> {
    pub penalties: [KeyPenalty; 21],
    #[serde(with = "serde_arrays")]
    pub pos: [usize; N],
    #[serde(with = "serde_arrays")]
    pub pos_pen: [f64; N],
    pub fingers: [usize; 10],
    pub hands: [usize; 2],
    pub bad_score_total: f64,
    pub good_score_total: f64,
    pub total: f64,
    pub len: usize,
    pub tri_pos: [usize; 3],
    #[serde(with = "serde_arrays")]
    pub pos_relation: [PosRelation<N>; N],
}

// impl Default for PenaltyMap {
//     fn default() -> Self {
//         [0.0;layout::NUM_OF_KEYS]
//     }
// }

// impl<T: Copy + Default> Default for [T; layout::NUM_OF_KEYS] {
//     #[inline]
//     fn default() -> [T; layout::NUM_OF_KEYS] {
//        [Default::default(); layout::NUM_OF_KEYS]
//     }
// }

// fn default_0() -> [PenaltyMap; layout::NUM_OF_KEYS] {
//     [0.0;layout::NUM_OF_KEYS]
//     //arr![MyStruct::new({i += 1; i - 1}); 33]
// }

// fn default_0() -> [PenaltyMap; layout::NUM_OF_KEYS] {
//     [[0.0;layout::NUM_OF_KEYS];layout::NUM_OF_KEYS]
// }

lazy_static! {
    pub static ref PENALTY_LIST: [KeyPenalty; 21] = {
        PenaltyDescriptions.into_iter()
            .map(|desc| KeyPenalty {
                name: desc.name.to_string(),
                show: desc.show,
                total: 0.0,
                times: 0,
            })
            .collect::<Vec<KeyPenalty>>()
            .try_into()
            .unwrap()
    };
}

impl Penalty<{ layout::NUM_OF_KEYS }> {
    pub fn new() -> Penalty<{ layout::NUM_OF_KEYS }> {
        // let mut penalties = Vec::new();
        // for desc in PenaltyDescriptions.into_iter() {
        //     penalties.push(KeyPenalty {
        //         name: desc.name.to_string(),
        //         show: desc.show,
        //         total: 0.0,
        //         times: 0,
        //     });
        // }
        Penalty {
            penalties: PENALTY_LIST.clone(),
            pos: [0; layout::NUM_OF_KEYS],
            pos_pen: [0.0; layout::NUM_OF_KEYS],
            fingers: [0; 10],
            hands: [0; 2],
            bad_score_total: 0.0,
            good_score_total: 0.0,
            total: 0.0,
            len: 0,
            tri_pos: [0; 3],
            pos_relation: [
                PosRelation {
                    relation_map: [0.0; layout::NUM_OF_KEYS],
                    penalty_types: [
                        PenaltyType { type_map: [usize::MAX; layout::NUM_OF_KEYS] };
                        layout::NUM_OF_KEYS
                    ],
                };
                layout::NUM_OF_KEYS
            ],
        }
    }
}

pub fn better_than_average_including_bad(
    first: &Penalty<{ layout::NUM_OF_KEYS }>,
    second: &Penalty<{ layout::NUM_OF_KEYS }>
) -> bool {
    first.total < second.total &&
        first.bad_score_total < second.bad_score_total &&
        better_than_other(first, second)
}

pub fn better_than_other(
    first: &Penalty<{ layout::NUM_OF_KEYS }>,
    second: &Penalty<{ layout::NUM_OF_KEYS }>
) -> bool {
        first.penalties[1].total < second.penalties[1].total &&
        first.penalties[2].total < second.penalties[2].total &&
        first.penalties[3].total < second.penalties[3].total &&
        first.penalties[4].total < second.penalties[4].total &&
        first.penalties[5].total < second.penalties[5].total &&
        first.penalties[6].total < second.penalties[6].total &&
        first.penalties[7].total < second.penalties[7].total &&
        first.penalties[9].total < second.penalties[9].total &&
        first.penalties[11].total < second.penalties[11].total &&
        first.penalties[12].total < second.penalties[12].total &&
        first.penalties[15].total < second.penalties[15].total &&
        first.penalties[16].total < second.penalties[16].total
}

pub fn better_than_average(
    first: &Penalty<{ layout::NUM_OF_KEYS }>,
    second: &Penalty<{ layout::NUM_OF_KEYS }>
) -> bool {
    first.penalties
        .to_vec()
        .into_iter()
        .zip(second.penalties.to_vec().into_iter())
        .map(|(first_penalty, second_penalty)| first_penalty.total <= second_penalty.total)
        .filter(|item| *item)
        .count() > (((first.penalties.len() as f64) * 0.4) as usize) &&
        first.penalties[1].total <= second.penalties[1].total
}

pub fn secondary_compare(
    first: &Penalty<{ layout::NUM_OF_KEYS }>,
    second: &Penalty<{ layout::NUM_OF_KEYS }>
) -> bool {
    match less_than_or_equal(first.bad_score_total, second.bad_score_total, 0.05) {
        true => true,
        false => {
            match less_than_or_equal(first.penalties[1].total, second.penalties[1].total, 0.05) {
                true => true,
                false => {
                    less_than_or_equal(first.penalties[9].total, second.penalties[9].total, 0.05)
                }
            }
        }
    }
}

pub fn less_than_or_equal(first: f64, second: f64, percent: f64) -> bool {
    match accepted_percent_difference(first, second, percent) {
        Ordering::Less => true,
        Ordering::Equal => true,
        _ => false,
    }
}

pub fn accepted_percent_difference(first: f64, second: f64, percent: f64) -> Ordering {
    match (first - second) / ((first + second) / 2.0) {
        score if score > percent => Ordering::Greater,
        score if score < percent => Ordering::Less,
        _ => Ordering::Equal,
    }
}

pub fn above_average(
    first: &Penalty<{ layout::NUM_OF_KEYS }>,
    second: &Penalty<{ layout::NUM_OF_KEYS }>
) -> Ordering {
    match better_than_average(first, second) {
        true => Ordering::Less,
        false => Ordering::Greater,
    }
}

pub fn difference_ordering(first: f64, second: f64) -> Option<Ordering> {
    Some(accepted_percent_difference(first, second, 0.05))
}

pub fn above_average_ordering(
    first: &Penalty<{ layout::NUM_OF_KEYS }>,
    second: &Penalty<{ layout::NUM_OF_KEYS }>
) -> Option<Ordering> {
    Some(above_average(first, second))
}

impl fmt::Display for Penalty<{ layout::NUM_OF_KEYS }> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let penalty = self;
        write!(f, "{:?} {:?} {:?}", penalty.fingers, penalty.hands, penalty.bad_score_total)
    }
}

/// chain two orderings: the first one gets more priority
fn chain_partial_ordering(o1: Option<Ordering>, o2: Option<Ordering>) -> Option<Ordering> {
    match o1 {
        Some(Ordering::Equal) => o2,
        _ => o1,
    }
}

fn chain_ordering(o1: Option<Ordering>, o2: Option<Ordering>) -> Ordering {
    match o1 {
        Some(ord) =>
            match ord {
                Ordering::Equal =>
                    match o2 {
                        Some(ord) => ord,
                        None => Ordering::Equal,
                    }
                _ => ord,
            }
        None => Ordering::Equal,
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BestLayoutsEntry {
    pub layout: Layout,
    pub penalty: Penalty<{ layout::NUM_OF_KEYS }>,
}
// impl Ord for BestLayoutsEntry {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         chain_ordering(
//             difference_ordering(self.penalty.bad_score_total, other.penalty.bad_score_total),
//             Some(chain_ordering(
//                 difference_ordering(
//                     self.penalty.penalties[1].total,
//                     other.penalty.penalties[1].total,
//                 ),
//                 Some(chain_ordering(
//                     difference_ordering(
//                         self.penalty.penalties[9].total,
//                         other.penalty.penalties[9].total,
//                     ),
//                     self.penalty
//                         .good_score_total
//                         .partial_cmp(&other.penalty.good_score_total),
//                 )),
//             )),
//         )
//     }
// }
impl Ord for BestLayoutsEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.penalty.total.partial_cmp(&other.penalty.total).unwrap()
        // chain_ordering(
        //     self.penalty
        //     .total
        //     .partial_cmp(&other.penalty.total),
        //     above_average_ordering(&self.penalty, &other.penalty),

        //     // chain_partial_ordering(
        //     //     difference_ordering(self.penalty.bad_score_total, other.penalty.bad_score_total),
        //     //     self.penalty
        //     //         .bad_score_total
        //     //         .partial_cmp(&other.penalty.bad_score_total),
        //     // ),
        // )
    }
}
impl PartialEq for BestLayoutsEntry {
    fn eq(&self, other: &Self) -> bool {
        self.penalty.bad_score_total == other.penalty.bad_score_total &&
            self.penalty.good_score_total == other.penalty.good_score_total
    }
}
impl Eq for BestLayoutsEntry {}
// impl PartialOrd for BestLayoutsEntry {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         chain_partial_ordering(
//             difference_ordering(self.penalty.bad_score_total, other.penalty.bad_score_total),
//             chain_partial_ordering(
//                 difference_ordering(
//                     self.penalty.penalties[1].total,
//                     other.penalty.penalties[1].total,
//                 ),
//                 chain_partial_ordering(
//                     difference_ordering(
//                         self.penalty.penalties[9].total,
//                         other.penalty.penalties[9].total,
//                     ),
//                     self.penalty
//                         .good_score_total
//                         .partial_cmp(&other.penalty.good_score_total),
//                 ),
//             ),
//         )
//     }
// }
impl PartialOrd for BestLayoutsEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.penalty.total.partial_cmp(&other.penalty.total)
        // chain_partial_ordering(
        //     self.penalty
        //     .total
        //     .partial_cmp(&other.penalty.total),
        //     above_average_ordering(&self.penalty, &other.penalty),

        //     // chain_partial_ordering(
        //     //     difference_ordering(self.penalty.bad_score_total, other.penalty.bad_score_total),
        //     //     self.penalty
        //     //         .bad_score_total
        //     //         .partial_cmp(&other.penalty.bad_score_total),
        //     // ),
        // )
    }
}

impl fmt::Display for KeyPenalty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.total)
    }
}

#[rustfmt::skip]
pub static BASE_PENALTY: PenaltyMap = [
        4.0, 4.25, 5.0,     5.0, 4.25, 4.0,
        0.5, 0.6, 1.25,     3.5, 0.6, 0.5,
   3.0, 0.3, 0.3, 1.0 ,     3.0, 0.3, 0.3, 3.0,
   3.0, 1.0, 1.0, 1.25,     3.5, 1.0, 1.0, 3.0,
                   5.0,     5.0,
         4.0, 0.2, 5.0,     5.0, 0.25, 4.0,
];

static PenaltyDescriptions: [KeyPenaltyDescription; 21] = [
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
    //16
    KeyPenaltyDescription {
        name: "roll in bad",
        show: true,
    },
    //17
    KeyPenaltyDescription {
        name: "unbalanced fingers",
        show: true,
    },
    //18
    KeyPenaltyDescription {
        name: "unbalance hand",
        show: true,
    },
    //19
    KeyPenaltyDescription {
        name: "right hand reduction",
        show: true,
    },
    //19
    KeyPenaltyDescription {
        name: "bad hand swap",
        show: true,
    },
];

pub fn calculate_position_penalty<'a>(
    old2: EmptyKeyPress,
    old1: EmptyKeyPress,
    curr1: EmptyKeyPress
) -> Penalty<{ layout::NUM_OF_KEYS }> {
    let mut result = Penalty::new();

    let count: &usize = &1;
    result.len += count;
    result.len += count;
    result.len += count;

    result.tri_pos[0] = old2.pos;
    result.tri_pos[1] = old1.pos;
    result.tri_pos[2] = curr1.pos;

    update_hand(&old2, count, &mut result);
    update_hand(&old1, count, &mut result);
    update_hand(&curr1, count, &mut result);
    // 0: Base penalty.
    log_base_penalty(&old2, count, &mut result);
    log_base_penalty(&old1, count, &mut result);
    log_base_penalty(&curr1, count, &mut result);

    log_base_relation_penalty(&old2, &old1, count, &mut result);
    log_base_relation_penalty(&old1, &curr1, count, &mut result);

    evaluate_same_hand_penalties(&old2, &old1, count, &mut result);
    evaluate_same_hand_penalties(&old1, &curr1, count, &mut result);
    //8: Alternation
    evaluate_different_hand_penalties(&old2, &old1, count, &mut result);
    evaluate_different_hand_penalties(&old1, &curr1, count, &mut result);
    evaluate_trigram_penalties(&old2, &old1, &curr1, count, &mut result);
    //result.penalties[0].times

    evaluate_unbalanced_finger_penalty(&mut result);
    evaluate_unbalanced_hand_penalty(&mut result);
    right_hand_reduction_penalty(&mut result);

    // let mut type_count = 0;
    // let mut all_empty: Vec<bool> = Vec::new();

    // result.pos_relation.into_iter().for_each(|relation|{
    //     relation.penalty_types.into_iter().for_each(|penalty_type|{
    //             all_empty.push(penalty_type.type_map.to_vec().iter().all(|item|{*item == -1}));
    //     });
    // });
    // println!("empty: {:?}", all_empty);
    // if all_empty.into_iter().all(|item| item == true){
        
    // }

    result
}

fn run_penalty_calculation(
    string: &String,
    count: &usize,
    result: Penalty<{ layout::NUM_OF_KEYS }>,
    position_map: LayoutPosMap,
    timer: &mut std::collections::HashMap<String, TimerState>
) -> Penalty<{ layout::NUM_OF_KEYS }> {
    let trigram1 = &string[0..2];
    let trigram2 = &string[1..3];
    [trigram1, trigram2]
        .iter()
        .enumerate()
        .for_each(|(i, v)| {});
    result
}

fn use_finger(
    kp: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>,
    i: usize
) {
    match kp.finger {
        Finger::Pinky => {
            result.fingers[i] += count;
        }
        Finger::Ring => {
            result.fingers[i + 1] += count;
        }
        Finger::Middle => {
            result.fingers[i + 2] += count;
        }
        Finger::Index => {
            result.fingers[i + 3] += count;
        }
        Finger::Thumb => {
            result.fingers[i + 4] += count;
        }
        Finger::ThumbBottom => {
            result.fingers[i + 4] += count;
        }
    }
}

pub fn update_hand(
    kp: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    match kp.hand {
        Hand::Left => {
            result.hands[0] += count;
            use_finger(kp, count, result, 0);
            update_position(kp, count, result);
        }
        Hand::Right => {
            result.hands[1] += count;
            use_finger(kp, count, result, 5);
            update_position(kp, count, result);
        }
    }
}

pub fn update_position_penalty(
    kp: &EmptyKeyPress,
    penalty: f64,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    result.pos_pen[kp.pos] += penalty;
}

pub fn update_relation_penalty(
    penalty_index: usize,
    prev: &EmptyKeyPress,
    curr: &EmptyKeyPress,
    penalty: f64,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    result.pos_relation[prev.pos].relation_map[curr.pos] += penalty;

    let mut found = false;
    for index in 0..layout::NUM_OF_KEYS{
        if !found && result.pos_relation[prev.pos].penalty_types[curr.pos].type_map[index] == usize::MAX {
            found = true;
            result.pos_relation[prev.pos].penalty_types[curr.pos].type_map[index] = penalty_index;
        }
    }
    
}

pub fn update_position(
    kp: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    result.pos[kp.pos] += count;
}

pub fn log_base_penalty(
    curr: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    let penalty = BASE_PENALTY[curr.pos] / 5.0;
    log_penalty(0, penalty, count, result);
    update_position_penalty(curr, penalty, result);
}

pub fn log_base_relation_penalty(
    prev: &EmptyKeyPress,
    curr: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    update_relation_penalty(0, prev, curr, BASE_PENALTY[prev.pos] + BASE_PENALTY[curr.pos], result);
}

pub fn log_same_finger_penalty(
    prev: &EmptyKeyPress,
    curr: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    //sfb
    let penalty = match (curr.finger.index() == prev.finger.index(), curr.pos != prev.pos) {
        (true, true) => 10.0,
        _ => 0.0,
        //let penalty = 15.0; //+ if curr.center { 5.0 } else { 0.0 } ;
        // log(1, penalty);
    };

    if penalty > 0.0 {
        log_penalty(1, penalty, count, result);
        update_relation_penalty(1,prev, curr, penalty, result);
    }
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
    
}

pub fn log_long_jump_hand(
    prev: &EmptyKeyPress,
    curr: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    let penalty = match (prev.row, curr.row) {
        (Row::Bottom, Row::Top) => 2.5,
        (Row::Top, Row::Bottom) => 2.5,
        _ => 0.0,
    };

    if penalty > 0.0 {
        log_penalty(2, penalty, count, result);
        update_relation_penalty(2,prev, curr, penalty, result);
    }
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
    
}

pub fn log_long_jump(
    prev: &EmptyKeyPress,
    curr: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    let penalty = match (prev.row, curr.row, curr.finger == prev.finger) {
        (Row::Bottom, Row::Top, true) => 10.0,
        (Row::Top, Row::Bottom, true) => 10.0,
        _ => 0.0,
    };
    if penalty > 0.0 {
        log_penalty(3, penalty, count, result);
        update_relation_penalty(3,prev, curr, penalty, result);
    }
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
    
}

pub fn evaluate_trigram_penalties(
    first: &EmptyKeyPress,
    second: &EmptyKeyPress,
    third: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    if first.hand == second.hand && second.hand == third.hand {
        // 6: Roll reversal.
        let penalty_reversal = match
            (
                third.finger.index() < first.finger.index(),
                first.finger.index() < second.finger.index(),
                third.finger.index() < second.finger.index(),
                third.finger == Finger::Pinky,
                third.finger == Finger::Thumb || third.finger == Finger::ThumbBottom,
                first.finger == third.finger,
            )
        {
            //rollback out in
            (true, true, true, false, false, false) => 6.0,
            //rollback in out wide pinky
            (false, false, false, true, false, false) => 4.0,
            //rollback in out wide thumb
            (true, true, true, false, true, false) => 4.0,
            //rollback in out wide ring
            (false, false, false, false, false, false) => 7.0,
            //roll in out thin
            (true, false, false, false, false, false) => 4.0,

            //rollback to same
            //ring or index rollback smaller
            (false, false, false, false, false, true) => 2.0,
            //ring or index rollback bigger
            (false, true, true, false, false, true) => 2.0,
            //pinky rollback smaller
            (false, false, false, true, false, true) => 6.0,
            //invalid case //pinky rollback bigger
            // (false, true, true, true, false, true) => {
            //     12.0
            // }
            //thumb rollback
            (false, true, true, false, true, true) => 2.0,
            _ => 0.0,
        };

        if penalty_reversal > 0.0 {
            log_penalty(6, penalty_reversal, count, result);
            update_relation_penalty(
                6,
                first,
                second,
                penalty_reversal,
                result
            );
            update_relation_penalty(
                6,
                second,
                third,
                penalty_reversal,
                result
            );
        }

        // 12: Twist.
        let penalty_twist = match
            (
                first.row.index() > second.row.index(),
                second.row.index() > third.row.index(),
                first.row.index() != second.row.index(),
                second.row.index() != third.row.index(),
                first.finger.index() != second.finger.index(),
                second.finger.index() != third.finger.index(),
                first.finger.index() != third.finger.index(),
                second.row.difference(first.row) > 1 || third.row.difference(second.row) > 1,
            )
        {
            (true, true, true, true, true, true, true, false) => 4.0,
            (false, false, true, true, true, true, true, false) => 4.0,
            (true, true, true, true, true, true, true, true) => 7.0,
            (false, false, true, true, true, true, true, true) => 7.0,
            _ => 0.0,
        };
        if penalty_twist > 0.0 {
            log_penalty(12, penalty_twist, count, result);
            update_relation_penalty(
                12,
                first,
                second,
                penalty_twist,
                result
            );
            update_relation_penalty(
                12,
                second,
                third,
                penalty_twist,
                result
            );
        }

        //15 same finger trigram
        let penalty_trigram = match
            (
                first.finger == second.finger,
                second.finger == third.finger,
                first.pos != second.pos,
                second.pos != third.pos,
                third.pos != first.pos,
            )
        {
            (true, true, true, true, true) => 5.0,
            (true, true, true, true, false) => 5.0,
            (true, true, true, false, true) => 5.0,
            (true, true, false, true, true) => 5.0,
            _ => 0.0,
        };

        if penalty_trigram > 0.0 {
            log_penalty(15, penalty_trigram, count, result);
            update_relation_penalty(
                15,
                first,
                second,
                penalty_trigram,
                result
            );
            update_relation_penalty(
                15,
                second,
                third,
                penalty_trigram,
                result
            );
        }
        update_position_penalty(first, penalty_reversal + penalty_twist + penalty_trigram, result);
        update_position_penalty(second, penalty_reversal + penalty_twist + penalty_trigram, result);
        update_position_penalty(third, penalty_reversal + penalty_twist + penalty_trigram, result);
    }

    // 11: Long jump sandwich. dsfb
    let penalty_sandwich = match
        (
            third.hand == first.hand,
            third.finger == first.finger,
            third.row.index() > first.row.index(),
            third.row.index() < first.row.index(),
            third.row.difference(first.row) > 1,
        )
    {
        (true, true, true, false, true) => 3.0,
        (true, true, false, true, true) => 3.0,
        _ => 0.0,
    };
    if penalty_sandwich > 0.0 {
        log_penalty(11, penalty_sandwich, count, result);
        update_relation_penalty(
            11,
            first,
            second,
            penalty_sandwich,
            result
        );
        update_relation_penalty(
            11,
            second,
            third,
            penalty_sandwich,
            result
        );
        update_position_penalty(first, penalty_sandwich, result);
        update_position_penalty(second, penalty_sandwich, result);
        update_position_penalty(third, penalty_sandwich, result);
    }

        // 20: Bad hand swap
        let penalty_bad_hand_swap = match
        (
            first.hand == second.hand,
            second.hand == third.hand,
            third.hand == first.hand
        )
    {
        (false, true, false) => {
            update_relation_penalty(
                20,
                first,
                second,
                5.0,
                result
            );
            5.0
        },
        (false, false, false) => {
            update_relation_penalty(
                20,
                first,
                second,
                9.0,
                result
            );
            update_relation_penalty(
                20,
                second,
                third,
                9.0,
                result
            );
            9.0
        },
        _ => 0.0,
    };
    if penalty_bad_hand_swap > 0.0 {
        log_penalty(20, penalty_bad_hand_swap, count, result);
        update_position_penalty(first, penalty_bad_hand_swap, result);
        update_position_penalty(second, penalty_bad_hand_swap, result);
        update_position_penalty(third, penalty_bad_hand_swap, result);
    }

}

pub fn evaluate_different_hand_penalties(
    prev: &EmptyKeyPress,
    curr: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    if prev.hand != curr.hand {
        //8: Alternation
        let penalty = -0.1;
        log_penalty(8, penalty, count, result);
        update_position_penalty(prev, penalty, result);
        update_position_penalty(curr, penalty, result);
        update_relation_penalty(8,prev, curr, penalty, result);
    }
}

pub fn evaluate_same_hand_penalties(
    prev: &EmptyKeyPress,
    curr: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
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

pub fn right_hand_reduction_penalty(result: &mut Penalty<{ layout::NUM_OF_KEYS }>) {
    let base_factor = 0.8 / 16.0;
    let penalty_right_hand = base_factor * result.bad_score_total;
    if (result.hands[0] as f64) * 100.0 < (result.hands[1] as f64) * 100.0 {
        if penalty_right_hand > 0.0 {
            log_penalty(19, penalty_right_hand, &1, result);
        }
    }
}

pub fn evaluate_unbalanced_hand_penalty(result: &mut Penalty<{ layout::NUM_OF_KEYS }>) {
    let mut unbalanced_penalty = 0.0;
    let mut sum = 0.0;
    for index in 0..2 {
        sum += ((result.hands[index] as f64) * 100.0) / (result.len as f64);
    }
    let mean = sum / (2.0 as f64);
    let base_factor = ((1.0 / (2.0 as f64)) * 0.3) / 16.0;
    for index in 0..2 {
        unbalanced_penalty +=
            (((result.hands[index] as f64) * 100.0) / (result.len as f64) - mean).abs() * //.powf(3.0)
            base_factor *
            result.bad_score_total;
    }

    if unbalanced_penalty > 0.0 {
        log_penalty(18, unbalanced_penalty, &1, result);
    }
}

pub fn evaluate_unbalanced_finger_penalty(result: &mut Penalty<{ layout::NUM_OF_KEYS }>) {
    let mut unbalanced_penalty = 0.0;
    let finger_count = result.fingers.len() / 2;
    let mut sum = 0.0;
    for index in 0..finger_count {
        sum += ((result.fingers[index] as f64) * 100.0) / (result.len as f64);
    }
    let mean = sum / (finger_count as f64);
    let base_factor = ((1.0 / (finger_count as f64)) * 0.6) / 16.0;
    for index in 0..finger_count {
        unbalanced_penalty +=
            (((result.fingers[index] as f64) * 100.0) / (result.len as f64) - mean).abs() * //.powf(3.0)
            base_factor *
            result.bad_score_total;
    }

    if unbalanced_penalty > 0.0 {
        log_penalty(17, unbalanced_penalty, &1, result);
    }
}

pub fn log_penalty(
    i: usize,
    penalty: f64,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    let p = penalty * (*count as f64);

    match penalty {
        d if d < 0.0 => {
            //println!("{}; {}", i, penalty);
            result.penalties[i].times += count;
            result.penalties[i].total += p;
            result.good_score_total += p;
        }
        d if d > 0.0 => {
            //println!("{}; {}", i, penalty);
            result.penalties[i].times += count;
            result.penalties[i].total += p;
            result.bad_score_total += p;
        }
        _ => (),
    }
    result.total += p;
    // if penalty.abs() > 0.0 {
    // let p = penalty * *count as f64;
    // //println!("{}; {}", i, penalty);
    // result.penalties[i].times += count;
    // result.penalties[i].total += p;
    // result.bad_score_total += p;
    // }
}

pub fn log_long_jump_consecutive(
    prev: &EmptyKeyPress,
    curr: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    let penalty = match (prev.finger != curr.finger, curr.row.difference(prev.row)) {
        (true, 2) => 3.0,
        (true, 3) => 4.5,
        _ => 0.0,
    };
    if penalty > 0.0 {
        log_penalty(4, penalty, count, result);
        update_relation_penalty(4,prev, curr, penalty, result);
    }
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
    
}

pub fn log_pinky_ring_twist(
    prev: &EmptyKeyPress,
    curr: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    let penalty = match (prev.finger, curr.finger, prev.row, curr.row) {
        //pinky twist into ring
        (Finger::Pinky, Finger::Ring, Row::MiddleBottom, Row::MiddleBottom) => 5.0,
        (Finger::Pinky, Finger::Ring, Row::Bottom, Row::Bottom) => 5.0,
        (Finger::Pinky, Finger::Ring, Row::MiddleBottom, Row::Bottom) => 5.0,

        //ring twist into pinky bottom
        (Finger::Ring, Finger::Pinky, Row::Top, Row::Bottom) => 5.0,
        (Finger::Ring, Finger::Pinky, Row::MiddleTop, Row::Bottom) => 5.0,
        (Finger::Ring, Finger::Pinky, Row::MiddleBottom, Row::Bottom) => 5.0,

        //ring twist into pinky middle bottom
        (Finger::Ring, Finger::Pinky, Row::Top, Row::MiddleBottom) => 5.0,
        (Finger::Ring, Finger::Pinky, Row::MiddleTop, Row::MiddleBottom) => 5.0,

        _ => 0.0,
    };
    if penalty > 0.0 {
        log_penalty(5, penalty, count, result);
        update_relation_penalty(5,prev, curr, penalty, result);
    }
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
    
}

pub fn log_roll_out(
    prev: &EmptyKeyPress,
    curr: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    let penalty = match (prev.finger.index() < curr.finger.index(), curr.row.difference(prev.row)) {
        (true, 0) => {
            log_penalty(9, 4.0, count, result);
            update_relation_penalty(9,prev, curr, 4.0, result);
            4.0
        }
        (true, 1) => {
            log_penalty(9, 6.0, count, result);
            update_relation_penalty(9,prev, curr, 6.0, result);
            6.0
        }
        (true, 2) => {
            log_penalty(7, 7.0, count, result);
            update_relation_penalty(7,prev, curr, 7.0, result);
            7.0
        }
        (true, 3) => {
            log_penalty(7, 9.0, count, result);
            update_relation_penalty(7,prev, curr, 9.0, result);
            9.0
        }

        _ => {
            //log_penalty(9, 0.0, count, result);
            0.0
        }
    };
    //log_penalty(9, penalty, count, result);
    //TODO: need to update to log 7 when long rollout
    update_position_penalty(prev, penalty, result);
    update_position_penalty(curr, penalty, result);
}

pub fn log_roll_in(
    prev: &EmptyKeyPress,
    curr: &EmptyKeyPress,
    count: &usize,
    result: &mut Penalty<{ layout::NUM_OF_KEYS }>
) {
    let penalty = match (prev.finger.index() > curr.finger.index(), curr.row.difference(prev.row)) {
        (true, 0) => {
            log_penalty(10, -6.5, count, result);
            update_relation_penalty(10,prev, curr, -6.5, result);
            -6.5
        }
        (true, 1) => {
            log_penalty(10, -3.0, count, result);
            update_relation_penalty(10,prev, curr, -3.0, result);
            -3.0
        }
        (true, 2) => {
            log_penalty(16, 3.0, count, result);
            update_relation_penalty(16,prev, curr, 3.0, result);
            3.0
        }
        (true, 3) => {
            log_penalty(16, 8.0, count, result);
            update_relation_penalty(16,prev, curr, 8.0, result);
            8.0
        }

        _ => {
            //log_penalty(10, 0.0, count, result);
            0.0
        }
    };
    //log_penalty(10, penalty, count, result);
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
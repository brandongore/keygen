use crate::{corpus_manager::NgramList, file_manager::*, layout, penalty::BASE_PENALTY};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash, ops::Index};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct KeyPositionPenalty {
    key: u8,
    position: u32,
    penalty: f64,
    frequency: f64
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct PositionPenalty {
    position: u32,
    penalty: f64,
    frequency: f64
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct PenaltyFrequency {
    penalty: f64,
    frequency: f64
}

pub struct PositionPenaltyList {
    pub map: HashMap<u32, PenaltyFrequency>,
}


pub struct KeyPositionPenaltyList {
    pub map: HashMap<String, Vec<KeyPositionPenalty>>,
}

pub struct MinMaxKeyPositionPenaltyList {
    pub map: HashMap<u8, MinMaxPositionPenalty>,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub struct MinMaxPositionPenalty {
    min_penalty: f64,
    max_penalty: f64,
    range: f64,
}

impl MinMaxPositionPenalty {
    pub fn new() -> Self {
        Self {
            min_penalty: f64::MAX,
            max_penalty: f64::MIN,
            range: 0.0,
        }
    }

    pub fn update(&mut self, penalty: f64) {
        if self.min_penalty > penalty {
            self.min_penalty = penalty
        }
        if self.max_penalty < penalty {
            self.max_penalty = penalty
        }
        self.range = self.max_penalty - self.min_penalty
    }
}

pub struct MinMaxFrequency {
    min_frequency: f64,
    max_frequency: f64,
    range: f64,
}

impl MinMaxFrequency {
    pub fn new() -> Self {
        Self {
            min_frequency: f64::MAX,
            max_frequency: f64::MIN,
            range: 0.0,
        }
    }

    pub fn update(&mut self, penalty: f64) {
        if self.min_frequency > penalty {
            self.min_frequency = penalty
        }
        if self.max_frequency < penalty {
            self.max_frequency = penalty
        }
        self.range = self.max_frequency - self.min_frequency
    }
}

pub type PenaltyMap = [f64; layout::NUM_OF_KEYS];

pub fn evaluate(ngram_list: NgramList) {
    let mut ngram_penalty_list: HashMap<String, Vec<KeyPositionPenalty>> = HashMap::new();
    for ngram in ngram_list.map {
        for character in ngram.0.chars() {
            let frequency = ngram.1 as f64;
            let key = character as u8;
            for i in 0..layout::NUM_OF_KEYS {
                let penalty = BASE_PENALTY[i] * frequency;
                let position = i as u32;
                let position_penalty = KeyPositionPenalty {
                    key,
                    position,
                    penalty,
                    frequency
                };
                ngram_penalty_list
                    .entry(ngram.0.to_string())
                    .and_modify(|key_penalty_list| {
                        key_penalty_list.push(position_penalty);
                    })
                    .or_insert_with(|| {
                        let mut new_vec: Vec<KeyPositionPenalty> = Vec::new();
                        new_vec.push(position_penalty);
                        new_vec
                    });
            }
        }
    }
    let mut key_position_list: HashMap<u8, PositionPenaltyList> = HashMap::new();
    for (key, val) in ngram_penalty_list {
        for item in val {
            key_position_list
                .entry(item.key)
                .and_modify(|penalty_list| {
                    penalty_list
                        .map
                        .entry(item.position)
                        .and_modify(|entry| {
                            entry.penalty = entry.penalty + item.penalty;
                            entry.frequency = entry.frequency + item.frequency;
                        })
                        .or_insert_with(|| {
                            let penalty_frequency = PenaltyFrequency {
                                penalty: item.penalty,
                                frequency: item.frequency,
                            };
                            penalty_frequency
                        });
                })
                .or_insert_with(|| {
                    let penalty_map = HashMap::new();
                    let mut newVec: PositionPenaltyList = PositionPenaltyList { map: penalty_map };
                    let penalty_frequency = PenaltyFrequency {
                        penalty: item.penalty,
                        frequency: item.frequency,
                    };
                    newVec.map.insert(item.position, penalty_frequency);
                    newVec
                });
        }
    }
    let mut min_max_key_position_list: HashMap<u8, MinMaxPositionPenalty> = HashMap::new();
    let mut min_max_frequency: MinMaxFrequency = MinMaxFrequency::new();
    for (key, mut val) in &mut key_position_list {
        for item in &mut val.map {
            min_max_frequency.update(item.1.frequency);
            min_max_key_position_list
                .entry(*key)
                .and_modify(|min_max_position_penalty| {
                    min_max_position_penalty.update(item.1.penalty);
                })
                .or_insert_with(|| {
                    let mut min_max_position_penalty = MinMaxPositionPenalty::new();
                    min_max_position_penalty.update(item.1.penalty);
                    min_max_position_penalty
                });
        }
    }

    for (key, mut val) in &mut key_position_list {
        for item in &mut val.map {
            let min_max_position_penalty = min_max_key_position_list
                .entry(*key).or_default();

            let mut penalty_frequency = *item.1;
            //normalize penalty values for current character
            penalty_frequency.penalty = (penalty_frequency.penalty - min_max_position_penalty.min_penalty) / min_max_position_penalty.range;

            //normalize frequency values across all characters
            penalty_frequency.frequency = (penalty_frequency.frequency - min_max_frequency.min_frequency) / min_max_frequency.range;

            *item.1 = penalty_frequency;
        }
    }
}

use crate::{corpus_manager::NgramList, file_manager::*, layout, penalty::BASE_PENALTY};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash, ops::Index, cmp::Ordering};

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
    frequency: f64,
    penalty_normalized: f64,
    frequency_normalized: f64
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct keyPenaltyFrequency {
    key: u8,
    penalty: f64,
    frequency: f64
}

impl Ord for keyPenaltyFrequency {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.partial_cmp(&other) {
            Some(ord) => ord,
            None => std::cmp::Ordering::Equal,
        }
    }
}
impl PartialEq for keyPenaltyFrequency {
    fn eq(&self, other: &Self) -> bool {
        self.penalty == other.penalty
    }
}
impl Eq for keyPenaltyFrequency {}
impl PartialOrd for keyPenaltyFrequency {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.frequency.partial_cmp(&other.frequency)
        //chain_ordering(self.frequency.partial_cmp(&other.frequency),self.penalty.partial_cmp(&other.penalty))   
    }
}

fn chain_ordering(o1: Option<std::cmp::Ordering>, o2: Option<std::cmp::Ordering>) -> Option<std::cmp::Ordering> {
    match o1{
        Some(ord) => {
            match ord {
                Ordering::Equal => {
                    match o2 {
                        Some(ord2) => {
                            match ord2 {
                                Ordering::Equal => Some(std::cmp::Ordering::Equal),
                                _ => Some(ord2.reverse()),
                            }
                        },
                        None => Some(std::cmp::Ordering::Equal),
                    }
                },
                _ => Some(ord),
            }
        },
        None => Some(std::cmp::Ordering::Equal),
    }
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
                                penalty_normalized: 0.0,
                                frequency_normalized: 0.0
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
                        penalty_normalized: 0.0,
                        frequency_normalized: 0.0
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
            penalty_frequency.penalty_normalized = (penalty_frequency.penalty - min_max_position_penalty.min_penalty) / min_max_position_penalty.range;

            //normalize frequency values across all characters
            penalty_frequency.frequency_normalized = (penalty_frequency.frequency - min_max_frequency.min_frequency) / min_max_frequency.range;

            *item.1 = penalty_frequency;
        }
    }

    let mut key_penalty_frequency_map: HashMap<u32, Vec<keyPenaltyFrequency>> = HashMap::new();
    for (key, val) in &mut key_position_list {
        for item in &mut val.map {
            key_penalty_frequency_map
            .entry(*item.0)
            .and_modify(|key_penalty_frequency_list| {
                let key_penalty_frequency = keyPenaltyFrequency { key: *key, penalty:item.1.penalty_normalized, frequency:item.1.frequency_normalized };
                key_penalty_frequency_list.push(key_penalty_frequency);
                key_penalty_frequency_list.sort_by(|a,b| a.cmp(b).reverse() );
            })
            .or_insert_with(|| {
                let mut key_penalty_frequency_list: Vec<keyPenaltyFrequency> = Vec::new();
                let key_penalty_frequency = keyPenaltyFrequency { key: *key, penalty:item.1.penalty_normalized, frequency:item.1.frequency_normalized };
                key_penalty_frequency_list.push(key_penalty_frequency);
                key_penalty_frequency_list
            });
        }
    }

    pub struct PositionPenalty {
        position: u32,
        penalty: f64,
    }
    let mut position_penalty_list: Vec<PositionPenalty> = Vec::new();
    for (index,penalty) in BASE_PENALTY.into_iter().enumerate() {
        position_penalty_list.push(PositionPenalty{position: index as u32, penalty: penalty })
    }
    position_penalty_list.sort_by(|a,b|a.penalty.partial_cmp(&b.penalty).unwrap());

    let position_penalty_groups = &position_penalty_list
    .into_iter()
    .group_by(|elt| elt.penalty);

    pub struct KeyPosition {
        position: u32,
        key: u32
    }

    pub struct KeyPositionList {
        positions: Vec<KeyPosition>
    }

    pub struct PositionGroup {
        positions: Vec<u32>,
        count: usize,
        index: u32
    }

    let mut position_penalty_group_list: Vec<PositionGroup> = Vec::new();
    let mut groupIndex = 0;
    for (key, group) in position_penalty_groups {
        let group_list = &group.collect_vec();
        let group_positions = group_list.into_iter().map(|g|g.position).collect();
        let group_count = group_list.len();
        let position_group = PositionGroup {
            positions : group_positions,
            count : group_count,
            index : groupIndex
        };
        position_penalty_group_list.push(position_group);
        groupIndex = groupIndex + 1;
    }

    let mut position_penalty_group_list: Vec<KeyPositionList> = Vec::new();
    for (item) in position_penalty_group_list{
        
    }
}

use crate::{
    corpus_manager::NgramList,
    file_manager::*,
    layout::{self, Finger, KeyMap, LayerKeys, Row, KEY_FINGERS, KEY_ROWS, NUM_OF_KEYS},
    penalty::BASE_PENALTY,
};
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap, hash::Hash, ops::Index, slice::Iter};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct KeyFrequency {
    key: u8,
    frequency: f64,
    frequency_normalized: f64,
}

impl PartialEq for KeyFrequency {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.frequency == other.frequency
            && self.frequency_normalized == other.frequency_normalized
    }
}
impl Eq for KeyFrequency {}

impl Hash for KeyFrequency {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

// impl PartialOrd for KeyFrequency {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         self.frequency.partial_cmp(&other.frequency)

//     }
// }

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

    pub fn update(&mut self, frequency: f64) {
        if self.min_frequency > frequency {
            self.min_frequency = frequency
        }
        if self.max_frequency < frequency {
            self.max_frequency = frequency
        }
        self.range = self.max_frequency - self.min_frequency
    }
}

pub fn evaluate(ngram_list: NgramList) {
    let mut ngram_penalty_list: Vec<KeyFrequency> = Vec::new();
    for ngram in ngram_list.map {
        for character in ngram.0.chars() {
            let frequency = ngram.1 as f64;
            let key = character as u8;

            let key_frequency = KeyFrequency {
                key,
                frequency,
                frequency_normalized: 0.0,
            };
            ngram_penalty_list.push(key_frequency);
        }
    }

    let elements_to_remove = ['e', 't'];
    for element in elements_to_remove {
        ngram_penalty_list.retain(|keyfreq| keyfreq.key != element as u8);
    }

    let mut min_max_frequency: MinMaxFrequency = MinMaxFrequency::new();
    for item in &ngram_penalty_list {
        min_max_frequency.update(item.frequency);
    }

    for item in &mut ngram_penalty_list {
        let mut key_frequency = *item;

        //normalize frequency values across all characters
        key_frequency.frequency_normalized =
            (key_frequency.frequency - min_max_frequency.min_frequency) / min_max_frequency.range;

        *item = key_frequency;
    }

    ngram_penalty_list.sort_by(|a, b| {
        a.frequency_normalized
            .partial_cmp(&b.frequency_normalized)
            .unwrap()
            .reverse()
    });

    pub struct PositionPenalty {
        position: u32,
        penalty: f64,
    }
    let mut position_penalty_list: Vec<PositionPenalty> = Vec::new();

    let thumb_indeces: Vec<usize> = KEY_FINGERS
        .into_iter()
        .enumerate()
        .filter(|f| f.1 == Finger::Thumb)
        .map(|f| f.0)
        .collect();
    let mut bad_finger_indeces: Vec<usize> = Vec::new();

    for (finger_index, finger) in KEY_FINGERS.into_iter().enumerate() {
        for (row_index, row) in KEY_ROWS.into_iter().enumerate() {
            if (finger_index == row_index) {
                let bad_position = (finger, row);
                match bad_position {
                    (Finger::Index, Row::Top) => bad_finger_indeces.push(row_index),
                    //investigate how this fixes issues
                    (Finger::Middle, Row::Bottom) => bad_finger_indeces.push(row_index),
                    _ => (),
                }
            }
        }
    }

    for (index, penalty) in BASE_PENALTY
        .into_iter()
        .enumerate()
        .filter(|b| !thumb_indeces.contains(&b.0))
        .filter(|b| !bad_finger_indeces.contains(&b.0))
    {
        position_penalty_list.push(PositionPenalty {
            position: index as u32,
            penalty: penalty,
        })
    }
    position_penalty_list.sort_by(|a, b| a.penalty.partial_cmp(&b.penalty).unwrap());

    let position_penalty_groups = &position_penalty_list
        .into_iter()
        .group_by(|elt| elt.penalty);

    pub struct PositionGroup {
        positions: Vec<u32>,
        count: usize,
        index: u32,
    }

    let mut position_penalty_group_list: Vec<PositionGroup> = Vec::new();
    let mut groupIndex = 0;
    for (_key, group) in position_penalty_groups {
        let group_list = &group.collect_vec();
        let group_positions = group_list.into_iter().map(|g| g.position).collect();
        let group_count = group_list.len();
        let position_group = PositionGroup {
            positions: group_positions,
            count: group_count,
            index: groupIndex,
        };
        position_penalty_group_list.push(position_group);
        groupIndex = groupIndex + 1;
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct KeyPosition {
        key: u8,
        position: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct KeyPositionList {
        positions: Vec<KeyPosition>,
    }

    let mut key_position_group_list: Vec<Vec<Vec<KeyPosition>>> = Vec::new();

    for (item) in position_penalty_group_list {
        let frequency_slice: Vec<KeyFrequency> = ngram_penalty_list.drain(0..item.count).collect();
        let mut key_position_grouping: Vec<Vec<KeyPosition>> = Vec::new();
        for perm in frequency_slice
            .iter()
            .permutations(frequency_slice.len())
            .unique()
        {
            //let mut positions: Vec<KeyPosition> = Vec::new();
            let mut key_positions_list: Vec<KeyPosition> = Vec::new();

            for keyfreq in perm.into_iter().enumerate() {
                key_positions_list.push(KeyPosition {
                    position: item.positions[keyfreq.0],
                    key: keyfreq.1.key,
                })
            }
            key_position_grouping.push(key_positions_list);
        }
        key_position_group_list.push(key_position_grouping);
    }

    let mut key_position_group_iter_list: Vec<Iter<'_, KeyPositionList>> = Vec::new();

    let mut key_position_group_combinations: Vec<Vec<Vec<KeyPosition>>> = Vec::new();

    //key_position_group_list.remove(5);

    // for group in key_position_group_list {
    //     key_position_group_iter_list.push(group.iter());
    // }
    let mut key_position_group_combinations3: Vec<Vec<Vec<KeyPosition>>> = Vec::new();

    // let g1 = key_position_group_list[0].clone();
    // let g2 = key_position_group_list[1].clone();
    // let g3 = key_position_group_list[2].clone();
    // let g4 = key_position_group_list[3].clone();
    // let g5 = key_position_group_list[4].clone();
    // let g6 = key_position_group_list[5].clone();
    // let g1 = key_position_group_list[3].clone().iter();
    // let g1 = key_position_group_list[4].clone().iter();
    // let g1 = key_position_group_list[5].clone().iter();
    // let us = g1.iter();
    // let vs = g2.iter();
    // let ws = g3.iter();
    // let xs = g4.iter();
    // let ys = g5.iter();
    // let zs = g6.iter();

    // let us = &us;
    // let vs = &vs;
    // let ws = &ws;
    // let xs = &xs;
    // let ys = &ys;
    //let zs = &zs;
    // let uvwxyzs =
    // zs.flat_map(move |z| ys.clone()
    // .flat_map(move |y| xs.clone()
    // .flat_map(move |x| ws.clone()
    // .flat_map(move |w| vs.clone()
    // .flat_map(move |v| us.clone()
    // .map(move |u| (x.clone(), y.clone(), z.clone(), w.clone(), v.clone(), u.clone())))))));

    // let uvwxyzs =
    // zs.flat_map(move |z| ys.clone()
    // .flat_map(move |y| xs.clone()
    // //.flat_map(move |x| ws.clone()
    // // .flat_map(move |w| vs.clone()
    // // .flat_map(move |v| us.clone()
    // .map(move |u| (y.clone(), z.clone(), u.clone()))));

    // for item in uvwxyzs{
    //     //println!("{:?}", item);
    //     let items: Vec<KeyPosition> = Vec::new();
    //     let mut newitem: Vec<Vec<KeyPosition>> = Vec::new();
    //     newitem.push(item.0);
    //     newitem.push(item.1);
    //     newitem.push(item.2);
    //     key_position_group_combinations3.push(newitem)
    // }

    //println!("{:?}", key_position_group_combinations3.len());

    // let mut key_position_group_list2: Vec<Vec<Vec<KeyPosition>>> = Vec::new();
    // key_position_group_list2.push(key_position_group_list[0].clone());
    // key_position_group_list2.push(key_position_group_list[1].clone());
    // key_position_group_list2.push(key_position_group_list[2].clone());
    // key_position_group_list2.push(key_position_group_list[3].clone());
    //key_position_group_list2.push(key_position_group_list[4].clone());
    //key_position_group_list2.push(key_position_group_list[5].clone());

    key_position_group_list.sort_by(|a, b| a.len().cmp(&b.len()));

    let biggest_group = key_position_group_list.pop().unwrap();
    let mut singlebig: Vec<Vec<KeyPosition>> = Vec::new();
    singlebig.push(biggest_group.get(0).unwrap().to_vec());
    // let final_combinations : Vec<Vec<Vec<KeyPosition>>> = biggest_group.into_par_iter().map(move|g|{
    //     let filename = g.clone().iter().map(|kp|kp.key as char).join("_");
    //     let folder = String::from("\\evaluation\\");

    //     let mut single_item = Vec::new();
    //     single_item.push(g);

    //     let mut key_position_group_list_copy = key_position_group_list.clone();
    //     key_position_group_list_copy.push(single_item);

    //     let mut single_key_position_group_combinations: Vec<Vec<Vec<KeyPosition>>> = Vec::new();
    //     for combination in key_position_group_list_copy.into_iter().map(IntoIterator::into_iter).multi_cartesian_product() {
    //         single_key_position_group_combinations.push(combination);
    //     }
    //     //println!("{:?}", single_key_position_group_combinations.len());

    //     save_file::<Vec<Vec<Vec<KeyPosition>>>>(String::from(filename), String::from(folder), &single_key_position_group_combinations);

    //     single_key_position_group_combinations
    // }).flat_map(|m|m).collect();

    let final_combinations = singlebig.into_par_iter().map(move|g|{
        let filename = g.clone().iter().map(|kp|kp.key as char).join("_");
        let folder = String::from("\\evaluation\\");

        let mut single_item = Vec::new();
        single_item.push(g);

        let mut key_position_group_list_copy = key_position_group_list.clone();
        key_position_group_list_copy.push(single_item);

        //let mut single_key_position_group_combinations: Vec<LayerKeys> = Vec::new();
        let mut single_key_position_group_combinations: Vec<String> = Vec::new();
        for combination in key_position_group_list_copy.into_iter().map(IntoIterator::into_iter).multi_cartesian_product() {
            let mut single_layout: KeyMap = [(); NUM_OF_KEYS].map(|_| ' ');
            for kp in combination.iter().flatten(){
                single_layout[kp.position as usize] = kp.key as char;
            };
            //let map = String::from_iter(combination.into_iter().flatten().sorted_by(|p1, p2|p1.position.cmp(&p2.position)).map(|k|k.key as char));
            let map = String::from_iter(single_layout);
            // for kp in combination.into_iter().flatten().sorted_by(|p1, p2|p1.position.cmp(&p2.position)).map(|k|k.key as char){
            //     single_layout[kp.position as usize] = kp.key as char;
            // };

            single_key_position_group_combinations.push(map);
            // single_key_position_group_combinations.push(LayerKeys::new(single_layout));
        }
        //println!("{:?}", single_key_position_group_combinations.len());
        save_small_file::<Vec<String>>(String::from(filename), String::from(folder), &single_key_position_group_combinations);
        // save_small_file::<Vec<LayerKeys>>(String::from(filename), String::from(folder), &single_key_position_group_combinations);

        single_key_position_group_combinations.len()
    }).reduce(|| 0, |a, b| a+b);

    // println!("{:?}", final_combinations);

    // for combination in key_position_group_list.into_iter().map(IntoIterator::into_iter).multi_cartesian_product() {
    //     key_position_group_combinations.push(combination);
    // }

    // println!("{:?}", key_position_group_combinations.len());
    //return;
    // let mut key_position_group_combinations2: Vec<Vec<Vec<KeyPosition>>> = Vec::new();
    let test2 = "";
    // let test = "";
    // let mut key_position_group_list3: Vec<Vec<Vec<KeyPosition>>> = Vec::new();
    // key_position_group_list3.push(key_position_group_list[4].clone());
    // key_position_group_list3.push(key_position_group_list[5].clone());
    // key_position_group_list3.push(key_position_group_combinations[0].clone());

    // for combination in key_position_group_list3.into_iter().map(IntoIterator::into_iter).multi_cartesian_product() {
    //     key_position_group_combinations2.push(combination);
    // }

    // let mut key_position_group_combinations_flattened: Vec<Vec<KeyPosition>> = Vec::new();

    // for combination in key_position_group_combinations.into_iter() {
    //     key_position_group_combinations_flattened.push(combination.into_iter().map(|group| group).flatten().collect::<Vec<KeyPosition>>());
    // }
}

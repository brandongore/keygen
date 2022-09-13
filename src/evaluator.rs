use crate::{
    corpus_manager::{ NgramList, normalize_ngram_list },
    file_manager::{ self, * },
    layout::{
        self,
        Finger,
        KeyMap,
        LayerKeys,
        Layout,
        Row,
        KEY_FINGERS,
        KEY_ROWS,
        NUM_OF_KEYS,
        Layer,
        get_empty_position_map,
    },
    penalty::{ self, calculate_penalty, BestLayoutsEntry, Penalty, BASE_PENALTY, KeyPenalty },
    timer::{ Timer, TimerState },
    evaluator_penalty::{
        calculate_position_penalty,
        self,
        PosRelation,
        PenaltyType,
        DisplayPosRelation,
        RelationMap,
        PosKeyPenalty,
        KeyFrequencyPenalty,
    },
};
use chrono::Utc;
use dashmap::DashMap;
use itertools::Itertools;
use jwalk::Parallelism;
use rayon::{
    iter::{
        IndexedParallelIterator,
        IntoParallelIterator,
        IntoParallelRefIterator,
        ParallelBridge,
        ParallelIterator,
    },
    slice::{ ParallelSlice, ParallelSliceMut },
};
use serde::{ Deserialize, Serialize };
use std::{ cmp::Ordering, collections::HashMap, hash::Hash, ops::Index, slice::Iter, sync::Arc };

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct KeyFrequency {
    key: u8,
    frequency: f64,
    frequency_normalized: f64,
}

impl PartialEq for KeyFrequency {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key &&
            self.frequency == other.frequency &&
            self.frequency_normalized == other.frequency_normalized
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
            self.min_frequency = frequency;
        }
        if self.max_frequency < frequency {
            self.max_frequency = frequency;
        }
        self.range = self.max_frequency - self.min_frequency;
    }
}

pub struct PositionPenalty {
    position: u32,
    penalty: f64,
}

pub struct PositionGroup {
    positions: Vec<u32>,
    count: usize,
    index: u32,
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

pub fn evaluate_by_ngram_frequency(ngram_list: NgramList) {
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
        ngram_penalty_list.retain(|keyfreq| keyfreq.key != (element as u8));
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
        a.frequency_normalized.partial_cmp(&b.frequency_normalized).unwrap().reverse()
    });

    let mut position_penalty_list: Vec<PositionPenalty> = Vec::new();

    let thumb_indeces: Vec<usize> = KEY_FINGERS.into_iter()
        .enumerate()
        .filter(|f| (f.1 == Finger::Thumb || f.1 == Finger::ThumbBottom))
        .map(|f| f.0)
        .collect();

    let mut bad_finger_indeces: Vec<usize> = Vec::new();

    for (finger_index, finger) in KEY_FINGERS.into_iter().enumerate() {
        for (row_index, row) in KEY_ROWS.into_iter().enumerate() {
            if finger_index == row_index {
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

    for (index, penalty) in BASE_PENALTY.into_iter()
        .enumerate()
        .filter(|b| !thumb_indeces.contains(&b.0))
        .filter(|b| !bad_finger_indeces.contains(&b.0)) {
        position_penalty_list.push(PositionPenalty {
            position: index as u32,
            penalty: penalty,
        });
    }
    position_penalty_list.sort_by(|a, b| a.penalty.partial_cmp(&b.penalty).unwrap());

    let position_penalty_groups = &position_penalty_list.into_iter().group_by(|elt| elt.penalty);

    let mut position_penalty_group_list: Vec<PositionGroup> = Vec::new();
    let mut groupIndex = 0;
    for (_key, group) in position_penalty_groups {
        let group_list = &group.collect_vec();
        let group_positions = group_list
            .into_iter()
            .map(|g| g.position)
            .collect();
        let group_count = group_list.len();
        let position_group = PositionGroup {
            positions: group_positions,
            count: group_count,
            index: groupIndex,
        };
        position_penalty_group_list.push(position_group);
        groupIndex = groupIndex + 1;
    }

    let mut key_position_group_list: Vec<Vec<Vec<KeyPosition>>> = Vec::new();

    for item in position_penalty_group_list {
        let frequency_slice: Vec<KeyFrequency> = ngram_penalty_list.drain(0..item.count).collect();
        let mut key_position_grouping: Vec<Vec<KeyPosition>> = Vec::new();
        for perm in frequency_slice.iter().permutations(frequency_slice.len()).unique() {
            //let mut positions: Vec<KeyPosition> = Vec::new();
            let mut key_positions_list: Vec<KeyPosition> = Vec::new();

            for keyfreq in perm.into_iter().enumerate() {
                key_positions_list.push(KeyPosition {
                    position: item.positions[keyfreq.0],
                    key: keyfreq.1.key,
                });
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
    // let mut singlebig: Vec<Vec<KeyPosition>> = Vec::new();
    // singlebig.push(biggest_group.get(0).unwrap().to_vec());
    // singlebig.push(biggest_group.get(1).unwrap().to_vec());
    // singlebig.push(biggest_group.get(2).unwrap().to_vec());
    // singlebig.push(biggest_group.get(3).unwrap().to_vec());
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

    let thumb_bottom_indeces: Vec<usize> = KEY_FINGERS.into_iter()
        .enumerate()
        .filter(|f| f.1 == Finger::ThumbBottom)
        .map(|f| f.0)
        .collect();

    let final_combinations = biggest_group
        .into_par_iter()
        .map(move |g| {
            let filename = g
                .clone()
                .iter()
                .map(|kp| kp.key as char)
                .join("_");
            let folder = String::from("\\evaluation\\");

            let mut single_item = Vec::new();
            single_item.push(g);

            let mut key_position_group_list_copy = key_position_group_list.clone();
            key_position_group_list_copy.push(single_item);

            let mut single_key_position_group_combinations: Vec<LayerKeys> = Vec::new();
            let mut single_key_position_group_combinations: Vec<String> = Vec::new();
            for combination in key_position_group_list_copy
                .into_iter()
                .map(IntoIterator::into_iter)
                .multi_cartesian_product() {
                let mut single_layout: KeyMap = [(); NUM_OF_KEYS].map(|_| ' ');

                single_layout[31] = 'e';
                single_layout[34] = 't';
                single_layout[35] = '\n';

                for kp in combination.iter().flatten() {
                    single_layout[kp.position as usize] = kp.key as char;
                }
                //let map = String::from_iter(combination.into_iter().flatten().sorted_by(|p1, p2|p1.position.cmp(&p2.position)).map(|k|k.key as char));
                let map = String::from_iter(single_layout);
                // for kp in combination.into_iter().flatten().sorted_by(|p1, p2|p1.position.cmp(&p2.position)).map(|k|k.key as char){
                //     single_layout[kp.position as usize] = kp.key as char;
                // };

                single_key_position_group_combinations.push(map);
                // single_key_position_group_combinations.push(LayerKeys::new(single_layout));
            }
            //println!("{:?}", single_key_position_group_combinations.len());
            save_small_file::<Vec<String>>(
                String::from(filename),
                String::from(folder),
                &single_key_position_group_combinations
            );
            // save_small_file::<Vec<LayerKeys>>(String::from(filename), String::from(folder), &single_key_position_group_combinations);

            single_key_position_group_combinations.len()
        })
        .reduce(
            || 0,
            |a, b| a + b
        );

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

pub fn evaluate_layouts(
    ngram_list: NgramList,
    mut layouts: Vec<FileResult<Vec<String>>>,
    timer: &mut HashMap<String, TimerState>
) {
    //-> Vec<FileResult<Vec<BestLayoutsEntry>>>
    let mut best_layout_results: Vec<FileResult<Vec<BestLayoutsEntry>>> = Vec::new();
    timer.start(String::from("ngrams"));
    let processed_ngrams: Vec<(Vec<char>, usize)> = ngram_list.map
        .into_iter()
        .map(|item| (item.0.chars().collect(), item.1))
        .collect();
    timer.stop(String::from("ngrams"));
    // let first = &layouts[0].data[0..1][0];

    // let layout = Layout::from_lower_string(&first[..]);
    // //let penalty = Penalty { penalties: Vec::new(), fingers: [(); 10].map(|_| 0), hands: [(); 2].map(|_| 0), total: 0.0, len: 0 };
    // //let best_layout = BestLayoutsEntry { layout, penalty};
    // timer.start(String::from("calculate penalty"));
    // let best_layout = calculate_penalty(&processed_ngrams, &layout);
    // timer.stop(String::from("calculate penalty"));

    for file_result in layouts.iter() {
        let mut chunked_results: Vec<FileResult<Vec<BestLayoutsEntry>>> = file_result.data
            .par_chunks(50000)
            .enumerate()
            .map(|(i, slice)| {
                let mut best_layouts: Vec<BestLayoutsEntry> = Vec::new();

                for layout_string in slice.iter() {
                    let layout = Layout::from_lower_string(&layout_string[..]);
                    let best_layout = calculate_penalty(&processed_ngrams, &layout);
                    best_layouts.push(best_layout);
                }

                let chunked_filename = [
                    file_result.filename.clone(),
                    String::from("_"),
                    i.to_string(),
                ].join("");
                let best_layouts_result = FileResult {
                    data: best_layouts,
                    filename: chunked_filename,
                };
                best_layouts_result
            })
            .collect();

        for entry in chunked_results {
            let folder = String::from("\\evaluated2\\");
            save_small_file::<Vec<BestLayoutsEntry>>(
                entry.filename,
                String::from(folder),
                &entry.data
            );
        }
        //best_layout_results.append(&mut chunked_results);
    }

    //return best_layout_results;
}

pub fn compare_layouts(
    mut layouts: Vec<file_manager::FileResult<Vec<BestLayoutsEntry>>>,
    timer: &mut HashMap<String, TimerState>
) -> Vec<BestLayoutsEntry> {
    let mut best_layout_results: Vec<BestLayoutsEntry> = Vec::new();

    // let first = &layouts[0].data[0..1][0];

    // let layout = Layout::from_lower_string(&first[..]);
    // //let penalty = Penalty { penalties: Vec::new(), fingers: [(); 10].map(|_| 0), hands: [(); 2].map(|_| 0), total: 0.0, len: 0 };
    // //let best_layout = BestLayoutsEntry { layout, penalty};
    // timer.start(String::from("calculate penalty"));
    // let best_layout = calculate_penalty(&processed_ngrams, &layout);
    // timer.stop(String::from("calculate penalty"));

    jwalk::WalkDir
        ::new(String::from("H:\\keygen\\evaluated"))
        .into_iter()
        .for_each(|dir_entry_result| {
            let dir_entry = dir_entry_result.ok().unwrap();
            if
                dir_entry.file_type().is_file() &&
                dir_entry.file_name.to_string_lossy().ends_with(&String::from(".json"))
            {
                let path = dir_entry.path();
                let filename: String = path.file_stem().unwrap().to_str().unwrap().to_owned();
                //println!("path: {}", filename);
                //println!("path2: {}", path.to_str().unwrap().to_string());
                let layouts = read_batch_json::<Vec<BestLayoutsEntry>>(
                    filename.clone(),
                    String::from("\\evaluated\\")
                );

                if layouts.is_ok() {
                    let mut items = layouts.ok().unwrap();
                    items.sort_unstable();
                    best_layout_results.extend_from_slice(&items[0..100]);
                    println!("finished: {:?}", filename);
                }
            }
        });

    // for file_result in layouts.iter() {
    //     let mut items = file_result.data.clone();
    //     items.sort_unstable(); //.sort_by(|layout1, layout2| layout1.penalty.total.total_cmp(&layout2.penalty.total));
    //     best_layout_results.extend_from_slice(&items[0..10]);
    // }

    best_layout_results.sort_unstable();
    best_layout_results.truncate(20);

    return best_layout_results;
}

pub fn refine_evaluation(
    ngram_list: NgramList,
    //mut layouts: Vec<file_manager::FileResult<Vec<BestLayoutsEntry>>>,
    layout_filepath: &String,
    dir_filetype_filter: &String,
    timer: &mut HashMap<String, TimerState>
) -> Vec<BestLayoutsEntry> {
    let mut best_layout_results: Vec<BestLayoutsEntry> = Vec::new();
    let position_map: Arc<DashMap<usize, DashMap<char, f64>>> = Arc::new(DashMap::new());

    let processed_ngrams: Vec<(Vec<char>, usize)> = ngram_list.map
        .into_iter()
        .map(|item| (item.0.chars().collect(), item.1))
        .collect();

    // jwalk::WalkDir::new(layout_filepath)
    //     .into_iter()
    //     .for_each(|dir_entry_result| {
    //         let dir_entry = dir_entry_result.ok().unwrap();
    //         if dir_entry.file_type().is_file()
    //             && dir_entry
    //                 .file_name
    //                 .to_string_lossy()
    //                 .ends_with(dir_filetype_filter)
    //         {
    //             let path = dir_entry.path();
    //             let filename: String = path.file_stem().unwrap().to_str().unwrap().to_owned();
    //             //println!("path: {}", filename);
    //             //println!("path2: {}", path.to_str().unwrap().to_string());
    //             let layouts = read_batch_json::<Vec<BestLayoutsEntry>>(
    //                 filename.clone(),
    //                 String::from("\\evaluated\\"),
    //             );

    //             if layouts.is_ok() {
    //                 layouts.ok().unwrap().par_iter().for_each(|item| {
    //                     for (position, character) in item
    //                         .layout
    //                         .get_character_positions()
    //                         .into_iter()
    //                         .enumerate()
    //                     {
    //                         let position_frequency = item.penalty.pos[position];
    //                         let position_penalty = item.penalty.pos_pen[position];

    //                         match position_map.get(&position) {
    //                             None => {
    //                                 let penalty_map: DashMap<char, f64> = DashMap::new();
    //                                 penalty_map.insert(
    //                                     character,
    //                                     position_frequency as f64 * position_penalty,
    //                                 );
    //                                 position_map.insert(position, penalty_map);
    //                             }
    //                             Some(entry) => match entry.get_mut(&character) {
    //                                 None => {
    //                                     entry.insert(
    //                                         character,
    //                                         position_frequency as f64 * position_penalty,
    //                                     );
    //                                 }
    //                                 Some(mut entry) => {
    //                                     *entry = *entry + position_frequency as f64 * position_penalty;
    //                                 }
    //                             },
    //                         }
    //                     }
    //                 });
    //                 println!("finished: {:?}", filename);
    //             }
    //         }
    //     });

    // let bestlayoutstring = String::from("xz  qjnlbfcsyaohurikvm dg pw   e  t\n");

    // jwalk::WalkDir::new(layout_filepath)
    //     .into_iter()
    //     .for_each(|dir_entry_result| {
    //         let dir_entry = dir_entry_result.ok().unwrap();
    //         if dir_entry.file_type().is_file()
    //             && dir_entry
    //                 .file_name
    //                 .to_string_lossy()
    //                 .ends_with(dir_filetype_filter)
    //         {
    //             let path = dir_entry.path();
    //             let filename: String = path.file_stem().unwrap().to_str().unwrap().to_owned();
    //             //println!("path: {}", filename);
    //             //println!("path2: {}", path.to_str().unwrap().to_string());
    //             let layouts = read_batch_json::<Vec<BestLayoutsEntry>>(
    //                 filename.clone(),
    //                 String::from("\\evaluated\\"),
    //             );

    //             if layouts.is_ok() {
    //                 layouts.ok().unwrap().par_iter().for_each(|item| {
    //                     let map: String = item.layout.get_character_positions().iter().collect();
    //                     if map.eq(&bestlayoutstring){
    //                         println!("found match: {:?}", item);
    //                     }
    //                 });

    //             }
    //         }
    //     });

    let testlayout = Layout::new(
        Layer::new(
            LayerKeys::new([
                'x',
                'z',
                ' ',
                ' ',
                'q',
                'j',
                'n',
                'l',
                'b',
                'f',
                'c',
                's',
                'y',
                'a',
                'o',
                'h',
                'u',
                'r',
                'i',
                'k',
                'v',
                'm',
                ' ',
                'd',
                'g',
                ' ',
                'p',
                'w',
                ' ',
                ' ',
                ' ',
                'e',
                ' ',
                ' ',
                't',
                '\n',
            ])
        ),
        Layer::new(
            LayerKeys::new([
                'X',
                'Z',
                ' ',
                ' ',
                'Q',
                'J',
                'N',
                'L',
                'B',
                'F',
                'C',
                'S',
                'Y',
                'A',
                'O',
                'H',
                'U',
                'R',
                'I',
                'K',
                'V',
                'M',
                ' ',
                'D',
                'G',
                ' ',
                'P',
                'W',
                ' ',
                ' ',
                ' ',
                'E',
                ' ',
                ' ',
                'T',
                '\n',
            ])
        )
    );
    let test = BestLayoutsEntry {
        layout: testlayout,
        penalty: Penalty {
            penalties: [
                KeyPenalty {
                    name: "Base".to_string(),
                    times: 3981591791622,
                    total: 1486164221039.1987,
                    show: true,
                },
                KeyPenalty {
                    name: "Same finger".to_string(),
                    times: 194053258389,
                    total: 2910798875835.0,
                    show: true,
                },
                KeyPenalty {
                    name: "Long jump hand".to_string(),
                    times: 290982213,
                    total: 1454911065.0,
                    show: true,
                },
                KeyPenalty {
                    name: "Long jump".to_string(),
                    times: 67955437,
                    total: 1359108740.0,
                    show: true,
                },
                KeyPenalty {
                    name: "Long jump consecutive".to_string(),
                    times: 43231383997,
                    total: 216826000313.0,
                    show: true,
                },
                KeyPenalty {
                    name: "Rinky/ring twist".to_string(),
                    times: 19211559687,
                    total: 192115596870.0,
                    show: true,
                },
                KeyPenalty {
                    name: "Roll reversal".to_string(),
                    times: 173721763952,
                    total: 1162842961918.0,
                    show: true,
                },
                KeyPenalty { name: "Long roll out".to_string(), times: 0, total: 0.0, show: true },
                KeyPenalty {
                    name: "Alternation".to_string(),
                    times: 1363222614834,
                    total: -545289045933.6003,
                    show: true,
                },
                KeyPenalty {
                    name: "Roll out".to_string(),
                    times: 556666764718,
                    total: 858485322915.0,
                    show: true,
                },
                KeyPenalty {
                    name: "Roll in".to_string(),
                    times: 556666764718,
                    total: -1787698377833.0,
                    show: true,
                },
                KeyPenalty {
                    name: "long jump sandwich".to_string(),
                    times: 23187461149,
                    total: 69562383447.0,
                    show: true,
                },
                KeyPenalty {
                    name: "twist".to_string(),
                    times: 22429786004,
                    total: 166317296658.0,
                    show: true,
                },
                KeyPenalty {
                    name: "4 times no alternation".to_string(),
                    times: 0,
                    total: 0.0,
                    show: false,
                },
                KeyPenalty {
                    name: "4 alternations in a row".to_string(),
                    times: 0,
                    total: 0.0,
                    show: false,
                },
                KeyPenalty {
                    name: "same finger trigram".to_string(),
                    times: 19946210118,
                    total: 199462101180.0,
                    show: true,
                },
                KeyPenalty {
                    name: "same finger trigram".to_string(),
                    times: 19946210118,
                    total: 199462101180.0,
                    show: true,
                },
            ],
            pos: [
                11031838001, 6726490369, 0, 0, 6870831463, 9167198105, 257860516467, 191422733780,
                65671254678, 62123627812, 183122613426, 211005060507, 25519238108, 352931359109,
                312563124261, 106134681925, 150430123563, 315159860574, 397429918429, 27007938135,
                64209753628, 130480126884, 0, 102313643199, 74813392218, 0, 121491293804, 46030035008,
                0, 0, 0, 431205392817, 0, 0, 318869745352, 0,
            ],
            pos_pen: [
                12894.00000000041, 10911.800000000409, 0.0, 0.0, 7246.200000000159, 11524.20000000035,
                8179.799999999922, 7124.19999999991, 5999.400000000071, 6336.90000000008,
                6801.399999999922, 9198.80000000016, 7124.5999999999585, 8421.400000000496,
                5961.100000000245, 4471.700000000076, 6357.700000000266, 4304.599999999873,
                7874.400000000431, 5768.799999999888, 8798.600000000033, 11380.199999999882, 0.0, 9571.100000000204,
                8132.400000000117, 0.0, 11192.999999999882, 9685.600000000177, 0.0, 0.0, 0.0, 2062.1999999999,
                0.0, 0.0, 1661.5999999999374, 0.0,
            ],
            fingers: [
                89728991736, 752303840461, 510712348410, 274119579802, 431205392817, 73037973143,
                739093470845, 505153305463, 287367143593, 318869745352,
            ],
            hands: [2058070153226, 1923521638396],
            bad_score_total: 4932401356213.608,
            good_score_total: 4932401356213.608,
            total: 4932401356213.608,
            len: 3981591791622,
        },
    };

    //     q  z     |    x  j
    //     c  n  d  | f  s  l
    //  y  i  o  h  | u  a  r  v
    //  w  p     b  | g     m  k
    //              |
    //        e     |    t

    //        0.758 0.955 0.000 | 0.000 0.933 0.939
    //        0.711 0.614 0.590 | 0.530 0.681 0.702
    //  0.573 0.686 0.474 0.408 | 0.577 0.511 0.566 0.407
    //  0.830 0.949 0.000 0.688 | 0.724 0.000 1.000 0.798
    //                    0.000 | 0.000
    //        0.000 0.179 0.000 | 0.000 0.150 0.000

    //        0.002 0.002 0.000 | 0.000 0.003 0.002
    //        0.046 0.065 0.026 | 0.016 0.053 0.048
    //  0.006 0.100 0.079 0.027 | 0.038 0.089 0.079 0.016
    //  0.012 0.031 0.000 0.016 | 0.019 0.000 0.033 0.007
    //                    0.000 | 0.000
    //        0.000 0.108 0.000 | 0.000 0.080 0.000
    //  hands: 0.518 | 0.482
    //  total: 3506354507682.60; scaled: 0.8806

    //  Name                           | % times |   Avg   | % Total  | Total
    //  ----------------------------------------------------------------------
    //  Base                           | 100.00  | 0.373   | 42.385   | 1486164221039
    //  Same finger                    | 3.31    | 0.497   | 56.446   | 1979186459025
    //  Long jump hand                 | 0.01    | 0.000   | 0.039    | 1365206725
    //  Long jump                      | 0.00    | 0.000   | 0.013    | 448601560
    //  Long jump consecutive          | 0.50    | 0.025   | 2.843    | 99695593186
    //  Rinky/ring twist               | 0.18    | 0.018   | 2.076    | 72781156810
    //  Roll reversal                  | 4.06    | 0.282   | 31.971   | 1121008337918
    //  Long roll out                  | 0.00    | 0.000   | 0.000    | 0
    //  Alternation                    | 35.88   | -0.144  | -20.3095  | -571363524896
    //  Roll out                       | 13.12   | 0.197   | 22.358   | 783961854673
    //  Roll in                        | 13.12   | -0.431  | -48.893  | -1714368528818
    //  long jump sandwich             | 0.40    | 0.012   | 1.347    | 47213701122
    //  twist                          | 0.47    | 0.029   | 3.242    | 113668981788
    //  same finger trigram            | 0.22    | 0.022   | 2.470    | 86592447550
    //  ----------------------------------------------------------------------

    //   1.8  17.8  14.5   6.9  | 14.4  20.30   2.3  10.8
    //  51.8 | 420.30

    // // for file_result in layouts.iter() {
    // //     file_result.data.par_iter().for_each(|item| {
    // //         for (position, character) in item
    // //             .layout
    // //             .get_character_positions()
    // //             .into_iter()
    // //             .enumerate()
    // //         {
    // //             let position_frequency = item.penalty.pos[position];
    // //             let position_penalty = item.penalty.pos_pen[position];

    // //             match character_map.get(&character) {
    // //                 None => {
    // //                     let penalty_map: DashMap<usize, f64> = DashMap::new();
    // //                     penalty_map.insert(position, position_frequency as f64 * position_penalty);
    // //                     character_map.insert(character, penalty_map);
    // //                 }
    // //                 Some(entry) => match entry.get_mut(&position) {
    // //                     None => {
    // //                         entry.insert(position, position_frequency as f64 * position_penalty);
    // //                     }
    // //                     Some(mut entry) => {
    // //                         *entry = position_frequency as f64 * position_penalty;
    // //                     }
    // //                 },
    // //             }
    // //         }
    // //     });
    // // }

    // println!("starting char penalties");
    // let mut save_penalty:DashMap<usize, Vec<(char, u128)>> = DashMap::new();
    // let character_penalty_groups: Vec<Vec<(char, usize)>> = position_map
    //     .par_iter()
    //     .map(|penalty_map| {
    //         let mut character_penalties: Vec<(char, u128)> = penalty_map
    //             .iter_mut()
    //             .map(|item| (*item.key(), (item.value() * 100.0) as u128))
    //             .collect::<Vec<(char, u128)>>();

    //         character_penalties.sort_by(|first, second| first.1.cmp(&second.1));

    //         save_penalty.insert(*penalty_map.key(), character_penalties.clone());

    //         if character_penalties.len() >= 3 {
    //             character_penalties
    //             .drain(0..3)
    //             .map(|(character, _)| (character, *penalty_map.key()))
    //             .collect::<Vec<(char, usize)>>()
    //         }
    //         else {
    //             character_penalties
    //             .into_iter()
    //             .map(|(character, _)| (character, *penalty_map.key()))
    //             .collect::<Vec<(char, usize)>>()
    //         }

    //     })
    //     .collect();

    // println!("save_penalty: {:?}", save_penalty);
    // println!("penalties: {:?}", character_penalty_groups);

    //foreach character, find position that has lowest score for character
    //lock in character to position and remove from all other positions, plus remove all other possibilities for locked in position
    //possible do some calculation between positions such that the overall score is lower, eg some position might be best but causes way
    //higher penalty scores for the rest of positions so could find the combination that collectively reduces the overall score
    //the next lowest penalty for character can be in the next position
    //until each character has its lowest scored penalty
    //can try lookup if that layout exists in all files
    //if it doesnt, can compare against lowest layout score overall

    //can also pick 2 best candidates for each position and create a position group

    // {
    //     0: [('x', 109952883413144586682368)],
    //     1: [('z', 58902453651353485967360)],
    //     2: [(' ', 0)],
    //     3: [(' ', 0)],
    //     4: [('q', 40554024385754861404160)],
    //     5: [('j', 82416053141547343937536)],
    //     6: [('n', 1652361967030224652599296)],
    //     7: [('l', 1095516611484865271955456)],
    //     8: [('b', 331865712961127951892480)],
    //     9: [('f', 300870539573730836742144)],
    //     10: [('c', 1058607789029008223502336)],
    //     11: [('s', 1528434730608235005345792)],
    //     12: [('y', 133601325446393015304192)],
    //     13: [('a', 2346442738370774374023168)],
    //     14: [('o', 1389988775343097473138688)],
    //     15: [('h', 764360201670597504139264)],
    //     16: [('u', 1605883233521615058763776)],
    //     17: [('r', 1132762965602327767023616)],
    //     18: [('i', 2554772113144792785879040)],
    //     19: [('k', 123312354354148151394304)],
    //     20: [('v', 423311019558925441695744)],
    //     21: [('m', 2281231160762752252772352)],
    //     22: [(' ', 0)],
    //     23: [('d', 794720922199036428025856)],
    //     24: [('g', 493575218384862208589824)],
    //     25: [(' ', 0)],
    //     26: [('p', 2160372073162695510065152)],
    //     27: [('w', 355249375735088435691520)],
    //     28: [(' ', 0)],
    //     29: [(' ', 0)],
    //     30: [(' ', 0)],
    //     31: [('e', 2788766083120951378051072)],
    //     32: [(' ', 0)],
    //     33: [(' ', 0)],
    //     34: [('t', 1748593497196488302788608)],
    //     35: [('\n', 0)],
    // }

    // {
    //     0: [('q', 47368412228962314354688), ('z', 64162914546707821756416), ('j', 83385321140415044255744), ('x', 109952883413144586682368)],
    //     1: [('q', 41491218884883615055872), ('z', 58902453651353485967360), ('j', 75311619630030190018560), ('x', 97182901601937160929280)],
    //     2: [(' ', 0)],
    //     3: [(' ', 0)],
    //     4: [('q', 40554024385754861404160), ('z', 58044027365184654802944), ('j', 74388429245197854965760), ('x', 96184826376269496057856)],
    //     5: [('q', 46440322432144915824640), ('z', 63352640890903151509504), ('j', 82416053141547343937536), ('x', 108979157436717800095744)],
    //     6: [('c', 1243675012859872705249280), ('l', 1289169818333288004059136), ('s', 1529578561118029721108480), ('n', 1652361967030224652599296)],
    //     7: [('c', 1063157712103410820448256), ('l', 1095516611484865271955456), ('s', 1296540882447523700015104), ('n', 1406536705351641810862080)],
    //     8: [('f', 304349664133858960867328), ('b', 331865712961127951892480), ('g', 371240090644624705585152), ('d', 578076085732928034701312)],
    //     9: [('f', 300870539573730836742144), ('b', 328337776329833198911488), ('g', 368029208826797417824256), ('d', 573785411026365790027776)],
    //     10: [('c', 1058607789029008223502336), ('l', 1088779520689547729960960), ('s', 1294726477493200889053184), ('n', 1394135441447442664390656)],
    //     11: [('c', 1240728801297547608981504), ('l', 1283651167901445911478272), ('s', 1528434730608235005345792), ('n', 1642834060199707380023296)],
    //     12: [('k', 124385442457863820673024), ('y', 133601325446393015304192), ('w', 215874700611231751864320), ('v', 254589207405924232200192)],
    //     13: [('r', 1711281856620992076447744), ('o', 2015303993484505283297280), ('a', 2346442738370774374023168), ('i', 2559329115556269566984192)],
    //     14: [('r', 1133867015662200718098432), ('o', 1389988775343097473138688), ('a', 1643596784803285290713088), ('i', 1791864083993996373786624)],
    //     15: [('h', 764360201670597504139264), ('u', 1602888791206909353918464)],
    //     16: [('h', 764394055415034030325760), ('u', 1605883233521615058763776)],
    //     17: [('r', 1132762965602327767023616), ('o', 1391083857301439707611136), ('a', 1643596784803949668466688), ('i', 1788573339915975542177792)],
    //     18: [('r', 1710679780964676635983872), ('o', 2015902779757220455251968), ('a', 2346442738371070189895680), ('i', 2554772113144792785879040)],
    //     19: [('k', 123312354354148151394304), ('y', 132754687908107018829824), ('w', 214130066070495685509120), ('v', 250896105894106433060864)],
    //     20: [('k', 206831822961888498548736), ('y', 222968758003728893607936), ('w', 356994010275692129812480), ('v', 423311019558925441695744)],
    //     21: [('p', 2167802017673001429893120), ('m', 2281231160762752252772352)],
    //     22: [(' ', 0)],
    //     23: [('f', 398756817769556376289280), ('b', 451704819676072514682880), ('g', 497411424929704207974400), ('d', 794720922199036428025856)],
    //     24: [('f', 394609901944084314980352), ('b', 446686566554307949232128), ('g', 493575218384862208589824), ('d', 788841902608159191597056)],
    //     25: [(' ', 0)],
    //     26: [('p', 2160372073162695510065152), ('m', 2275558416283936153927680)],
    //     27: [('k', 205790906676822098837504), ('y', 222183020202193156308992), ('w', 355249375735088435691520), ('v', 420001237964600436064256)],
    //     28: [(' ', 0)],
    //     29: [(' ', 0)],
    //     30: [(' ', 0)],
    //     31: [('e', 2788766083120951378051072)],
    //     32: [(' ', 0)],
    //     33: [(' ', 0)],
    //     34: [('t', 1748593497196488302788608)],
    //     35: [('\n', 0)],
    // }

    //     let mut cpg: Vec<Vec<(char, usize)>> = Vec::new();

    //     cpg.push([('h', 15), ('h', 16)].to_vec());
    //     cpg.push([('w', 19), ('w', 12), ('w', 27)].to_vec());
    //     cpg.push([('i', 17), ('i', 14), ('i', 18)].to_vec());
    //     cpg.push([('v', 19), ('v', 12), ('v', 27)].to_vec());
    //     cpg.push([('c', 10), ('c', 7), ('c', 11)].to_vec());
    //     cpg.push([('q', 4), ('q', 1), ('q', 5)].to_vec());
    //     cpg.push([('a', 14), ('a', 17), ('a', 13)].to_vec());
    //     cpg.push([('f', 9), ('f', 8), ('f', 24)].to_vec());
    //     cpg.push([('d', 9), ('d', 8), ('d', 24)].to_vec());
    //    // cpg.push([('t', 34)].to_vec());
    //    // cpg.push([(' ', 32), (' ', 28), (' ', 29)].to_vec());
    //     cpg.push([('r', 17), ('r', 14), ('r', 18)].to_vec());
    //     cpg.push([('b', 9), ('b', 8), ('b', 24)].to_vec());
    //     cpg.push([('k', 19), ('k', 12), ('k', 27)].to_vec());
    //     cpg.push([('g', 9), ('g', 8), ('g', 24)].to_vec());
    //    // cpg.push([('\n', 35)].to_vec());
    //     cpg.push([('x', 4), ('x', 1), ('x', 5)].to_vec());
    //     cpg.push([('j', 4), ('j', 1), ('j', 5)].to_vec());
    //     cpg.push([('z', 4), ('z', 1), ('z', 5)].to_vec());
    //     cpg.push([('y', 19), ('y', 12), ('y', 27)].to_vec());
    //     cpg.push([('s', 10), ('s', 7), ('s', 11)].to_vec());
    //     //cpg.push([('e', 31)].to_vec());
    //     cpg.push([('o', 14), ('o', 17), ('o', 13)].to_vec());
    //     cpg.push([('u', 15), ('u', 16)].to_vec());
    //     cpg.push([('p', 26), ('p', 21)].to_vec());
    //     cpg.push([('n', 10), ('n', 7), ('n', 11)].to_vec());
    //     cpg.push([('l', 10), ('l', 7), ('l', 11)].to_vec());

    //[[('h', 15), ('h', 16)],
    //[('w', 19), ('w', 12), ('w', 27)],
    //[('i', 17), ('i', 14), ('i', 18)],
    //[('m', 26), ('m', 21)],
    //[('v', 19), ('v', 12), ('v', 27)],
    //[('c', 10), ('c', 7), ('c', 11)],
    //[('q', 4), ('q', 1), ('q', 5)],
    //[('a', 14), ('a', 17), ('a', 13)]
    //[('f', 9), ('f', 8), ('f', 24)]
    //[('d', 9), ('d', 8), ('d', 24)]
    //[('t', 34)]
    //[(' ', 32), (' ', 28), (' ', 29)]
    //[('r', 17), ('r', 14), ('r', 18)]
    //[('b', 9), ('b', 8), ('b', 24)]
    //[('k', 19), ('k', 12), ('k', 27)]
    //[('g', 9), ('g', 8), ('g', 24)]
    //[('\n', 35)]
    //[('x', 4), ('x', 1), ('x', 5)]
    //[('j', 4), ('j', 1), ('j', 5)]
    //[('z', 4), ('z', 1), ('z', 5)]
    //[('y', 19), ('y', 12), ('y', 27)]
    //[('s', 10), ('s', 7), ('s', 11)]
    //[('e', 31)]
    //[('o', 14), ('o', 17), ('o', 13)]
    //[('u', 15), ('u', 16)]
    //[('p', 26), ('p', 21)]
    //[('n', 10), ('n', 7), ('n', 11)]
    //[('l', 10), ('l', 7), ('l', 11)]
    //]

    // let mut refined_layouts: Vec<String> = Vec::new();
    // let mut count:i128 = 0;

    // for combination in cpg
    //     .into_iter()
    //     .map(IntoIterator::into_iter)
    //     .multi_cartesian_product()
    //     .filter(|item| {
    //         item.iter().zip(item.iter()).all(|(first, second)| {
    //             !((first.0 != second.0 && first.1 == second.1)
    //                 || (first.0 == second.0 && first.1 != second.1))
    //         })
    //     })
    // {
    //     count += 1;
    //     println!("count: {:?}", count);
    // }

    // println!("count: {:?}", count);

    // for combination in character_penalty_groups
    //     .into_iter()
    //     .map(IntoIterator::into_iter)
    //     .multi_cartesian_product()
    //     .filter(|item| {
    //         item.iter().zip(item.iter()).all(|(first, second)| {
    //             !((first.0 != second.0 && first.1 == second.1)
    //                 || (first.0 == second.0 && first.1 != second.1))
    //         })
    //     })
    // {
    //     let mut single_layout: KeyMap = [(); NUM_OF_KEYS].map(|_| ' ');

    //     single_layout[31] = 'e';
    //     single_layout[34] = 't';
    //     single_layout[35] = '\n';

    //     for (key, position) in combination.into_iter() {
    //         single_layout[position as usize] = key as char;
    //     }
    //     //let map = String::from_iter(combination.into_iter().flatten().sorted_by(|p1, p2|p1.position.cmp(&p2.position)).map(|k|k.key as char));
    //     let map = String::from_iter(single_layout);
    //     // for kp in combination.into_iter().flatten().sorted_by(|p1, p2|p1.position.cmp(&p2.position)).map(|k|k.key as char){
    //     //     single_layout[kp.position as usize] = kp.key as char;
    //     // };

    //     refined_layouts.push(map);
    //     // single_key_position_group_combinations.push(LayerKeys::new(single_layout));
    // }

    // for layout_string in refined_layouts.iter() {
    //     let layout = Layout::from_lower_string(&layout_string[..]);
    //     let best_layout = calculate_penalty(&processed_ngrams, &layout);
    //     best_layout_results.push(best_layout);
    // }

    //build up list of top 3 positions for each character based on lowest penalty position score
    //for each 3 position array for each character, generate all the permutations across the board
    //for all permutations remove any permutation where any of the positions conflict
    //for each new layout, calculate the penalty
    //return the top lowest penalty layouts

    //best_layout_results.sort_unstable();
    //best_layout_results.truncate(10);

    return best_layout_results;
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct tri_pos {
    pub p1: usize,
    pub p2: usize,
    pub p3: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct bi_pos {
    pub p1: usize,
    pub p2: usize,
}

pub fn evaluate_positions(ngram_list: NgramList) {
    //let perms = (0..NUM_OF_KEYS).permutations(3);
    // perms.into_iter().for_each(|item|{
    //     println!("{}, {}, {}", item[0], item[1], item[2]);
    // });
    //println!("perms: {}", perms.count());

    let position_map = get_empty_position_map();
    let mut position_penalties: Vec<
        evaluator_penalty::Penalty<{ layout::NUM_OF_KEYS }>
    > = Vec::new();

    let mut dup_check: Vec<String> = Vec::new();

    for permutation in (0..layout::NUM_OF_KEYS).collect::<Vec<usize>>().iter().permutations(3) {
        let p0 = *permutation[0];
        let p1 = *permutation[1];
        let p2 = *permutation[2];
        // dup_check.push(vec![&p0.to_string(), &p1.to_string(), &p2.to_string()].iter().join("_"));

        position_penalties.push(
            calculate_position_penalty(
                *position_map.get_key_position(p0),
                *position_map.get_key_position(p1),
                *position_map.get_key_position(p2)
            )
        );
    }
    //println!("{:?}", dup_check[0]);

    // let mut dup_map: HashMap<String, usize> = HashMap::new();

    // for item in dup_check {
    //     let counter = dup_map.entry(item).or_insert(0);
    //     *counter += 1;
    // }

    // println!("dupcheck : {:?}", dup_map.entry("11_10_29".to_string()));

    // for item in dup_map {
    //     if item.1 > 1 {
    //         println!("{:?} - {:?}", item.0, item.1);
    //     }
    // }

    let mut pos_relation: [PosRelation<{ layout::NUM_OF_KEYS }>; layout::NUM_OF_KEYS] = [
        PosRelation {
            relation_map: [0.0; layout::NUM_OF_KEYS],
            penalty_types: [
                PenaltyType { type_map: [usize::MAX; layout::NUM_OF_KEYS] };
                layout::NUM_OF_KEYS
            ],
        };
        layout::NUM_OF_KEYS
    ];
    for item in &position_penalties {
        for (position, relation) in item.pos_relation.iter().enumerate() {
            for (relation_position, penalty) in relation.relation_map.iter().enumerate() {
                pos_relation[position].relation_map[relation_position] += penalty;
            }

            //print!("item: {:?}", item.tri_pos);

            //let mut penty = Vec::new();

            for (type_position, penalty_type) in relation.penalty_types.iter().enumerate() {
                let existing_types = pos_relation[position].penalty_types[type_position].type_map
                    .to_vec()
                    .into_iter()
                    .filter(|penalty| { *penalty != usize::MAX })
                    .collect::<Vec<usize>>();
                let mut types = penalty_type.type_map
                    .to_vec()
                    .into_iter()
                    .filter(|penalty| { *penalty != usize::MAX })
                    //.filter(|penalty| { *penalty != (-1 as i8) })
                    .filter(|new| { !existing_types.contains(new) })
                    .collect::<Vec<usize>>();
                //println!("types: {:?}", types);
                //penty.append(&mut types);
                //pos_relation[position].penalty_types[type_position].type_map

                if types.len() > 0 {
                    //println!("types: {:?}", types);
                    for index in 0..layout::NUM_OF_KEYS {
                        //let test = pos_relation[position].penalty_types[type_position].type_map.iter().position(predicate);
                        //if pos_relation[position].penalty_types[type_position].type_map.iter().position(predicate) == None {
                        if
                            pos_relation[position].penalty_types[type_position].type_map[index] ==
                            usize::MAX
                        {
                            if types.len() > 0 {
                                pos_relation[position].penalty_types[type_position].type_map[
                                    index
                                ] = types.pop().unwrap();
                            }
                        }
                    }
                    //}
                }
            }
            //println!("penty: {:?}", penty);
        }
    }

    let mut display_pos_relation: Vec<DisplayPosRelation> = Vec::new();

    for relation in pos_relation.iter() {
        let mut relation_map_items: Vec<f64> = Vec::new();
        let mut penalty_type_items: Vec<Vec<usize>> = Vec::new();

        relation_map_items.append(&mut relation.relation_map.into_iter().collect::<Vec<f64>>());

        for penalty_type in relation.penalty_types.iter() {
            let mut items: Vec<usize> = Vec::new();
            items.append(
                &mut penalty_type.type_map
                    .to_vec()
                    .into_iter()
                    .filter(|penalty| { *penalty != usize::MAX })
                    .collect()
            );
            penalty_type_items.push(items);
        }

        display_pos_relation.push(DisplayPosRelation {
            relation_map: relation_map_items,
            penalty_types: penalty_type_items,
        });

        //let display_relation = DisplayPosRelation { relation_map = Vec::new() }
    }

    //println!("relations: {:?}", pos_relation[0].penalty_types);
    //println!("relations: {:?}", display_pos_relation[0].relation_map);

    // print_relation(display_pos_relation[2].clone());
    // print_relation(display_pos_relation[7].clone());
    // print_relation(display_pos_relation[14].clone());
    // print_relation(display_pos_relation[19].clone());
    // print_relation(display_pos_relation[27].clone());

    // let nlist = &ngram_list.map
    //     .iter()
    //     .sorted_by(|a, b| { a.1.cmp(b.1).reverse() })
    //     .collect::<Vec<(&String, &usize)>>();

    let mut single_layout: KeyMap = [(); NUM_OF_KEYS].map(|_| ' ');

    let mut tri_permutation_penalties: Vec<(tri_pos, f64)> = Vec::new();

    let mut bigram_from_penalties: HashMap<usize, f64> = HashMap::new();

    let mut bigram_to_penalties: HashMap<usize, f64> = HashMap::new();

    for permutation in (0..layout::NUM_OF_KEYS).collect::<Vec<usize>>().iter().permutations(3) {
        //let mut tripos: Vec<tri_pos> = vec![*permutation[0], *permutation[1], *permutation[2]];
        //need to figure out better way to create permutations to take order into account

        let permutation_tripos = tri_pos {
            p1: *permutation[0],
            p2: *permutation[1],
            p3: *permutation[2],
        };

        // if *permutation[0] == 11 && *permutation[1] == 10 && *permutation[2] == 29  {
        //     println!("{:?}", permutation_tripos);
        // }

        let first_bigram_relation_penalty =
            pos_relation[*permutation[0]].relation_map[*permutation[1]];
        let second_bigram_relation_penalty =
            pos_relation[*permutation[1]].relation_map[*permutation[2]];

        let from_bigram_penalty1 = bigram_from_penalties.entry(*permutation[0]).or_insert(0.0);
        *from_bigram_penalty1 += first_bigram_relation_penalty;

        let to_bigram_penalty1 = bigram_to_penalties.entry(*permutation[1]).or_insert(0.0);
        *to_bigram_penalty1 += first_bigram_relation_penalty;

        let from_bigram_penalty2 = bigram_from_penalties.entry(*permutation[1]).or_insert(0.0);
        *from_bigram_penalty2 += second_bigram_relation_penalty;

        let to_bigram_penalty2 = bigram_to_penalties.entry(*permutation[2]).or_insert(0.0);
        *to_bigram_penalty2 += second_bigram_relation_penalty;

        tri_permutation_penalties.push((
            permutation_tripos,
            first_bigram_relation_penalty + second_bigram_relation_penalty,
        ));
    }

    tri_permutation_penalties.sort_by(|tripos1, tripos2| {
        tripos1.1.partial_cmp(&tripos2.1).unwrap()
    });

    println!("{:?}", tri_permutation_penalties[0]);
    println!("{:?}", tri_permutation_penalties[1]);
    println!("{:?}", tri_permutation_penalties[2]);
    println!("{:?}", tri_permutation_penalties[3]);
    println!("{:?}", tri_permutation_penalties[4]);
    println!("{:?}", tri_permutation_penalties[5]);
    println!("{:?}", tri_permutation_penalties[6]);
    println!("{:?}", tri_permutation_penalties[7]);
    println!("{:?}", tri_permutation_penalties[8]);
    println!("{:?}", tri_permutation_penalties[9]);
    println!("{:?}", tri_permutation_penalties[10]);
    println!("{:?}", tri_permutation_penalties[11]);
    println!("{:?}", tri_permutation_penalties[12]);
    println!("{:?}", tri_permutation_penalties[13]);
    println!("{:?}", tri_permutation_penalties[14]);
    println!("{:?}", tri_permutation_penalties[15]);
    println!("{:?}", tri_permutation_penalties[16]);
    println!("{:?}", tri_permutation_penalties[17]);
    println!("{:?}", tri_permutation_penalties[18]);
    println!("{:?}", tri_permutation_penalties[19]);
    println!("{:?}", tri_permutation_penalties[20]);
    println!("{:?}", tri_permutation_penalties[21]);
    println!("{:?}", tri_permutation_penalties[22]);
    println!("{:?}", tri_permutation_penalties[23]);
    println!("{:?}", tri_permutation_penalties[24]);
    println!("{:?}", tri_permutation_penalties[25]);
    println!("{:?}", tri_permutation_penalties[26]);
    println!("{:?}", tri_permutation_penalties[27]);
    println!("{:?}", tri_permutation_penalties[28]);
    println!("{:?}", tri_permutation_penalties[29]);
    println!("{:?}", tri_permutation_penalties[30]);
    println!("{:?}", tri_permutation_penalties[31]);
    println!("{:?}", tri_permutation_penalties[32]);
    println!("{:?}", tri_permutation_penalties[33]);
    println!("{:?}", tri_permutation_penalties[34]);
    println!("{:?}", tri_permutation_penalties[35]);
    println!("{:?}", tri_permutation_penalties[36]);
    println!("{:?}", tri_permutation_penalties[37]);
    println!("{:?}", tri_permutation_penalties[38]);
    println!("{:?}", tri_permutation_penalties[39]);
    println!("{:?}", tri_permutation_penalties[40]);

    // [
    //        0, 1, 2,     3, 4, 5,
    //        6, 7, 8,     9, 10, 11,
    // 12, 13, 14, 15,     16, 17, 18, 19,
    // 20, 21, 22, 23,     24, 25, 26, 27,
    //             28,     29,
    //     30, 31, 32,     33, 34, 35,
    // ];

    [
                 "",  "",  "",   "",  "",  "", 
                "r", "h", "c",   "x", "l", "s", 
           "k", "t", "o", "u",   "b", "i", "n", "v", 
           "z", "d", "m", "p",   "",  "g", "y",  "", 
                          "j",   "", 
                "f", "e", "q",   "", "a", "w"
    ];

    //println!("types: {:?}", display_pos_relation[0].penalty_types);
    // println!("penalty 1: {:?}", position_penalties[0]);
    // println!("-----------------------------------------");
    // println!("penalty 7000: {:?}", position_penalties[7000]);
    // println!("-----------------------------------------");
    // println!("penalty 14000: {:?}", position_penalties[14000]);
    // for item in &position_penalties {
    //     if item.total < 0.0 {
    //         good_position_penalties.push(item.clone())
    //     }
    // }

    // // // // let mut good_position_penalties: Vec<evaluator_penalty::Penalty> = Vec::new();

    // // // // for item in &position_penalties {
    // // // //     if item.total < 0.0 {
    // // // //         good_position_penalties.push(item.clone())
    // // // //     }
    // // // // }

    // // // // //println!("good pos count: {}", good_position_penalties.len());

    // // // // //println!("good pos pen 1: {}", good_position_penalties[0]);
    // // // // //println!("good pos pen 2: {}", good_position_penalties[0]);

    // // // // let mut good_positions: Vec<Vec<usize>> = good_position_penalties.iter().map(|penalty|penalty.tri_pos.to_vec()).collect();

    // // // // //println!("good pos 1: {:?}", good_positions[0]);

    // // // // let mut valid_combination: Vec<Vec<usize>> = Vec::new();

    // // // // let first_ngram = String::from("tio");
    // // // // let first_ngram_bigram1 = &first_ngram[0..2];
    // // // // let first_ngram_bigram2 = &first_ngram[1..3];

    // // // // let mut matching_ngrams: Vec<(String, usize)> = Vec::new();

    // // // // matching_ngrams.append(&mut ngram_list.clone()
    // // // //     .map
    // // // //     .into_iter()
    // // // //     .map(|item| (item.0, item.1))
    // // // //     //.map(|item| (item.0.chars().collect::<Vec<char>>(), item.1))
    // // // //     .filter(|(key, frequency)|{
    // // // //         let bigram1 = &key[0..2];
    // // // //         let bigram2 = &key[1..3];
    // // // //         if(bigram1.eq(first_ngram_bigram1) || bigram1.eq(first_ngram_bigram2) || bigram2.eq(first_ngram_bigram1) || bigram2.eq(first_ngram_bigram2)){
    // // // //             println!("bigram1 : {}- bigram2: {} - key {}", bigram1, bigram2, key);
    // // // //             return true;
    // // // //         }

    // // // //         //println!("bigram1 : {}- bigram2: {} - key {}", bigram1, bigram2, key);

    // // // //         false
    // // // //     })
    // // // //     //.map(|item| (item.0.into_iter().collect(), item.1))
    // // // //     .collect::<Vec<(String, usize)>>()
    // // // // );

    // // // // println!("matching count : {}", matching_ngrams.len());

    let single_letter_gram = normalize_ngram_list(ngram_list.clone(), 1);

    let mut sorted_single_letter_gram = single_letter_gram
        .clone()
        .map.into_iter()
        .map(|item| (item.0, item.1))
        .collect::<Vec<(String, usize)>>();

    sorted_single_letter_gram.sort_by(|ngram1, ngram2| ngram1.1.cmp(&ngram2.1).reverse());

    //println!("single_letter_gram : {:?}", sorted_single_letter_gram);
    println!("\n");
    for gram in sorted_single_letter_gram {
        println!("{:?}", gram);
    }

    let mut sorted_ngrams = ngram_list
        .clone()
        .map.into_iter()
        .map(|item| (item.0, item.1))
        .collect::<Vec<(String, usize)>>();

    sorted_ngrams.sort_by(|ngram1, ngram2| ngram1.1.cmp(&ngram2.1).reverse());
    println!("-----------------------------------------");
    println!("sorted 0: {:?} {:?}", sorted_ngrams[0].0, sorted_ngrams[0].1);
    println!("sorted 1: {:?} {:?}", sorted_ngrams[1].0, sorted_ngrams[1].1);
    println!("sorted 2: {:?} {:?}", sorted_ngrams[2].0, sorted_ngrams[2].1);
    println!("sorted 3: {:?} {:?}", sorted_ngrams[3].0, sorted_ngrams[3].1);
    println!("sorted 4: {:?} {:?}", sorted_ngrams[4].0, sorted_ngrams[4].1);
    println!("sorted 5: {:?} {:?}", sorted_ngrams[5].0, sorted_ngrams[5].1);

    let mut common_letter_list: HashMap<String, usize> = HashMap::new();

    let mut unique_bigram_letter_list: HashMap<String, usize> = HashMap::new();
    let mut unique_from_bigram_letter_frequency_list: HashMap<String, usize> = HashMap::new();
    let mut unique_to_bigram_letter_frequency_list: HashMap<String, usize> = HashMap::new();
    let mut from_letter_list: HashMap<String, usize> = HashMap::new();
    let mut to_letter_list: HashMap<String, usize> = HashMap::new();
    let mut unique_permutation_letter_list: HashMap<String, usize> = HashMap::new();
    let mut single_letter_frequency_list: HashMap<String, usize> = HashMap::new();

    for (ngram, count) in ngram_list.clone().map.into_iter() {
        let bigram1 = &ngram[0..2];
        let bigram2 = &ngram[1..3];

        let firstBigram = unique_bigram_letter_list.entry(bigram1.to_string()).or_insert(0);
        *firstBigram += 1;

        let secondBigram = unique_bigram_letter_list.entry(bigram2.to_string()).or_insert(0);
        *secondBigram += 1;

        let fromBigram1 = &ngram[0..1];
        let toBigram1 = &ngram[1..2];
        let fromBigram2 = &ngram[1..2];
        let toBigram2 = &ngram[2..3];

        let single_letter_frequency_item1 = single_letter_frequency_list
            .entry(fromBigram1.to_string())
            .or_insert(0);
        *single_letter_frequency_item1 += 1 * count;

        let single_letter_frequency_item2 = single_letter_frequency_list
            .entry(toBigram1.to_string())
            .or_insert(0);
        *single_letter_frequency_item2 += 1 * count;

        let single_letter_frequency_item3 = single_letter_frequency_list
            .entry(fromBigram2.to_string())
            .or_insert(0);
        *single_letter_frequency_item3 += 1 * count;

        let single_letter_frequency_item4 = single_letter_frequency_list
            .entry(toBigram2.to_string())
            .or_insert(0);
        *single_letter_frequency_item4 += 1 * count;

        let fromBigramPenalty1 = unique_from_bigram_letter_frequency_list
            .entry(fromBigram1.to_string())
            .or_insert(0);
        *fromBigramPenalty1 += 1 * count;

        let toBigramPenalty1 = unique_to_bigram_letter_frequency_list
            .entry(toBigram1.to_string())
            .or_insert(0);
        *toBigramPenalty1 += 1 * count;

        let fromBigramPenalty2 = unique_from_bigram_letter_frequency_list
            .entry(fromBigram2.to_string())
            .or_insert(0);
        *fromBigramPenalty2 += 1 * count;

        let toBigramPenalty2 = unique_to_bigram_letter_frequency_list
            .entry(toBigram2.to_string())
            .or_insert(0);
        *toBigramPenalty2 += 1 * count;
    }

    for (ngram, count) in unique_bigram_letter_list.clone().into_iter() {
        //ba : 3
        //be : 2
        //eb : 2
        //bi : 5

        //from b: 3
        //from e: 1
        //to b: 1
        //to e: 1
        //to a: 1
        //to i: 1

        let char1 = &ngram[0..1];
        let char2 = &ngram[1..2];

        let unique1 = unique_permutation_letter_list.entry(char1.to_string()).or_insert(0);
        *unique1 += 1;

        let unique2 = unique_permutation_letter_list.entry(char2.to_string()).or_insert(0);
        *unique2 += 1;

        let from_entry = from_letter_list.entry(char1.to_string()).or_insert(0);
        *from_entry += 1 * count;

        let to_entry = to_letter_list.entry(char2.to_string()).or_insert(0);
        *to_entry += 1 * count;
    }

    let mut sorted_unique_letter_frequency = unique_permutation_letter_list
        .clone()
        .into_iter()
        .map(|item| (item.0, item.1))
        .collect::<Vec<(String, usize)>>();

    sorted_unique_letter_frequency.sort_by(|ngram1, ngram2| ngram1.1.cmp(&ngram2.1).reverse());

    let mut sorted_from_letter_frequency = from_letter_list
        .clone()
        .into_iter()
        .map(|item| (item.0, item.1))
        .collect::<Vec<(String, usize)>>();

    sorted_from_letter_frequency.sort_by(|ngram1, ngram2| ngram1.1.cmp(&ngram2.1).reverse());

    let mut sorted_to_letter_frequency = to_letter_list
        .clone()
        .into_iter()
        .map(|item| (item.0, item.1))
        .collect::<Vec<(String, usize)>>();

    sorted_to_letter_frequency.sort_by(|ngram1, ngram2| ngram1.1.cmp(&ngram2.1).reverse());

    let mut sorted_common_letter = common_letter_list
        .clone()
        .into_iter()
        .map(|item| (item.0, item.1))
        .collect::<Vec<(String, usize)>>();

    sorted_common_letter.sort_by(|ngram1, ngram2| ngram1.1.cmp(&ngram2.1).reverse());

    println!("-----------------------------------------");
    for gram in sorted_common_letter {
        println!("{:?}", gram);
    }

    let mut common_letter_frequency_list: HashMap<String, usize> = HashMap::new();

    for (ngram, count) in common_letter_list.clone().into_iter() {
        let letter_frequency = single_letter_gram.map.get(&ngram).unwrap();

        let entry = common_letter_frequency_list.entry(ngram).or_insert(0);
        *entry = count * letter_frequency;
    }

    let mut sorted_common_letter_frequency = common_letter_frequency_list
        .clone()
        .into_iter()
        .map(|item| (item.0, item.1))
        .collect::<Vec<(String, usize)>>();

    sorted_common_letter_frequency.sort_by(|ngram1, ngram2| ngram1.1.cmp(&ngram2.1).reverse());

    println!("-----------------------------------------");
    for gram in sorted_common_letter_frequency {
        println!("{:?}", gram);
    }

    println!("--------------------UNIQUE---------------------");
    for gram in sorted_unique_letter_frequency {
        println!("{:?}", gram);
    }

    println!("--------------------FROM---------------------");
    for gram in sorted_from_letter_frequency {
        println!("{:?}", gram);
    }

    println!("--------------------TO---------------------");
    for gram in sorted_to_letter_frequency {
        println!("{:?}", gram);
    }

    let mut sorted_bigram_from_penalties = bigram_from_penalties
        .clone()
        .into_iter()
        .map(|item| (item.0.to_string(), item.1))
        .collect::<Vec<(String, f64)>>();

    sorted_bigram_from_penalties.sort_by(|ngram1, ngram2|
        ngram1.1.partial_cmp(&ngram2.1).unwrap().reverse()
    );

    let mut sorted_bigram_to_penalties = bigram_to_penalties
        .clone()
        .into_iter()
        .map(|item| (item.0.to_string(), item.1))
        .collect::<Vec<(String, f64)>>();

    sorted_bigram_to_penalties.sort_by(|ngram1, ngram2|
        ngram1.1.partial_cmp(&ngram2.1).unwrap().reverse()
    );

    println!("--------------------FROM Penalty---------------------");
    for gram in sorted_bigram_from_penalties {
        println!("{:?}", gram);
    }

    println!("--------------------TO Penalty---------------------");
    for gram in sorted_bigram_to_penalties {
        println!("{:?}", gram);
    }

    let mut sorted_single_letter_frequency_list = single_letter_frequency_list
        .clone()
        .into_iter()
        .map(|item| (item.0.to_string(), item.1))
        .collect::<Vec<(String, usize)>>();

    sorted_single_letter_frequency_list.sort_by(|ngram1, ngram2|
        ngram1.1.partial_cmp(&ngram2.1).unwrap().reverse()
    );

    println!("--------------------single letter Penalty---------------------");
    for gram in sorted_single_letter_frequency_list {
        println!("{:?}", gram);
    }

    let mut sorted_unique_from_bigram_letter_frequency_list =
        unique_from_bigram_letter_frequency_list
            .clone()
            .into_iter()
            .map(|item| (item.0.to_string(), item.1))
            .collect::<Vec<(String, usize)>>();

    sorted_unique_from_bigram_letter_frequency_list.sort_by(|ngram1, ngram2|
        ngram1.1.partial_cmp(&ngram2.1).unwrap().reverse()
    );

    println!("--------------------bigram from unique Penalty---------------------");
    for gram in sorted_unique_from_bigram_letter_frequency_list {
        println!("{:?}", gram);
    }

    let mut sorted_unique_to_bigram_letter_frequency_list = unique_to_bigram_letter_frequency_list
        .clone()
        .into_iter()
        .map(|item| (item.0.to_string(), item.1))
        .collect::<Vec<(String, usize)>>();

    sorted_unique_to_bigram_letter_frequency_list.sort_by(|ngram1, ngram2|
        ngram1.1.partial_cmp(&ngram2.1).unwrap().reverse()
    );

    println!("--------------------bigram to unique Penalty---------------------");
    for gram in sorted_unique_to_bigram_letter_frequency_list {
        println!("{:?}", gram);
    }

    //let key = character as u8;
    //let mut from_key_penalty: PosKeyPenalty<{ layout::NUM_OF_KEYS }> = PosKeyPenalty::new();
    //let mut to_key_penalty: PosKeyPenalty<{ layout::NUM_OF_KEYS }> = PosKeyPenalty::new();
    //let mut combined_key_penalty: PosKeyPenalty<{ layout::NUM_OF_KEYS }> = PosKeyPenalty::new();

    let mut dup_map: HashMap<String, f64> = HashMap::new();

    let mut from_key_penalty = [(); layout::NUM_OF_KEYS].map(|_| HashMap::<String, f64>::new());
    let mut to_key_penalty = [(); layout::NUM_OF_KEYS].map(|_| HashMap::<String, f64>::new());
    let mut combined_key_penalty = [(); layout::NUM_OF_KEYS].map(|_| HashMap::<String, f64>::new());
    // let mut test = [
    //     dup_map,
    //     36
    // ];

    for (position, penalty) in bigram_from_penalties.clone().into_iter() {
        for (key, frequency) in unique_from_bigram_letter_frequency_list.clone().into_iter() {
            let frequency_penalty = penalty * (1.0 / (frequency as f64));
            let from_key_position_penalty = from_key_penalty[position].entry(key).or_insert(0.0);
            *from_key_position_penalty = frequency_penalty;
        }
    }

    for (position, penalty) in bigram_to_penalties.clone().into_iter() {
        for (key, frequency) in unique_to_bigram_letter_frequency_list.clone().into_iter() {
            let frequency_penalty = penalty * (1.0 / (frequency as f64));
            let to_key_position_penalty = to_key_penalty[position].entry(key).or_insert(0.0);
            *to_key_position_penalty = frequency_penalty;
        }
    }

    for (position, map) in from_key_penalty.into_iter().enumerate() {
        for (key, mut penalty) in map.clone().into_iter() {
            let to_key_position_penalty = to_key_penalty[position]
                .entry(key.clone())
                .or_insert(0.0);

            let combined_key_position_penalty = combined_key_penalty[position]
                .entry(key)
                .or_insert(0.0);
            *combined_key_position_penalty = *to_key_position_penalty + penalty;
            //let frequency_penalty = penalty * (frequency as f64);
            //let to_key_position_penalty = to_key_penalty[position].entry(key).or_insert(0.0);
            //*to_key_position_penalty = frequency_penalty;
        }
    }

    let mut sorted_combined_key_position_penalty = combined_key_penalty
        .clone()
        .into_iter()
        .enumerate()
        .map(|item| {
            let mut sorted_items = item.1
                .clone()
                .into_iter()
                .map(|item| (item.0.to_string(), item.1))
                .collect::<Vec<(String, f64)>>();
            sorted_items.sort_by(|ngram1, ngram2| ngram1.1.partial_cmp(&ngram2.1).unwrap());

            (item.0, sorted_items)
        })
        .collect::<Vec<(usize, Vec<(String, f64)>)>>();

    // sorted_unique_to_bigram_letter_frequency_list.sort_by(|ngram1, ngram2|
    //     ngram1.1.partial_cmp(&ngram2.1).unwrap().reverse()
    // );

    // println!("--------------------combined from and to penalty---------------------");
    // for gram in sorted_combined_key_position_penalty.clone() {
    //     println!("{:?}", gram);
    // }

    let mut evaluated_layout = [(); layout::NUM_OF_KEYS].map(|_| "".to_string());

    let mut penalty_list = sorted_combined_key_position_penalty.clone();

    let mut last_lowest_letter = "".to_string();
    let mut last_lowest_score = f64::MAX;
    let mut last_lowest_position = 0;
    let mut last_lowest_index = 0;

    let mut positions_to_remove: Vec<usize> = Vec::new();
    let mut letters_to_remove: Vec<String> = Vec::new();

    // let initial_penalty = |
    //     list: Vec<(usize, Vec<(String, f64)>)>,
    //     position_remove_list: Vec<usize>,
    //     letter_remove_list: Vec<String>
    // | -> Vec<(usize, Vec<(String, f64)>)> {
    //     let updated_list = list
    //         .clone()
    //         .into_iter()
    //         .filter(|(position, _)| { position_remove_list.contains(position) })
    //         .map(|(position, penalties)| {
    //             let mut updated_penalties: Vec<(String, f64)> = penalties
    //                 .clone()
    //                 .into_iter()
    //                 .filter(|(letter, _)| { letter_remove_list.contains(&letter.clone()) })
    //                 .collect::<Vec<(String, f64)>>();
    //             (position, updated_penalties)
    //         })
    //         .collect();
    //     println!("remove list {:?}", letter_remove_list);
    //     return updated_list;
    // };

    for _ in (0..layout::NUM_OF_KEYS).collect::<Vec<usize>>().iter() {
        let internal_penalty_list = get_penalty_list(
            penalty_list.clone(),
            positions_to_remove.clone(),
            letters_to_remove.clone()
        );

        let mut last_lowest_letter = "".to_string();
        let mut last_lowest_score = f64::MAX;
        let mut last_lowest_position = 0;
        let mut last_lowest_index = 0;

        if internal_penalty_list.len() > 0 {
        for (index, (position, letter_penalties)) in internal_penalty_list.clone().into_iter().enumerate() {
            println!("internal_penalty_list {:?}", internal_penalty_list.clone());
            if letter_penalties.len() > 0 {
                if last_lowest_letter == "".to_string() || last_lowest_score > letter_penalties[0].1 {
                    last_lowest_letter = letter_penalties[0].0.clone();
                    last_lowest_score = letter_penalties[0].1;
                    last_lowest_position = position;
                    last_lowest_index = index;
                }
            }
            else {
                last_lowest_position = position;
                last_lowest_letter = "".to_string();
            }
        }

        evaluated_layout[last_lowest_position] = last_lowest_letter.clone();

        if last_lowest_letter != "".to_string(){
            positions_to_remove.push(last_lowest_position.clone());
            letters_to_remove.push(last_lowest_letter);
        }
        }
    }

    println!("--------------------lowest layout---------------------");
    println!("{:?}", evaluated_layout);
    // for gram in evaluated_layout.clone() {
    //     println!("{:?}", evaluated_layout);
    // }

    // for (position, penalty) in bigram_to_penalties.clone().into_iter() {
    //     let mut last_key_position = 0;
    //     for (key, frequency) in unique_from_bigram_letter_frequency_list.clone().into_iter() {

    //         to_key_penalty.pos_key_penalty[position].key_penalty_map[last_key_position] = KeyFrequencyPenalty {
    //             key: key.chars().collect::<Vec<char>>()[0] as u8,
    //             penalty: penalty * (frequency as f64),
    //         };
    //         last_key_position += 1;
    //     }
    // }

    // for (position, from_key_penalty_item) in from_key_penalty.pos_key_penalty.iter().enumerate() {
    //     for penalty in from_key_penalty_item.key_penalty_map.iter().enumerate() {
    //         penalty.
    //     }
    // }

    // for (relation_position, penalty) in relation.relation_map.iter().enumerate() {
    //     pos_relation[position].relation_map[relation_position] += penalty;
    // }

    // matching_ngrams.append(&mut ngram_list.map.iter().filter(|(key, frequency)|{
    //     false
    // }).collect::<Vec<(String, usize)>>())

    //let combinations = good_positions.iter().combinations(12);//.unique();
    // let mut count = 0;
    // for combination in combinations {
    //     count += 1;
    //     if count % 100000000 == 0 {
    //         println!("100millions: {}", count);
    //     }
    // }

    //println!("{:?}", &position_penalties[1].bad_score_total);
    // print_penalty(&position_penalties[0]);
    // print_penalty(&position_penalties[1000]);
    // print_penalty(&position_penalties[10000]);
    // print_penalty(&position_penalties[20000]);
    // print_penalty(&position_penalties[30000]);
    // print_penalty(&position_penalties[40000]);

    // println!("penalty 1: {}", position_penalties[0]);
    // println!("penalty 1: {}", position_penalties[1]);
    //println!("starting comb");
    //let combinations = perms.combinations(12);//.unique();
    // let mut count = 0;
    // for combination in perms.combinations(12) {
    //     count += 1;
    //     if count % 100000000 == 0 {
    //         println!("100millions: {}", count);
    //     }
    // }
    //println!("combinations: {}", perms.combinations(12).count());
    //println!("combinations: {}", combinations.count());
}

pub fn get_penalty_list(
    list: Vec<(usize, Vec<(String, f64)>)>,
    position_remove_list: Vec<usize>,
    letter_remove_list: Vec<String>
) -> Vec<(usize, Vec<(String, f64)>)> {
    //println!("list {:?}", list);
    println!("position_remove_list {:?}", position_remove_list);
    println!("letter_remove_list {:?}", letter_remove_list);


    let updated_list = list
        .clone()
        .into_iter()
        .filter(|(position, _)| { !position_remove_list.contains(position) })
        .map(|(position, penalties)| {
            let mut updated_penalties: Vec<(String, f64)> = penalties
                .clone()
                .into_iter()
                .filter(|(letter, _)| { !letter_remove_list.contains(&letter.clone()) })
                .collect::<Vec<(String, f64)>>();
            (position, updated_penalties)
        })
        .collect();
    
    return updated_list;
}

pub fn print_relation(relation: DisplayPosRelation) {
    print!(
        "{}",
        format!(
            "\n{}\n{}\n{}\n{}\n{}\n{}\n",
            format!(
                "{:<8.2} {:<8.2} {:<8.2} {:<8.2} | {:<8.2} {:<8.2} {:<8.2} {:<8.2}",
                "",
                relation.relation_map[0],
                relation.relation_map[1],
                relation.relation_map[2],
                relation.relation_map[3],
                relation.relation_map[4],
                relation.relation_map[5],
                ""
            ),
            format!(
                "{:<8.2} {:<8.2} {:<8.2} {:<8.2} | {:<8.2} {:<8.2} {:<8.2} {:<8.2}",
                "",
                relation.relation_map[6],
                relation.relation_map[7],
                relation.relation_map[8],
                relation.relation_map[9],
                relation.relation_map[10],
                relation.relation_map[11],
                ""
            ),
            format!(
                "{:<8.2} {:<8.2} {:<8.2} {:<8.2} | {:<8.2} {:<8.2} {:<8.2} {:<8.2}",
                relation.relation_map[12],
                relation.relation_map[13],
                relation.relation_map[14],
                relation.relation_map[15],
                relation.relation_map[16],
                relation.relation_map[17],
                relation.relation_map[18],
                relation.relation_map[19]
            ),
            format!(
                "{:<8.2} {:<8.2} {:<8.2} {:<8.2} | {:<8.2} {:<8.2} {:<8.2} {:<8.2}",
                relation.relation_map[20],
                relation.relation_map[21],
                relation.relation_map[22],
                relation.relation_map[23],
                relation.relation_map[24],
                relation.relation_map[25],
                relation.relation_map[26],
                relation.relation_map[27]
            ),
            format!(
                "{:<8.2} {:<8.2} {:<8.2} {:<8.2} | {:<8.2} {:<8.2} {:<8.2} {:<8.2}",
                "",
                "",
                "",
                relation.relation_map[28],
                relation.relation_map[29],
                "",
                "",
                ""
            ),
            format!(
                "{:<8.2} {:<8.2} {:<8.2} {:<8.2} | {:<8.2} {:<8.2} {:<8.2} {:<8.2}",
                "",
                relation.relation_map[30],
                relation.relation_map[31],
                relation.relation_map[32],
                relation.relation_map[33],
                relation.relation_map[34],
                relation.relation_map[35],
                ""
            )
        )
    );

    println!("-----------------------------------------------------------------");

    print!(
        "{}",
        format!(
            "\n{}\n{}\n{}\n{}\n{}\n{}\n",
            format!(
                "{:<20.30} {:<20.30} {:<20.30} {:<20.30} | {:<20.30} {:<20.30} {:<20.30} {:<20.30}",
                "",
                relation.penalty_types[0].iter().join(","),
                relation.penalty_types[1].iter().join(","),
                relation.penalty_types[2].iter().join(","),
                relation.penalty_types[3].iter().join(","),
                relation.penalty_types[4].iter().join(","),
                relation.penalty_types[5].iter().join(","),
                ""
            ),
            format!(
                "{:<20.30} {:<20.30} {:<20.30} {:<20.30} | {:<20.30} {:<20.30} {:<20.30} {:<20.30}",
                "",
                relation.penalty_types[6].iter().join(","),
                relation.penalty_types[7].iter().join(","),
                relation.penalty_types[8].iter().join(","),
                relation.penalty_types[9].iter().join(","),
                relation.penalty_types[10].iter().join(","),
                relation.penalty_types[11].iter().join(","),
                ""
            ),
            format!(
                "{:<20.30} {:<20.30} {:<20.30} {:<20.30} | {:<20.30} {:<20.30} {:<20.30} {:<20.30}",
                relation.penalty_types[12].iter().join(","),
                relation.penalty_types[13].iter().join(","),
                relation.penalty_types[14].iter().join(","),
                relation.penalty_types[15].iter().join(","),
                relation.penalty_types[16].iter().join(","),
                relation.penalty_types[17].iter().join(","),
                relation.penalty_types[18].iter().join(","),
                relation.penalty_types[19].iter().join(",")
            ),
            format!(
                "{:<20.30} {:<20.30} {:<20.30} {:<20.30} | {:<20.30} {:<20.30} {:<20.30} {:<20.30}",
                relation.penalty_types[20].iter().join(","),
                relation.penalty_types[21].iter().join(","),
                relation.penalty_types[22].iter().join(","),
                relation.penalty_types[23].iter().join(","),
                relation.penalty_types[24].iter().join(","),
                relation.penalty_types[25].iter().join(","),
                relation.penalty_types[26].iter().join(","),
                relation.penalty_types[27].iter().join(",")
            ),
            format!(
                "{:<20.30} {:<20.30} {:<20.30} {:<20.30} | {:<20.30} {:<20.30} {:<20.30} {:<20.30}",
                "",
                "",
                "",
                relation.penalty_types[28].iter().join(","),
                relation.penalty_types[29].iter().join(","),
                "",
                "",
                ""
            ),
            format!(
                "{:<20.30} {:<20.30} {:<20.30} {:<20.30} | {:<20.30} {:<20.30} {:<20.30} {:<20.30}",
                "",
                relation.penalty_types[30].iter().join(","),
                relation.penalty_types[31].iter().join(","),
                relation.penalty_types[32].iter().join(","),
                relation.penalty_types[33].iter().join(","),
                relation.penalty_types[34].iter().join(","),
                relation.penalty_types[35].iter().join(","),
                ""
            )
        )
    );
}

pub fn normalize_count(count: usize, len: usize) -> f64 {
    return (count as f64) / (len as f64);
}

pub fn normalize_penalty(penalty: f64, min: f64, range: f64) -> f64 {
    return (penalty - min) / range;
}

pub fn print_penalty<'a>(item: &evaluator_penalty::Penalty<{ layout::NUM_OF_KEYS }>) {
    let bad_score_total = item.bad_score_total;
    let good_score_total = item.good_score_total;
    let total = good_score_total + bad_score_total;
    let len = item.len;
    let penalties = &item.penalties;
    let penalty = &item;
    let fingers = &penalty.fingers;
    let hands = &penalty.hands;
    let show_all = false;
    let positions = item.pos;
    let position_penalties = item.pos_pen;
    let mut position_working = [0; NUM_OF_KEYS];
    position_penalties
        .into_iter()
        .enumerate()
        .for_each(|(i, penalty)| {
            println!("penalty {i} : {penalty}");
            position_working[i] = (penalty * 100.0) as i128;
        });
    position_working.sort_unstable();

    let max_position = position_working[NUM_OF_KEYS - 1];
    let min_position_penalty = (position_working[0] as f64) / 100.0;
    let range_position_penalty = (max_position as f64) / 100.0 - min_position_penalty;

    println!("position_penalties {:?}", &position_penalties[0]);
    println!("min_position_penalty {:?}", &min_position_penalty);
    println!("range_position_penalty {:?}", &range_position_penalty);

    print!(
        "{}{}{}{}{}{}{}{}{}{}{}",
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n",
            format!(
                "{:<5.4} {:<5.4} {:<5.4} {:<5.4} | {:<5.4} {:<5.4} {:<5.4} {:<5.4}",
                "",
                normalize_penalty(
                    position_penalties[0],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[1],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[2],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[3],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[4],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[5],
                    min_position_penalty,
                    range_position_penalty
                ),
                ""
            ),
            format!(
                "{:<5.4} {:<5.4} {:<5.4} {:<5.4} | {:<5.4} {:<5.4} {:<5.4} {:<5.4}",
                "",
                normalize_penalty(
                    position_penalties[6],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[7],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[8],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[9],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[10],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[11],
                    min_position_penalty,
                    range_position_penalty
                ),
                ""
            ),
            format!(
                "{:<5.4} {:<5.4} {:<5.4} {:<5.4} | {:<5.4} {:<5.4} {:<5.4} {:<5.4}",
                normalize_penalty(
                    position_penalties[12],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[13],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[14],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[15],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[16],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[17],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[18],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[19],
                    min_position_penalty,
                    range_position_penalty
                )
            ),
            format!(
                "{:<5.4} {:<5.4} {:<5.4} {:<5.4} | {:<5.4} {:<5.4} {:<5.4} {:<5.4}",
                normalize_penalty(
                    position_penalties[20],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[21],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[22],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[23],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[24],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[25],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[26],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[27],
                    min_position_penalty,
                    range_position_penalty
                )
            ),
            format!(
                "{:<5.4} {:<5.4} {:<5.4} {:<5.4} | {:<5.4} {:<5.4} {:<5.4} {:<5.4}",
                "",
                "",
                "",
                normalize_penalty(
                    position_penalties[28],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[29],
                    min_position_penalty,
                    range_position_penalty
                ),
                "",
                "",
                ""
            ),
            format!(
                "{:<5.4} {:<5.4} {:<5.4} {:<5.4} | {:<5.4} {:<5.4} {:<5.4} {:<5.4}",
                "",
                normalize_penalty(
                    position_penalties[30],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[31],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[32],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[33],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[34],
                    min_position_penalty,
                    range_position_penalty
                ),
                normalize_penalty(
                    position_penalties[35],
                    min_position_penalty,
                    range_position_penalty
                ),
                ""
            )
        ),
        format!(
            "\n{}\n{}\n{}\n{}\n{}\n{}\n",
            format!(
                "{:<5.4} {:<5.4} {:<5.4} {:<5.4} | {:<5.4} {:<5.4} {:<5.4} {:<5.4}",
                "",
                normalize_count(positions[0], len),
                normalize_count(positions[1], len),
                normalize_count(positions[2], len),
                normalize_count(positions[3], len),
                normalize_count(positions[4], len),
                normalize_count(positions[5], len),
                ""
            ),
            format!(
                "{:<5.4} {:<5.4} {:<5.4} {:<5.4} | {:<5.4} {:<5.4} {:<5.4} {:<5.4}",
                "",
                normalize_count(positions[6], len),
                normalize_count(positions[7], len),
                normalize_count(positions[8], len),
                normalize_count(positions[9], len),
                normalize_count(positions[10], len),
                normalize_count(positions[11], len),
                ""
            ),
            format!(
                "{:<5.4} {:<5.4} {:<5.4} {:<5.4} | {:<5.4} {:<5.4} {:<5.4} {:<5.4}",
                normalize_count(positions[12], len),
                normalize_count(positions[13], len),
                normalize_count(positions[14], len),
                normalize_count(positions[15], len),
                normalize_count(positions[16], len),
                normalize_count(positions[17], len),
                normalize_count(positions[18], len),
                normalize_count(positions[19], len)
            ),
            format!(
                "{:<5.4} {:<5.4} {:<5.4} {:<5.4} | {:<5.4} {:<5.4} {:<5.4} {:<5.4}",
                normalize_count(positions[20], len),
                normalize_count(positions[21], len),
                normalize_count(positions[22], len),
                normalize_count(positions[23], len),
                normalize_count(positions[24], len),
                normalize_count(positions[25], len),
                normalize_count(positions[26], len),
                normalize_count(positions[27], len)
            ),
            format!(
                "{:<5.4} {:<5.4} {:<5.4} {:<5.4} | {:<5.4} {:<5.4} {:<5.4} {:<5.4}",
                "",
                "",
                "",
                normalize_count(positions[28], len),
                normalize_count(positions[29], len),
                "",
                "",
                ""
            ),
            format!(
                "{:<5.4} {:<5.4} {:<5.4} {:<5.4} | {:<5.4} {:<5.4} {:<5.4} {:<5.4}",
                "",
                normalize_count(positions[30], len),
                normalize_count(positions[31], len),
                normalize_count(positions[32], len),
                normalize_count(positions[33], len),
                normalize_count(positions[34], len),
                normalize_count(positions[35], len),
                ""
            )
        ),
        format!(
            "hands: {:<5.4} | {:<5.4}\n",
            normalize_penalty(hands[0] as f64, 0.0, len as f64),
            normalize_penalty(hands[1] as f64, 0.0, len as f64)
        ),
        format!(
            "bad score total: {0:<10.2}; good score total: {1:<10.2}; bad score scaled: {2:<10.4}; total: {3:<10.4}\n",
            bad_score_total,
            good_score_total,
            bad_score_total / (len as f64),
            total
        ),
        //format!("base {}\n",penalties[0]),
        format!(
            "\n{:<30} | {:^7} | {:^7} | {:^8} | {:<10}\n",
            "Name",
            "% times",
            "Avg",
            "% Total",
            "Total"
        ),
        "----------------------------------------------------------------------\n",
        penalties
            .into_iter()
            .map(|penalty| {
                if penalty.show || show_all {
                    format!(
                        "{:<30} | {:<7.3} | {:<7.4} | {:<8.4} | {:<10.4}\n",
                        penalty.name,
                        penalty.times,
                        penalty.total / (len as f64),
                        (100.0 * penalty.total) / bad_score_total,
                        penalty.total
                    )
                } else {
                    "".to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(""),
        "----------------------------------------------------------------------\n",
        format!(
            "\n{:^5.1} {:^5.1} {:^5.1} {:^5.1} | {:^5.1} {:^5.1} {:^5.1} {:^5.1}\n",
            ((fingers[0] as f64) * 100.0) / (len as f64),
            ((fingers[1] as f64) * 100.0) / (len as f64),
            ((fingers[2] as f64) * 100.0) / (len as f64),
            ((fingers[3] as f64) * 100.0) / (len as f64),
            ((fingers[7] as f64) * 100.0) / (len as f64),
            ((fingers[6] as f64) * 100.0) / (len as f64),
            ((fingers[5] as f64) * 100.0) / (len as f64),
            ((fingers[4] as f64) * 100.0) / (len as f64)
        ),

        format!(
            "{:^5.1}| {:^5.1}\n",
            ((penalty.hands[0] as f64) * 100.0) / (len as f64),
            ((penalty.hands[1] as f64) * 100.0) / (len as f64)
        ),
        "##########################################################################\n"
    );
}
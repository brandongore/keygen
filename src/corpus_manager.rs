use crate::{file_manager::*, evaluator_penalty, layout, evaluator_penalty_small};
use chrono::{ DateTime, Utc };
use itertools::Itertools;
use serde::{ de::{ MapAccess, SeqAccess }, Deserialize, Serialize };
use std::{ collections::HashMap, fmt, hash::Hash };

use serde::de::{ Deserializer, Error, Visitor };
use serde_json::{ Map, Value };

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NgramList {
    pub map: HashMap<String, usize>,
    pub gram: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NgramListRelation {
    pub ngram: String,
    pub frequency: usize,
    pub after_map: NgramList,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NgramListRelationMapping {
    pub ngrams: HashMap<String, NgramListRelation>,
    pub gram: usize,
}

pub struct SwapCharList {
    pub map: HashMap<char, char>,
}

impl NgramList {
    fn new() -> NgramList {
        let mut ngram_list = HashMap::new();
        NgramList {
            map: ngram_list,
            gram: 0,
        }
    }

    fn add(&mut self, elem: String) {
        let entry = self.map.entry(elem).or_insert(0);
        *entry += 1;
    }

    fn add_sized(&mut self, elem: String, count: usize) {
        let entry = self.map.entry(elem).or_insert(0);
        *entry += count;
    }
}

impl FromIterator<NgramList> for NgramList {
    fn from_iter<I: IntoIterator<Item = NgramList>>(iter: I) -> Self {
        let mut ngram_list = NgramList::new();
        for item in iter {
            for (string, count) in item.map {
                ngram_list.add_sized(string, count);
            }
        }
        ngram_list
    }
}

impl NgramListRelation {
    fn new(ngram: String, frequency: usize) -> NgramListRelation {
        let mut ngram_list = NgramList::new();
        NgramListRelation {
            ngram: ngram,
            frequency: frequency,
            after_map: ngram_list,
        }
    }
}

impl NgramListRelationMapping {
    fn new() -> NgramListRelationMapping {
        let mut ngram_list: HashMap<String, NgramListRelation> = HashMap::new();

        NgramListRelationMapping {
            ngrams: ngram_list,
            gram: 0,
        }
    }

    fn add(&mut self, ngram: String) {
        let mut padded_ngram = ngram.clone();
        let length: usize = 3;
        if padded_ngram.len() < 3 {
            //this could swap counts more to space character
            let slice = format!("{:<length$}", &ngram.to_lowercase());
            padded_ngram = slice;
        }

        let mut ngram_tuples = padded_ngram.chars().tuple_windows::<(_, _, _)>();

        let mut ngram_list: Vec<String> = Vec::new();

        //(t,h,e), (h,e,r), (e,r,e), (r,e,' '), (e,'',i),('',i,s), (i,s,'')
        //"there is"

        for item in ngram_tuples.clone() {
            let ngram = format!("{}{}{}", item.0, item.1, item.2);
            //println!("ngram {} ({}{}{}) {}{}{}",ngram, item.0, item.1, item.2, item.0 as i32, item.1 as i32, item.2 as i32);
            if (item.0 as i32) != 32 || (item.1 as i32) != 32 || (item.2 as i32) != 32 {
                if
                    ngram
                        .chars()
                        .all(
                            |c|
                                ((c as i32) >= 65 && (c as i32) <= 90) ||
                                ((c as i32) >= 97 && (c as i32) <= 122) ||
                                (c as i32) == 32
                        )
                {
                    ngram_list.push(ngram.to_lowercase());
                } else {
                    ngram_list.push(ngram);
                }
            }
            // println!("chars {:?}", ngram.chars().into_iter().map(|chars| chars as i32).collect::<Vec<i32>>());
        }

        if ngram_list.len() > 0 {
            let last_ngram = ngram_list
                .last()
                .unwrap()
                .chars()
                .collect_tuple::<(_, _, _)>()
                .unwrap();

            if
                !ngram_list
                    .last()
                    .unwrap()
                    .chars()
                    .all(|c| (c as i32) != 32)
            {
                ngram_list.push(format!("{}{}{}", last_ngram.1, last_ngram.2, ' '));
            }

            for (index, ngram) in ngram_list.clone().into_iter().enumerate() {
                if
                    ngram
                        .chars()
                        .all(
                            |c|
                                ((c as i32) >= 65 && (c as i32) <= 90) ||
                                ((c as i32) >= 97 && (c as i32) <= 122) ||
                                (c as i32) == 32
                        )
                {
                    let entry = self.ngrams.entry(ngram.clone()).or_insert(NgramListRelation {
                        ngram: ngram.clone(),
                        frequency: 0,
                        after_map: NgramList::new(),
                    });
                    entry.frequency += 1;

                    let next_index = index + 2; // plus one means its follow, could be an option
                    if next_index < ngram_list.clone().len() - 1 {
                        let next_ngram = ngram_list[next_index].clone();

                        if
                            next_ngram
                                .chars()
                                .all(
                                    |c|
                                        ((c as i32) >= 65 && (c as i32) <= 90) ||
                                        ((c as i32) >= 97 && (c as i32) <= 122) ||
                                        (c as i32) == 32
                                )
                        {
                            let after_entry = entry.after_map.map.entry(next_ngram).or_insert(0);
                            *after_entry += 1;
                        }
                    }
                }
            }
        }

        // for (key, val) in ngram_list.map {
        //     let after_map_entry = entry.after_map.map.entry(key).or_insert(0);
        //     *after_map_entry += val;

        //     // if key.len() >= length {
        //     //     for j in 0..key.chars().count() - length {
        //     //         let slice = &key[j..j + length];
        //     //         if slice.chars().all(|c| (c as i32) <= 128) {
        //     //             let entry = ngram_list.entry(slice.to_string()).or_insert(0);
        //     //             *entry += val;
        //     //         }
        //     //     }
        //     // } else {
        //     //     count_missed += 1;
        //     // }
        // }

        //     let after_map_entry = entry.after_map.map.entry(key).or_insert(NgramList {
        //         ngram: ngram,
        //         frequency: frequency,
        //         after_map: ngram_list,

        // }
    }

    fn remove_non_alpha(&mut self) {
        let alpha: Vec<String> = self.ngrams
            .clone()
            .into_iter()
            .filter(|item| {
                item.0
                    .chars()
                    .any(
                        |c|
                            !(
                                ((c as i32) >= 65 && (c as i32) <= 90) ||
                                ((c as i32) >= 97 && (c as i32) <= 122) ||
                                (c as i32) == 32
                            )
                    )
            })
            .map(|(key, _)| key)
            .collect::<Vec<String>>();

        alpha.iter().for_each(|ngram| {
            self.ngrams.remove(ngram);
        });
    }
}

pub fn prepare_ngram_list(
    corpus: &String,
    swap_char_list: SwapCharList,
    split_char: &String,
    length: usize
) -> NgramList {
    let mut ngram_list: HashMap<String, usize> = HashMap::new();

    let mut processed_corpus: String = String::new();

    //convert windows newline to just newline for better ngrams
    let corpus = corpus.replace("\r\n", "\n");

    //swap any single character in corpus with another character if its exists in the swap list
    for x in corpus.chars() {
        match x {
            x if swap_char_list.map.contains_key(&x) => {
                processed_corpus.push(*swap_char_list.map.get(&x).unwrap_or(&(0 as char)));
            }
            x => processed_corpus.push(x),
        }
    }

    if !split_char.to_string().is_empty() {
        let lines: Vec<String> = processed_corpus
            .lines()
            .map(|x| String::from(x))
            .collect();
        for i in 0..lines.len() {
            let line = &lines[i];
            let split: Vec<&str> = line.split(split_char).collect();

            for word in split {
                if word.chars().all(|c| (c as i32) <= 128) {
                    let entry = ngram_list.entry(word.to_string()).or_insert(0);
                    *entry += 1;
                }
            }
        }
    } else {
        for i in 0..processed_corpus.chars().count() - length {
            let slice = &processed_corpus[i..i + length];
            if slice.chars().all(|c| (c as i32) <= 128) {
                let entry = ngram_list.entry(slice.to_string()).or_insert(0);
                *entry += 1;
            }
        }
    }

    NgramList {
        map: ngram_list,
        gram: length,
    }
}

pub fn generate_ngram_list(corpus: Vec<String>, length: usize) -> NgramList {
    let mut ngram_list: HashMap<String, usize> = HashMap::new();
    //missing hundreds of items, eg iff 475 output 561 in base

    for item in corpus {
        if item.chars().all(|c| (c as i32) <= 128) {
            if item.chars().count() < length {
                let slice = format!("{}{:<indent$}", &item.to_lowercase(), indent = length);
                if
                    slice
                        .chars()
                        .all(
                            |c|
                                ((c as i32) >= 65 && (c as i32) <= 90) ||
                                ((c as i32) >= 97 && (c as i32) <= 122) ||
                                (c as i32) == 32
                        )
                {
                    let entry = ngram_list.entry(slice.to_string()).or_insert(0);
                    *entry += 1;
                }
            } else {
                for i in 0..item.chars().count() - length {
                    let slice = &item.to_lowercase()[i..i + length];
                    if
                        slice
                            .chars()
                            .all(
                                |c|
                                    ((c as i32) >= 65 && (c as i32) <= 90) ||
                                    ((c as i32) >= 97 && (c as i32) <= 122) ||
                                    (c as i32) == 32
                            )
                    {
                        let entry = ngram_list.entry(slice.to_string()).or_insert(0);
                        *entry += 1;
                    }
                }
            }
        }
    }

    NgramList {
        map: ngram_list,
        gram: length,
    }
}

pub fn generate_ngram_relation_list(
    corpus: Vec<String>,
    length: usize
) -> NgramListRelationMapping {
    let mut ngram_relation_mapping: NgramListRelationMapping = NgramListRelationMapping::new();
    //missing hundreds of items, eg iff 475 output 561 in base

    for item in corpus {
        //if item.chars().all(|c| (c as i32) <= 128) {
        if item.chars().count() < length {
            let slice = format!("{}{:<indent$}", &item, indent = length);
            if
                slice
                    .chars()
                    .all(
                        |c|
                            ((c as i32) >= 65 && (c as i32) <= 90) ||
                            ((c as i32) >= 97 && (c as i32) <= 122) ||
                            (c as i32) == 32
                    )
            {
                //println!("slice {}", slice);
                ngram_relation_mapping.add(slice.to_lowercase());
                // let entry = ngram_relation_mapping.ngrams
                //     .entry(slice.to_string())
                //     .or_insert(NgramListRelation {
                //         ngram: slice.to_string(),
                //         frequency: 1,
                //         after_map: NgramList::new(),
                //     });
            }
            // else{
            //     ngram_relation_mapping.add(slice);
            // }
        } else {
            let slice = item.to_string();

            // let character_penalty_groups: Vec<&str> = slice.
            // .par_iter()
            // .map(|penalty_map| {
            //     let mut character_penalties: Vec<(char, u128)> = penalty_map
            //         .iter_mut()
            //         .map(|item| (*item.key(), (item.value() * 100.0) as u128))
            //         .collect::<Vec<(char, u128)>>();

            //     character_penalties.sort_by(|first, second| first.1.cmp(&second.1));

            //     save_penalty.insert(*penalty_map.key(), character_penalties.clone());

            //     if character_penalties.len() >= 3 {
            //         character_penalties
            //         .drain(0..3)
            //         .map(|(character, _)| (character, *penalty_map.key()))
            //         .collect::<Vec<(char, usize)>>()
            //     }
            //     else {
            //         character_penalties
            //         .into_iter()
            //         .map(|(character, _)| (character, *penalty_map.key()))
            //         .collect::<Vec<(char, usize)>>()
            //     }

            // })
            // .collect();

            ngram_relation_mapping.add(slice);

            // for i in 0..slice.chars().count() - length {

            //     let ngram = &item.to_lowercase()[i..i + length];

            //     if
            //     ngram
            //         .chars()
            //         .all(
            //             |c|
            //                 ((c as i32) >= 65 && (c as i32) <= 90) ||
            //                 ((c as i32) >= 97 && (c as i32) <= 122) ||
            //                 (c as i32) == 32
            //         )
            //     {
            //         println!("slice {}", ngram);
            //         ngram_relation_mapping.add(ngram.to_lowercase());
            //     }
            // }

            // else{
            //     ngram_relation_mapping.add(slice);
            // }
            // let mut test = item.chars().tuple_windows::<(_, _, _)>();
            // let ngram = test.next().unwrap();
            // let mut start= format!("{}{}{}", ngram.0,ngram.0,ngram.0);

            // for relation in test.enumerate() {

            // }

            // for i in 0..item.chars().count() - length {
            //     let slice = &item.to_lowercase()[i..i + length];
            //     if
            //         slice
            //             .chars()
            //             .all(
            //                 |c|
            //                     ((c as i32) >= 65 && (c as i32) <= 90) ||
            //                     ((c as i32) >= 97 && (c as i32) <= 122) ||
            //                     (c as i32) == 32
            //             )
            //     {
            //         let entry = ngram_list.entry(slice.to_string()).or_insert(0);
            //         *entry += 1;
            //     }
            // }
        }
        //}
    }

    //ngram_relation_mapping.remove_non_alpha();

    return ngram_relation_mapping;
}

//THE OG GENERATE WITHOUT CHANGES FOR CODING CHARACTERS
// pub fn generate_ngram_relation_list(
//     corpus: Vec<String>,
//     length: usize
// ) -> NgramListRelationMapping {
//     let mut ngram_relation_mapping: NgramListRelationMapping = NgramListRelationMapping::new();
//     //missing hundreds of items, eg iff 475 output 561 in base

//     for item in corpus {
//         if item.chars().all(|c| (c as i32) <= 128) {
//             if item.chars().count() < length {
//                 let slice = format!("{}{:<indent$}", &item.to_lowercase(), indent = length);
//                 if
//                     slice
//                         .chars()
//                         .all(
//                             |c|
//                                 ((c as i32) >= 65 && (c as i32) <= 90) ||
//                                 ((c as i32) >= 97 && (c as i32) <= 122) ||
//                                 (c as i32) == 32
//                         )
//                 {
//                     ngram_relation_mapping.add(slice);
//                     // let entry = ngram_relation_mapping.ngrams
//                     //     .entry(slice.to_string())
//                     //     .or_insert(NgramListRelation {
//                     //         ngram: slice.to_string(),
//                     //         frequency: 1,
//                     //         after_map: NgramList::new(),
//                     //     });
//                 }
//             } else {
//                 let slice = item.to_lowercase().to_string();
//                 if
//                     slice
//                         .chars()
//                         .all(
//                             |c|
//                                 ((c as i32) >= 65 && (c as i32) <= 90) ||
//                                 ((c as i32) >= 97 && (c as i32) <= 122) ||
//                                 (c as i32) == 32
//                         )
//                 {
//                     ngram_relation_mapping.add(slice);
//                 }
//                 // let mut test = item.chars().tuple_windows::<(_, _, _)>();
//                 // let ngram = test.next().unwrap();
//                 // let mut start= format!("{}{}{}", ngram.0,ngram.0,ngram.0);

//                 // for relation in test.enumerate() {

//                 // }

//                 // for i in 0..item.chars().count() - length {
//                 //     let slice = &item.to_lowercase()[i..i + length];
//                 //     if
//                 //         slice
//                 //             .chars()
//                 //             .all(
//                 //                 |c|
//                 //                     ((c as i32) >= 65 && (c as i32) <= 90) ||
//                 //                     ((c as i32) >= 97 && (c as i32) <= 122) ||
//                 //                     (c as i32) == 32
//                 //             )
//                 //     {
//                 //         let entry = ngram_list.entry(slice.to_string()).or_insert(0);
//                 //         *entry += 1;
//                 //     }
//                 // }
//             }
//         }
//     }

//     return ngram_relation_mapping;
// }

pub fn save_ngram_list(filename: &String, ngram_list: NgramList) {
    let folder = String::from("\\processed\\");
    save_file::<NgramList>(String::from(filename), String::from(folder), &ngram_list);
}

pub fn save_ngram_list_relation_mapping(
    filename: &String,
    ngram_list_relation_mapping: NgramListRelationMapping
) {
    let folder = String::from("\\processed\\");
    save_file::<NgramListRelationMapping>(
        String::from(filename),
        String::from(folder),
        &ngram_list_relation_mapping
    );
}

pub fn save_vec_array_list<T: serde::Serialize>(
    filename: &String,
    vec_array: Vec<T>
) {
    let folder = String::from("\\processed\\");
    save_small_file::<Vec<T>>(
        String::from(filename),
        String::from(folder),
        &vec_array
    );
    // save_file::<Vec<evaluator_penalty_small::Penalty<{ layout::NUM_OF_KEYS }>>>(
    //     String::from(filename),
    //     String::from(folder),
    //     &position_penalties
    // );
}

pub fn save_generic_list<T: serde::Serialize>(
    filename: &String,
    list: T
) {
    let folder = String::from("\\processed\\");
    save_small_file::<T>(
        String::from(filename),
        String::from(folder),
        &list
    );
    // save_file::<Vec<evaluator_penalty_small::Penalty<{ layout::NUM_OF_KEYS }>>>(
    //     String::from(filename),
    //     String::from(folder),
    //     &position_penalties
    // );
}

pub fn save_position_penalty_list(
    filename: &String,
    position_penalties: Vec<evaluator_penalty_small::Penalty<{ layout::NUM_OF_KEYS }>>
) {
    let folder = String::from("\\processed\\");
    save_small_file::<Vec<evaluator_penalty_small::Penalty<{ layout::NUM_OF_KEYS }>>>(
        String::from(filename),
        String::from(folder),
        &position_penalties
    );
    // save_file::<Vec<evaluator_penalty_small::Penalty<{ layout::NUM_OF_KEYS }>>>(
    //     String::from(filename),
    //     String::from(folder),
    //     &position_penalties
    // );
}

pub fn save_position_penalty_hashmap(
    filename: &String,
    position_penalties_map: HashMap<String, evaluator_penalty_small::Penalty<{ layout::NUM_OF_KEYS }>>
) {
    let folder = String::from("\\processed\\");
    save_small_file::<HashMap<String, evaluator_penalty_small::Penalty<{ layout::NUM_OF_KEYS }>>>(
        String::from(filename),
        String::from(folder),
        &position_penalties_map
    );
}

pub fn save_string_list(filename: &String, string_list: Vec<String>) {
    let folder = String::from("\\processed\\");
    save_file::<Vec<String>>(String::from(filename), String::from(folder), &string_list);
}

pub fn read_ngram_list(filepath: &String) -> NgramList {
    return read_json::<NgramList>(
        filepath.to_string(),
        String::from("\\processed\\")
    ).unwrap_or_else(|_| panic!("Could not read corpus"));
}

pub fn read_ngram_relation_mapping(filepath: &String) -> NgramListRelationMapping {
    return read_json::<NgramListRelationMapping>(
        filepath.to_string(),
        String::from("\\processed\\")
    ).unwrap_or_else(|_| panic!("Could not read corpus"));
}

pub fn read_position_penalty_list(filepath: &String) -> Vec<evaluator_penalty_small::Penalty<{ layout::NUM_OF_KEYS }>> {
    return read_json::<Vec<evaluator_penalty_small::Penalty<{ layout::NUM_OF_KEYS }>>>(
        filepath.to_string(),
        String::from("\\processed\\")
    ).unwrap_or_else(|_| panic!("Could not read corpus"));
}

pub fn read_position_penalty_hashmap(filepath: &String) -> HashMap<String, evaluator_penalty_small::Penalty<{ layout::NUM_OF_KEYS }>> {
    return read_json::<HashMap<String, evaluator_penalty_small::Penalty<{ layout::NUM_OF_KEYS }>>>(
        filepath.to_string(),
        String::from("\\processed\\")
    ).unwrap_or_else(|_| panic!("Could not read corpus"));
}

pub fn read_vec_array_list<T: for<'a> serde::Deserialize<'a>>(filepath: &String) -> Vec<T> {
    return read_json::<Vec<T>>(
        filepath.to_string(),
        String::from("\\processed\\")
    ).unwrap_or_else(|_| panic!("Could not read corpus"));
}

pub fn read_generic_list<T: for<'a> serde::Deserialize<'a>>(filepath: &String) -> T {
    return read_json::<T>(
        filepath.to_string(),
        String::from("\\processed\\")
    ).unwrap_or_else(|_| panic!("Could not read corpus"));
}

pub fn read_json_array_list(filepath: &String) -> Vec<String> {
    return read_json::<Vec<String>>(
        filepath.to_string(),
        String::from("\\processed\\")
    ).unwrap_or_else(|_| panic!("Could not read list"));
}

pub fn merge_ngram_lists(filepaths: Vec<String>) -> NgramList {
    let mut ngram_lists: Vec<NgramList> = Vec::new();
    for filepath in filepaths {
        let ngram_list = read_json::<NgramList>(
            filepath.to_string(),
            String::from("\\processed\\")
        ).unwrap_or_else(|_| panic!("Could not read corpus"));
        ngram_lists.push(ngram_list);
    }

    let flattened_list = ngram_lists
        .iter()
        .map(|x| x.clone())
        .collect::<NgramList>();

    return flattened_list;
}

pub fn parse_ngram_list(
    corpus: &String,
    split_char: &String,
    calc_gram: bool,
    length: usize
) -> NgramList {
    let mut ngram_list: HashMap<String, usize> = HashMap::new();

    //convert windows newline to just newline for better ngrams
    let corpus = corpus.replace("\r\n", "\n");

    let lines: Vec<String> = corpus
        .lines()
        .map(|x| String::from(x))
        .collect();

    println!("lines 1 {}", lines[0]);

    let mut gram_type = length;
    if !split_char.to_string().is_empty() {
        for i in 0..lines.len() {
            let line = &lines[i];
            let split: (&str, &str) = line.split(split_char).collect_tuple().unwrap();

            if split.0.chars().all(|c| (c as i32) <= 128) {
                let entry = ngram_list.entry(split.0.to_string()).or_insert(0);
                *entry += split.1.parse::<usize>().unwrap_or_default();
            }
        }

        if calc_gram {
            let first = lines.first().unwrap();
            let first_split: (&str, &str) = first.split("\t").collect_tuple().unwrap();
            gram_type = first_split.0.len();
        }
    }

    NgramList {
        map: ngram_list,
        gram: gram_type,
    }
}

pub fn batch_parse_ngram_list(
    corpora: Vec<String>,
    split_char: &String,
    length: usize
) -> NgramList {
    let mut ngram_list: HashMap<String, usize> = HashMap::new();

    //convert windows newline to just newline for better ngrams
    let processed_corpora: Vec<String> = corpora
        .iter()
        .map(|x| x.replace("\r\n", "\n"))
        .collect();

    for corpus in processed_corpora {
        if !split_char.to_string().is_empty() {
            let lines: Vec<String> = corpus
                .lines()
                .map(|x| String::from(x))
                .collect();
            for i in 0..lines.len() {
                let line = &lines[i];
                let split: Vec<&str> = line.split(split_char).collect();

                for word in split {
                    if word.chars().all(|c| (c as i32) <= 128) {
                        let entry = ngram_list.entry(word.to_string()).or_insert(0);
                        *entry += 1;
                    }
                }
            }
        } else {
            for i in 0..corpus.chars().count() - length {
                let slice = &corpus[i..i + length];
                if slice.chars().all(|c| (c as i32) <= 128) {
                    let entry = ngram_list.entry(slice.to_string()).or_insert(0);
                    *entry += 1;
                }
            }
        }
    }

    NgramList {
        map: ngram_list,
        gram: 0,
    }
}

pub fn normalize_ngram_list(existing_ngram_list: NgramList, length: usize) -> NgramList {
    //this method may have issues, doesnt seem to count from/to correctly
    let mut ngram_list: HashMap<String, usize> = HashMap::new();

    let mut count_missed = 0;
    for (key, val) in existing_ngram_list.map {
        if key.len() >= length {
            for j in 0..key.chars().count() - length {
                let slice = &key[j..j + length];
                if slice.chars().all(|c| (c as i32) <= 128) {
                    let entry = ngram_list.entry(slice.to_string()).or_insert(0);
                    *entry += val;
                }
            }
        } else {
            count_missed += 1;
        }
    }

    print!("number of items too short for list : {}", count_missed);

    NgramList {
        map: ngram_list,
        gram: length,
    }
}
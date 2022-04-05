use crate::file_manager::*;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{
    de::{MapAccess, SeqAccess},
    Deserialize, Serialize,
};
use std::{collections::HashMap, fmt};

use serde::de::{Deserializer, Error, Visitor};
use serde_json::{Map, Value};

#[derive(Clone, Serialize, Deserialize)]
pub struct NgramList {
    pub map: HashMap<String, usize>,
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

pub fn prepare_ngram_list(
    corpus: &String,
    swap_char_list: SwapCharList,
    split_char: &String,
    length: usize,
) -> NgramList {
    let mut ngram_list: HashMap<String, usize> = HashMap::new();

    let mut processed_corpus: String = String::new();

    //convert windows newline to just newline for better ngrams
    let corpus = corpus.replace("\r\n", "\n");

    //swap any single character in corpus with another character if its exists in the swap list
    for x in corpus.chars() {
        match x {
            x if swap_char_list.map.contains_key(&x) => {
                processed_corpus.push(*swap_char_list.map.get(&x).unwrap_or(&(0 as char)))
            }
            x => processed_corpus.push(x),
        }
    }

    if !split_char.to_string().is_empty() {
        let lines: Vec<String> = processed_corpus.lines().map(|x| String::from(x)).collect();
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

    for item in corpus {
        if item.chars().all(|c| (c as i32) <= 128) {
            for i in 0..item.chars().count() - length {
                let slice = &item.to_lowercase()[i..i + length];
                if slice.chars().all(|c|  ((c as i32) >= 65 && (c as i32) <= 90) || ((c as i32) >= 97 && (c as i32) <= 122)) {
                    let entry = ngram_list.entry(slice.to_string()).or_insert(0);
                    *entry += 1;
                }
            }
        }
    }

    NgramList {
        map: ngram_list,
        gram: length,
    }
}

pub fn save_ngram_list(filename: &String, ngram_list: NgramList) {
    let folder = String::from("\\processed\\");
    save_file::<NgramList>(String::from(filename), String::from(folder), &ngram_list);
}

pub fn read_ngram_list(filepath: &String) -> NgramList {
    return read_json::<NgramList>(filepath.to_string(), String::from("\\processed\\"))
        .unwrap_or_else(|_| panic!("Could not read corpus"));
}

pub fn read_json_array_list(filepath: &String) -> Vec<String> {
    return read_json::<Vec<String>>(filepath.to_string(), String::from("\\processed\\"))
        .unwrap_or_else(|_| panic!("Could not read list"));
}

pub fn merge_ngram_lists(filepaths: Vec<String>) -> NgramList {
    let mut ngram_lists: Vec<NgramList> = Vec::new();
    for filepath in filepaths {
        let ngram_list =
            read_json::<NgramList>(filepath.to_string(), String::from("\\processed\\"))
                .unwrap_or_else(|_| panic!("Could not read corpus"));
        ngram_lists.push(ngram_list);
    }

    let flattened_list = ngram_lists.iter().map(|x| x.clone()).collect::<NgramList>();

    return flattened_list;
}

pub fn parse_ngram_list(
    corpus: &String,
    split_char: &String,
    calc_gram: bool,
    length: usize,
) -> NgramList {
    let mut ngram_list: HashMap<String, usize> = HashMap::new();

    //convert windows newline to just newline for better ngrams
    let corpus = corpus.replace("\r\n", "\n");

    let lines: Vec<String> = corpus.lines().map(|x| String::from(x)).collect();

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
    length: usize,
) -> NgramList {
    let mut ngram_list: HashMap<String, usize> = HashMap::new();

    //convert windows newline to just newline for better ngrams
    let processed_corpora: Vec<String> = corpora.iter().map(|x| x.replace("\r\n", "\n")).collect();

    for corpus in processed_corpora {
        if !split_char.to_string().is_empty() {
            let lines: Vec<String> = corpus.lines().map(|x| String::from(x)).collect();
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

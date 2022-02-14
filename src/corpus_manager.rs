use std::collections::{HashMap};
use serde::{Deserialize, Serialize};
use crate::file_manager::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct NgramList {
    pub map: HashMap<String, usize>,
    pub gram: usize
}

impl NgramList {
    fn new() -> NgramList {
        let mut ngram_list = HashMap::new();
        NgramList { map: ngram_list, gram: 0}
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
    fn from_iter<I: IntoIterator<Item=NgramList>>(iter: I) -> Self
    {
        let mut ngram_list = NgramList::new();
        for item in iter {
            for (string, count) in item.map {
                ngram_list.add_sized(string, count);
            }
        }
        ngram_list
    }
}

pub fn prepare_ngram_list<'a>(string: &'a str, length: usize) -> NgramList {
    let mut ngram_list: HashMap<String, usize> = HashMap::new();

    for i in 0..string.chars().count() - length {
        let slice = &string[i..i + length];
        if slice.chars().all(|c| (c as i32) <= 128) {
            let entry = ngram_list.entry(slice.to_string()).or_insert(0);
            *entry += 1;
        }
    }
    NgramList { map: ngram_list, gram: length }
}

pub fn save_ngram_list(filename: &String, ngram_list: NgramList){
    let folder = String::from("\\processed\\");
    save_file::<NgramList>(String::from(filename),String::from(folder), &ngram_list);
}

pub fn read_ngram_list(filepath: &String) -> NgramList{
    return read_json::<NgramList>(
        filepath.to_string(),
        String::from("\\processed\\"),
    )
    .unwrap_or_else(|_| panic!("Could not read corpus"));
}

pub fn merge_ngram_lists(filepaths: Vec<String>) -> NgramList {

    let mut ngram_lists: Vec<NgramList> = Vec::new();
    for filepath in filepaths {
        let ngram_list = read_json::<NgramList>(
        filepath.to_string(),
        String::from("\\processed\\"),
    )
    .unwrap_or_else(|_| panic!("Could not read corpus"));
        ngram_lists.push(ngram_list);
    }

    let flattened_list = ngram_lists.iter().map(|x| x.clone()).collect::<NgramList>();

    return flattened_list;
}
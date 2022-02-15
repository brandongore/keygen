use std::collections::{HashMap};
use serde::{Deserialize, Serialize};
use crate::file_manager::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct NgramList {
    pub map: HashMap<String, usize>,
    pub gram: usize
}

pub struct SwapCharList {
    pub map: HashMap<char, char>
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

pub fn prepare_ngram_list(corpus: &String, swap_char_list: SwapCharList, length: usize) -> NgramList {
    let mut ngram_list: HashMap<String, usize> = HashMap::new();

    let mut processed_corpus:String=String::new();

    //convert windows newline to just newline for better ngrams
    let corpus = corpus.replace("\r\n", "\n");

    //swap any single character in corpus with another character if its exists in the swap list
    for x in corpus.chars() {
        match x { 
            x if swap_char_list.map.contains_key(&x) => processed_corpus.push(*swap_char_list.map.get(&x).unwrap_or(&(0 as char))), 
            x => processed_corpus.push(x)
        }
    }
    
    for i in 0..processed_corpus.chars().count() - length {
        let slice = &processed_corpus[i..i + length];
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
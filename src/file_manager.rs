use std::{fs::File, io::{BufWriter, Read, BufReader}};

use crate::{penalty, timer::{FuncTimerDisplay, get_sorted_times}, layout::{self, Layout}};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use penalty::*;
use std::error::Error;

#[derive(Serialize, Deserialize)]
pub struct RunState {
    pub git_commit: String,
    pub git_branch: String,
    pub layouts: Vec<BestLayoutsEntry>,
}

impl RunState {
	pub fn new(layouts: Vec<BestLayoutsEntry>) -> Self {
        RunState {
            git_commit: env!("GIT_COMMIT").to_string(),
            git_branch: env!("GIT_BRANCH").to_string(),
            layouts: layouts
        }
    }
}

pub fn read_corpus(corpus_filename: &String) -> String{
    let folder = String::from("\\corpus\\");
    let folder = folder.replace("/", "\\");
    let path = [env!("CARGO_MANIFEST_DIR"), &folder, &corpus_filename, ".txt"];

    let mut f = match File::open(path.join("")) {
		Ok(f) => f,
		Err(e) => {
			println!("Error: {}", e);
			panic!("could not read corpus");
		},
	};
	let mut corpus = String::new();
	match f.read_to_string(&mut corpus) {
		Ok(_) => {return corpus},
		Err(e) => {
			println!("Error: {}", e);
			panic!("could not read corpus");
		}
	};
}

pub fn read_layout(layout_filename: &String) -> Layout{
    let mut f = match File::open(layout_filename) {
        Ok(f) => f,
        Err(e) => {
            println!("Error: {} , reverting to base layout", e);
            return layout::BASE
        }
    };
    let mut layout_str = String::new();
    match f.read_to_string(&mut layout_str) {
        Ok(_) => (),
        Err(e) => {
            println!("Error: {}, reverting to base layout", e);
            return layout::BASE
        }
    };
    return layout::Layout::from_string(&layout_str[..]);
}

pub fn save_file<T>(filename: String, folder: String, data: &T) where T: Serialize {
    let folder = folder.replace("/", "\\");
    let path = [env!("CARGO_MANIFEST_DIR"), &folder, &filename, ".json"];
    let writer = BufWriter::new(File::create(path.join("")).unwrap());
    serde_json::to_writer_pretty(writer, &data).unwrap();
}

pub fn read_json<'a, T>(filename: String, folder: String) -> Result<T, Box<dyn Error>> where T: Deserialize<'a> {
    let folder = folder.replace("/", "\\");
    let path = [env!("CARGO_MANIFEST_DIR"), &folder, &filename, ".json"];
    
    let file = File::open(path.join("")).expect("Unable to open file");
    let mut reader = BufReader::new(file);

    let mut de = serde_json::Deserializer::from_reader(reader);
    let parsedValue = T::deserialize(&mut de)?;

    return Ok(parsedValue);
}

#[cfg(all(feature = "log_benchmark"))]
pub fn save_run_state(layouts: &Vec<BestLayoutsEntry>){
    let timestamp = Utc::now().to_string();
    let timestamp = timestamp.replace(":", "-");
    let path = [env!("CARGO_MANIFEST_DIR"), "\\results\\runstate_", &timestamp, ".json"];
    let writer = BufWriter::new(File::create(path.join("")).unwrap());
    serde_json::to_writer_pretty(writer, &RunState::new(layouts.to_vec())).unwrap();
}

#[cfg(all(feature = "func_timer", feature = "log_benchmark"))]
pub fn save_benchmark(benchmark: &FuncTimerDisplay){
    let timestamp = Utc::now().to_string();
    let timestamp = timestamp.replace(":", "-");
    let path = [env!("CARGO_MANIFEST_DIR"), "\\benchmarks\\benchmark", &timestamp, ".json"];
    let writer = BufWriter::new(File::create(path.join("")).unwrap());
    serde_json::to_writer_pretty(writer, &get_sorted_times(&benchmark)).unwrap();
}

#[cfg(not(feature = "log_benchmark"))]
pub fn save_run_state(_layouts: &Vec<BestLayoutsEntry>){
}

#[cfg(not(all(feature = "func_timer", feature = "log_benchmark")))]
pub fn save_benchmark(_benchmark: &FuncTimerDisplay){
}
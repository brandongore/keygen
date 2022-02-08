use std::{fs::File, io::BufWriter};

use crate::{penalty, timer::{FuncTimerDisplay, get_sorted_times}};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use penalty::*;

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

pub fn save_run_state(layouts: &Vec<BestLayoutsEntry>){
    let timestamp = Utc::now().to_string();
    let timestamp = timestamp.replace(":", "-");
    let path = [env!("CARGO_MANIFEST_DIR"), "\\results\\runstate_", &timestamp, ".json"];
    let writer = BufWriter::new(File::create(path.join("")).unwrap());
    serde_json::to_writer_pretty(writer, &RunState::new(layouts.to_vec())).unwrap();
}

#[cfg(feature = "func_timer")]
pub fn save_benchmark(benchmark: &FuncTimerDisplay){
    let timestamp = Utc::now().to_string();
    let timestamp = timestamp.replace(":", "-");
    let path = [env!("CARGO_MANIFEST_DIR"), "\\benchmarks\\benchmark", &timestamp, ".json"];
    let writer = BufWriter::new(File::create(path.join("")).unwrap());
    serde_json::to_writer_pretty(writer, &get_sorted_times(&benchmark)).unwrap();
}
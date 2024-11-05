use std::{ fs::{ File, DirEntry, self }, io::{ BufWriter, Read, BufReader }, path::Path };

use crate::{ penalty, timer::{ FuncTimerDisplay, get_sorted_times }, layout::{ self, Layout } };

use chrono::Utc;
use itertools::Itertools;
use rayon::iter::{
    ParallelBridge,
    ParallelIterator,
    IntoParallelIterator,
    IntoParallelRefIterator,
};
use serde::{ Deserialize, Serialize };
use penalty::*;
use std::error::Error;
use jwalk::{ WalkDir, Parallelism };

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
            layouts: layouts,
        }
    }
}

pub fn read_text(corpus_filename: &String) -> String {
    let folder = String::from("\\corpus\\");
    let folder = folder.replace("/", "\\");
    let path = [env!("CARGO_MANIFEST_DIR"), &folder, &corpus_filename, ".json"];

    let mut f = match File::open(path.join("")) {
        Ok(f) => f,
        Err(e) => {
            println!("Error: {}", e);
            panic!("could open file");
        }
    };
    let mut text = String::new();
    match f.read_to_string(&mut text) {
        Ok(_) => {
            return text;
        }
        Err(e) => {
            println!("Error: {}", e);
            panic!("could not read text");
        }
    }
}

pub fn read_layout(layout_filename: &String) -> Layout {
    let mut f = match File::open(layout_filename) {
        Ok(f) => f,
        Err(e) => {
            println!("Error: {} , reverting to base layout", e);
            return layout::BASE;
        }
    };
    let mut layout_str = String::new();
    match f.read_to_string(&mut layout_str) {
        Ok(_) => (),
        Err(e) => {
            println!("Error: {}, reverting to base layout", e);
            return layout::BASE;
        }
    }
    return layout::Layout::from_string(&layout_str[..]);
}

pub fn save_small_file<T>(filename: String, folder: String, data: &T) where T: Serialize {
    let folder = folder.replace("/", "\\");
    let path = [env!("CARGO_MANIFEST_DIR"), &folder, &filename, ".json"];
    //let path = ["H:\\keygen", &folder, &filename, ".json"];
    let writer = BufWriter::new(File::create(path.join("")).unwrap());
    serde_json::to_writer(writer, &data).unwrap();
}

pub fn save_file<T>(filename: String, folder: String, data: &T) where T: Serialize {
    let folder = folder.replace("/", "\\");
    let path = [env!("CARGO_MANIFEST_DIR"), &folder, &filename, ".json"];
    //let path = ["C:\\git\\rskeyboard", &folder, &filename, ".json"];
    println!("path - {:?}", path.join(""));
    //C:\dev\dactylmanuform\rustkeygen\mykeygen\keygen\target\release
    let writer = BufWriter::new(File::create(path.join("")).unwrap());
    serde_json::to_writer_pretty(writer, &data).unwrap();
}

pub fn read_batch_json<'a, T>(filename: String, folder: String) -> Result<T, serde_json::Error>
    where T: Deserialize<'a>
{
    let folder = folder.replace("/", "\\");
    let path = ["H:\\keygen", &folder, &filename, ".json"];
    println!("path----- {:?}", path.join(""));
    let file = File::open(path.join("")).expect("Unable to open file");
    let mut reader = BufReader::new(file);

    let mut de = serde_json::Deserializer::from_reader(reader);
    let parsedValue = T::deserialize(&mut de);

    // let parsedValue = match T::deserialize(&mut de) {
    //     Ok(parsedValue) => {
    //         return parsedValue;
    //         //println!("id = {:?}", parsedValue.unique_id);
    //     },
    //     Err(msg) => {
    //         println!("{:?}", msg);
    //         // handle error here
    //     }
    // };

    return parsedValue;
}

pub fn read_json<'a, T>(filename: String, folder: String) -> Result<T, serde_json::Error>
    where T: Deserialize<'a>
{
    let folder = folder.replace("/", "\\");
    let path = [env!("CARGO_MANIFEST_DIR"), &folder, &filename, ".json"];
    //let path = ["C:\\git\\rskeyboard", &folder, &filename, ".json"];
    
    println!("path----- {:?}", path.join(""));
    let file = File::open(path.join("")).expect("Unable to open file");
    let mut reader = BufReader::new(file);

    let mut de = serde_json::Deserializer::from_reader(reader);
    let parsedValue = T::deserialize(&mut de);

    // let parsedValue = match T::deserialize(&mut de) {
    //     Ok(parsedValue) => {
    //         return parsedValue;
    //         //println!("id = {:?}", parsedValue.unique_id);
    //     },
    //     Err(msg) => {
    //         println!("{:?}", msg);
    //         // handle error here
    //     }
    // };

    return parsedValue;
}

pub fn read_directory_files(
    directory: &String,
    dir_filetype_filter: &String
) -> Vec<std::string::String> {
    // let dir = fs::read_dir(".").expect("could not read directory");
    // dir
    let files = jwalk::WalkDir
        ::new(directory)
        .parallelism(Parallelism::RayonNewPool(0))
        .into_iter()
        .par_bridge()
        .filter_map(|dir_entry_result| {
            let dir_entry = dir_entry_result.ok()?;
            if
                dir_entry.file_type().is_file() &&
                dir_entry.file_name.to_string_lossy().ends_with(dir_filetype_filter)
            {
                let path = dir_entry.path();

                //println!("path: {}", path.to_str().unwrap().to_string());
                let text = std::fs::read_to_string(path).ok()?;
                if !text.is_empty() {
                    return Some(text);
                }
            }
            None
        })
        .collect::<Vec<_>>();

    let strings: Vec<Vec<String>> = files
        .into_par_iter()
        .map(|text| { text.split("\r\n").map(|line|{line.to_string()}).collect::<Vec<String>>() })
        .collect::<Vec<Vec<String>>>();

    return strings.into_iter().flatten().collect::<Vec<String>>();
}

pub struct FileResult<T> {
    pub data: T,
    pub filename: String,
}

pub fn read_json_directory_files<'a, T>(
    directory: &String,
    dir_filetype_filter: &String
) -> Vec<FileResult<T>>
    where T: Deserialize<'a> + std::marker::Send
{
    return jwalk::WalkDir
        ::new(directory)
        .parallelism(Parallelism::RayonNewPool(0))
        .into_iter()
        .par_bridge()
        .filter_map(|dir_entry_result| {
            let dir_entry = dir_entry_result.ok()?;
            if
                dir_entry.file_type().is_file() &&
                dir_entry.file_name.to_string_lossy().ends_with(dir_filetype_filter)
            {
                let path = dir_entry.path();
                let filename: String = path.file_stem().unwrap().to_str().unwrap().to_owned();
                println!("path: {}", filename);
                //println!("path2: {}", path.to_str().unwrap().to_string());
                let text = read_json::<T>(filename.clone(), String::from("\\evaluation\\"));

                if text.is_ok() {
                    let result = FileResult { data: text.ok()?, filename: filename };

                    return Some(result);
                }
            }
            None
        })
        .collect::<Vec<_>>();
}

pub fn read_json_evaluated_directory_files<'a, T>(
    directory: &String,
    dir_filetype_filter: &String
) -> Vec<FileResult<T>>
    where T: Deserialize<'a> + std::marker::Send
{
    return jwalk::WalkDir
        ::new(directory)
        .parallelism(Parallelism::RayonNewPool(0))
        .into_iter()
        .par_bridge()
        .filter_map(|dir_entry_result| {
            let dir_entry = dir_entry_result.ok()?;
            if
                dir_entry.file_type().is_file() &&
                dir_entry.file_name.to_string_lossy().ends_with(dir_filetype_filter)
            {
                let path = dir_entry.path();
                let filename: String = path.file_stem().unwrap().to_str().unwrap().to_owned();
                //println!("path: {}", filename);
                //println!("path2: {}", path.to_str().unwrap().to_string());
                let text = read_batch_json::<T>(filename.clone(), String::from("\\evaluated\\"));

                if text.is_ok() {
                    let result = FileResult { data: text.ok()?, filename: filename };

                    return Some(result);
                }
            }
            None
        })
        .collect::<Vec<_>>();
}

// pub fn process_json_evaluated_directory_files<'a, T>(directory: &String, dir_filetype_filter: &String) where T: Deserialize<'a>  + std::marker::Send{
//     return jwalk::WalkDir::new(directory)
//     .parallelism(Parallelism::RayonNewPool(0))
//     .into_iter()
//     .par_bridge()
//     .for_each(|dir_entry_result| {
//         let dir_entry = dir_entry_result.ok().unwrap();
//         if dir_entry.file_type().is_file() && dir_entry.file_name.to_string_lossy().ends_with(dir_filetype_filter) {
//             let path = dir_entry.path();
//             let filename: String = path.file_stem().unwrap().to_str().unwrap().to_owned();
//             //println!("path: {}", filename);
//             //println!("path2: {}", path.to_str().unwrap().to_string());
//             let text = read_batch_json::<T>(filename.clone(), String::from("\\evaluated\\"));

//             if text.is_ok() {
//                 callback(text.ok().unwrap());
//                 //return Some(text.ok()?);
//             }
//         }

//     });
// }

#[cfg(all(feature = "log_benchmark"))]
pub fn save_run_state(layouts: &Vec<BestLayoutsEntry>) {
    let timestamp = Utc::now().to_string();
    let timestamp = timestamp.replace(":", "-");
    let path = [env!("CARGO_MANIFEST_DIR"), "\\simulator_results\\runstate_", &timestamp, ".json"];
    let writer = BufWriter::new(File::create(path.join("")).unwrap());
    serde_json::to_writer_pretty(writer, &RunState::new(layouts.to_vec())).unwrap();
}

#[cfg(all(feature = "func_timer", feature = "log_benchmark"))]
pub fn save_benchmark(benchmark: &FuncTimerDisplay) {
    let timestamp = Utc::now().to_string();
    let timestamp = timestamp.replace(":", "-");
    let path = [env!("CARGO_MANIFEST_DIR"), "\\benchmarks\\benchmark", &timestamp, ".json"];
    let writer = BufWriter::new(File::create(path.join("")).unwrap());
    serde_json::to_writer_pretty(writer, &get_sorted_times(&benchmark)).unwrap();
}

#[cfg(not(feature = "log_benchmark"))]
pub fn save_run_state(_layouts: &Vec<BestLayoutsEntry>) {}

#[cfg(not(all(feature = "func_timer", feature = "log_benchmark")))]
pub fn save_benchmark(_benchmark: &FuncTimerDisplay) {}
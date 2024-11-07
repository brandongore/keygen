extern crate getopts;

mod annealing;
mod corpus_manager;
mod file_manager;
mod layout;
mod penalty;
mod simulator;
mod timer;
mod evaluator;
mod evaluator_penalty;
mod evaluator_penalty_small;

use chrono::Utc;
use corpus_manager::{batch_parse_ngram_list, read_json_array_list, generate_ngram_list, generate_ngram_relation_list, save_ngram_list_relation_mapping, save_string_list, read_ngram_relation_mapping};
use evaluator::{compare_layouts, evaluate_by_ngram_frequency, evaluate_layouts, evaluate_position_combinations, evaluate_position_penalty_hashmap, evaluate_relation};
use file_manager::{read_directory_files, read_json_directory_files, save_small_file};
use getopts::Options;
use itertools::Itertools;
use penalty::BestLayoutsEntry;
use std::fs::File;
use std::io::Read;
use std::{collections::HashMap, env};
use timer::{FuncTimer, FuncTimerDisplay, Timer, TimerState};

use crate::corpus_manager::{NgramList, SwapCharList, prepare_ngram_list, save_ngram_list, merge_ngram_lists, read_ngram_list, parse_ngram_list, normalize_ngram_list};
use crate::evaluator::{refine_evaluation, evaluate_positions};
use crate::file_manager::{read_text, read_layout, save_benchmark, read_json_evaluated_directory_files};
use crate::layout::{Layout, NUM_OF_KEYS, BASE};

//  made thumbs their own hand,
//  as they dont really matter from strain perspective when analysing alternation/rolls/etc

/* TODO:
        add penalities for uneven finger load
        add penalities for uneven hand load
        unicode support
        cache layout-position-map and apply swaps directly to it, so it doesnt have to realocate every cycle
*/

/* running options

    cargo run -- run corpus/books.short.txt
        tests reference layouts and then runs optimaliser

    cargo run -- run-ref corpus/books.short.txt
        test reference layouts


*/
fn main() {
    let ftimer = &mut FuncTimer::new();
    ftimer.start(String::from("main"));

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    opts.optflag("d", "debug", "show debug logging");
    opts.optopt(
        "b",
        "best",
        "number of best layouts to print (default: 1)",
        "BEST_LAYOUTS",
    );
    opts.optopt(
        "s",
        "swaps-per-iteration",
        "maximum number of swaps per iteration (default: 3)",
        "SWAPS",
    );
    opts.optflag("p", "processed", "load preprocessed ngrams from file");
    opts.optopt(
        "c",
        "character-for-split",
        "character to split parse ngram by (default: empty string)",
        "split char",
    );
    opts.optflag(
        "t",
        "tab-split",
        "tab to split parse ngram by (default: empty string)"
    );
    opts.optopt(
        "n",
        "normalize-length",
        "what ngram length to normalize list by",
        "normalize length",
    );
    opts.optopt(
        "f",
        "filter",
        "directory filetype filter",
        "directory filter",
    );

    let args: Vec<String> = env::args().collect();
    let progname = &args[0];
    if args.len() < 2 {
        print_usage(progname, opts);
        return;
    }
    let command = &args[1];
    let matches = match opts.parse(&args[2..]) {
        Ok(m) => m,
        Err(f) => {
            panic!("{}", f.to_string())
        }
    };

    // --help
    if matches.opt_present("h") {
        print_usage(progname, opts);
        return;
    }

    // Read corpus filename.
    let corpus_filename = match matches.free.get(0) {
        Some(f) => f,
        None => {
            println!("Could not read corpus");
            print_usage(progname, opts);
            return;
        }
    };

    // Read layout filename, if applicable.
    let mut layout_filename = String::new();
    layout_filename = match matches.free.get(1) {
        Some(f) => f.to_string(),
        None => "".to_string(),
    };

    // Read layout, if applicable.
    //let layout = read_layout(&layout_filename);

    // Parse options.
    let debug = matches.opt_present("d");
    let load_processed = matches.opt_present("p");
    let top = numopt(matches.opt_str("t"), 1usize);
    let swaps = numopt(matches.opt_str("s"), 2usize);
    let normalize_length = numopt(matches.opt_str("n"), 2);
    let split_char: String;
    //cant pass tab in console so using tab flag
    if matches.opt_present("t") {
        split_char = "\t".to_string();
    }
    else{
        split_char = numopt(matches.opt_str("c"), "".to_string());
    }
    let dir_filetype_filter = numopt(matches.opt_str("f"), "".to_string());

    match command.as_ref() {
        "prepare" => prepare(corpus_filename, split_char),
        "run" => run(corpus_filename, &BASE, debug, top, swaps, load_processed, split_char, ftimer),
        "merge" => merge(corpus_filename),
        "parse" => parse(corpus_filename, split_char),
        "normalize" => normalize(corpus_filename, normalize_length),
        "batch" => batch(corpus_filename, &dir_filetype_filter, split_char, normalize_length),
        "default" => create_default(corpus_filename),
        "evaluate" => batch_evaluate(corpus_filename, &layout_filename, &dir_filetype_filter, ftimer),
        "compare" => batch_compare(&layout_filename, &dir_filetype_filter, ftimer),
        "refine-evaluation" => batch_refine(corpus_filename, &layout_filename, &dir_filetype_filter, ftimer),
        "generate" => generate(corpus_filename, normalize_length),
        "run-ref" => run_ref(corpus_filename, split_char, normalize_length),
        //cargo run --release --features "func_timer" -- evaluate-positions result2_Brandon_Gore_messages_normalized_3
        "evaluate-positions" => prebuild_positions(corpus_filename),
        //cargo run --release --features "func_timer" -- evaluate-relation result2_Brandon_Gore_messages_normalized_relation_3-2022-12-11 18-51-48.196865 UTC
        "evaluate-relation" => evaluate_relation_mapping(corpus_filename),
        "evaluate-layout" => evaluate_layout(corpus_filename),
        "evaluate-score" => evaluate_layout_score(corpus_filename),
        _ => print_usage(progname, opts),
        //"refine" => ,//refine(&corpus[..], layout, debug, top, swaps),
    };
    ftimer.stop(String::from("main"));

    let timer_display = FuncTimerDisplay::new(ftimer);

    save_benchmark(&timer_display);

    print!("{}", timer_display);
}

fn run(
    filepath: &String,
    layout: &layout::Layout,
    debug: bool,
    top: usize,
    swaps: usize,
    load_processed: bool,
    split_char: String,
    timer: &mut HashMap<String, TimerState>,
) {
    let corpus: String;
    let ngram_list;
    timer.start(String::from("read"));
    if load_processed {
        ngram_list= read_ngram_list(&filepath);
    }
    else{
        corpus = read_text(&filepath.to_string());
        let swap_list: SwapCharList = SwapCharList { map: HashMap::new() };
        ngram_list = prepare_ngram_list(&corpus.to_string(), swap_list, &split_char, 4);
    }
    timer.stop(String::from("read"));

    timer.start(String::from("run"));
    simulator::simulate(ngram_list, layout, debug, top, swaps, timer);
    timer.stop(String::from("run"));
}

fn prepare(
    filepath: &String,
    split_char: String
) {
    let corpus= read_text(&filepath);
    let swap_list: SwapCharList = SwapCharList { map:  HashMap::from([
        
    ])};
    let ngram_list = prepare_ngram_list(&corpus.to_string(),swap_list, &split_char, 4);
    save_ngram_list(&filepath, ngram_list);
}

fn merge(
    filepaths: &String
) {
    let filepath_split = filepaths.split(",");
    let filepath_list: Vec<String> = filepath_split.map(|x| x.to_string()).collect::<Vec<String>>();
    let combined_filename = filepath_list.join("_");
    let ngram_list = merge_ngram_lists(filepath_list);
    save_ngram_list(&combined_filename, ngram_list);
}

fn parse(
    filepath: &String,
    split_char: String
) {
    let corpus= read_text(&filepath);
    let ngram_list = parse_ngram_list(&corpus.to_string(), &split_char, true, 3);
    save_ngram_list(&filepath, ngram_list);
}

fn normalize(
    filepath: &String,
    normalize_length: usize
) {
    let existing_ngram_list= read_ngram_list(&filepath);
    let ngram_list = normalize_ngram_list(existing_ngram_list, normalize_length);
    let normalized_filename = [filepath, "_normalized","_", normalize_length.to_string().as_str()].join("");
    save_ngram_list(&normalized_filename, ngram_list);
}

fn batch(
    filepath: &String,
    dir_filetype_filter: &String,
    split_char: String,
    normalize_length: usize
) {
    let contents = read_directory_files(filepath, dir_filetype_filter);

    let timestamp = Utc::now().to_string();
    let timestamp = &timestamp.replace(":", "").replace(" ", "_")[0..17];
    let string_list_filename = ["list","_", &dir_filetype_filter.replace(".", "") ,"_", normalize_length.to_string().as_str(),"_",timestamp].join("");
    save_string_list(&string_list_filename, contents);


    // let ngram_list = batch_parse_ngram_list(contents, &split_char, normalize_length);
    // let timestamp = Utc::now().to_string();
    // let timestamp = &timestamp.replace(":", "").replace(" ", "_")[0..17];
    // let normalized_filename = ["batch","_", normalize_length.to_string().as_str(),"_",timestamp].join("");
    // save_ngram_list(&normalized_filename, ngram_list);
}

fn create_default(
    filepath: &String
) {
    let ngram_list= read_ngram_list(&filepath);
    evaluate_by_ngram_frequency(ngram_list);
}

fn batch_evaluate(
    corpus_filepath: &String,
    layout_filepath: &String,
    dir_filetype_filter: &String,
    timer: &mut HashMap<String, TimerState>,
) {
     //cargo run --release --features func_timer -- evaluate count_1w_normalized_3 C:\dev\dactylmanuform\rustkeygen\mykeygen\keygen\evaluation -f ".json"

    let existing_ngram_list= read_ngram_list(&corpus_filepath);

    let layouts: Vec<file_manager::FileResult<Vec<String>>> = read_json_directory_files(layout_filepath, dir_filetype_filter);
    timer.start(String::from("evaluate"));
    let layout_list = evaluate_layouts(existing_ngram_list, layouts, timer);
    timer.stop(String::from("evaluate"));
    // println!("{}", layout_list.len());
    // for entry in layout_list {
    //     let folder = String::from("\\evaluated\\");
    //     save_small_file::<Vec<BestLayoutsEntry>>(entry.filename, String::from(folder), &entry.data);
    // }
}

fn batch_compare(
    layout_filepath: &String,
    dir_filetype_filter: &String,
    timer: &mut HashMap<String, TimerState>,
) {
     //cargo run --release --features func_timer -- evaluate count_1w_normalized_3 C:\dev\dactylmanuform\rustkeygen\mykeygen\keygen\evaluation -f ".json"
     println!("{}", layout_filepath);
    let layouts: Vec<file_manager::FileResult<Vec<BestLayoutsEntry>>> = Vec::new();//read_json_evaluated_directory_files(layout_filepath, dir_filetype_filter);
    println!("{}", layouts.len());
    let result = compare_layouts(layouts, timer);

    println!("................................................");

    for entry in result {
        println!("************************************************");
        print_result(&entry);
        println!("************************************************");
    }
}

fn batch_refine(
    corpus_filepath: &String,
    layout_filepath: &String,
    dir_filetype_filter: &String,
    timer: &mut HashMap<String, TimerState>,
) {
    //cargo run --release --features func_timer -- evaluate count_1w_normalized_3 C:\dev\dactylmanuform\rustkeygen\mykeygen\keygen\evaluation -f ".json"
    let existing_ngram_list= read_ngram_list(&corpus_filepath);

    // println!("{}", layout_filepath);
    // let layouts: Vec<file_manager::FileResult<Vec<BestLayoutsEntry>>> = read_json_evaluated_directory_files(layout_filepath, dir_filetype_filter);
    // println!("{}", layouts.len());
    let layout_list = refine_evaluation(existing_ngram_list, layout_filepath, dir_filetype_filter, timer);

    let timestamp = Utc::now().to_string();
    let timestamp = timestamp.replace(":", "-");
    let filename = ["refined_batch_time-", &timestamp].join("");
    let folder = String::from("\\refined\\");

    save_small_file::<Vec<BestLayoutsEntry>>(filename, String::from(folder), &layout_list);

    println!("................................................");

    for entry in layout_list {
        println!("************************************************");
        print_result(&entry);
        println!("************************************************");
    }
}

fn prebuild_positions(
    corpus_filepath: &String
) {
    let existing_ngram_list= read_ngram_list(&corpus_filepath);

    evaluate_positions(existing_ngram_list);
}

fn evaluate_relation_mapping(
    corpus_filepath: &String
) {
    let existing_ngram_list= read_ngram_relation_mapping(&corpus_filepath);

    evaluate_relation(existing_ngram_list);
}



pub fn normalize_count(count:usize, len: usize) -> f64{
    return (count as f64) / len as f64;
}

pub fn normalize_penalty(penalty:f64, min: f64, range: f64) -> f64{
    return (penalty - min) / range;
}

pub fn print_result<'a>(item: &BestLayoutsEntry) {
    let layout = &item.layout;
    let bad_score_total = item.penalty.bad_score_total;
    let good_score_total = item.penalty.good_score_total;
    let len = item.penalty.len;
    let penalties = &item.penalty.penalties;
    let penalty = &item.penalty;
    let fingers = &penalty.fingers;
    let hands = &penalty.hands;
    let show_all = false;
    let positions = item.penalty.pos;
    let position_penalties = item.penalty.pos_pen;
    let mut position_working = [0; NUM_OF_KEYS];
    position_penalties.into_iter().enumerate().for_each(|(i, penalty)|{
        position_working[i] = (penalty * 100.0) as i128;
    });
    position_working.sort();

    let max_position = position_working[NUM_OF_KEYS-1];
    let min_position_penalty = position_working[0] as f64 / 100.0;
    let range_position_penalty = max_position as f64/100.0 - min_position_penalty;

    print!(
        "{}{}{}{}{}{}{}{}{}{}{}{}",
        format!("\n{}\n", layout),
        format!("{}\n{}\n{}\n{}\n{}\n{}\n",
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"", normalize_penalty(position_penalties[0], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[1], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[2], min_position_penalty, range_position_penalty), 		normalize_penalty(position_penalties[3], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[4], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[5], min_position_penalty, range_position_penalty),""
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"",normalize_penalty(position_penalties[6], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[7], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[8], min_position_penalty, range_position_penalty), 		normalize_penalty(position_penalties[9], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[10], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[11], min_position_penalty, range_position_penalty),""
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	normalize_penalty(position_penalties[12], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[13], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[14], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[15], min_position_penalty, range_position_penalty), 	normalize_penalty(position_penalties[16], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[17], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[18], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[19], min_position_penalty, range_position_penalty), 
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	normalize_penalty(position_penalties[20], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[21], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[22], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[23], min_position_penalty, range_position_penalty), 	normalize_penalty(position_penalties[24], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[25], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[26], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[27], min_position_penalty, range_position_penalty), 
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"","","",normalize_penalty(position_penalties[28], min_position_penalty, range_position_penalty), 	normalize_penalty(position_penalties[29], min_position_penalty, range_position_penalty), "","",""
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"",normalize_penalty(position_penalties[30], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[31], min_position_penalty, range_position_penalty),normalize_penalty(position_penalties[32], min_position_penalty, range_position_penalty), 	normalize_penalty(position_penalties[33], min_position_penalty, range_position_penalty),normalize_penalty(position_penalties[34], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[35], min_position_penalty, range_position_penalty),""
), 
		),
		format!("\n{}\n{}\n{}\n{}\n{}\n{}\n",
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"", normalize_count(positions[0], len), normalize_count(positions[1], len), normalize_count(positions[2], len), 		normalize_count(positions[3], len), normalize_count(positions[4], len), normalize_count(positions[5], len),""
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"",normalize_count(positions[6], len), normalize_count(positions[7], len), normalize_count(positions[8], len), 		normalize_count(positions[9], len), normalize_count(positions[10], len), normalize_count(positions[11], len),""
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	normalize_count(positions[12], len), normalize_count(positions[13], len), normalize_count(positions[14], len), normalize_count(positions[15], len), 	normalize_count(positions[16], len), normalize_count(positions[17], len), normalize_count(positions[18], len), normalize_count(positions[19], len), 
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	normalize_count(positions[20], len), normalize_count(positions[21], len), normalize_count(positions[22], len), normalize_count(positions[23], len), 	normalize_count(positions[24], len), normalize_count(positions[25], len), normalize_count(positions[26], len), normalize_count(positions[27], len), 
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"","","",normalize_count(positions[28], len), 	normalize_count(positions[29], len), "","",""
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"",normalize_count(positions[30], len), normalize_count(positions[31], len),normalize_count(positions[32], len), 	normalize_count(positions[33], len),normalize_count(positions[34], len), normalize_count(positions[35], len),""
), 
		),
        format!("hands: {:<5.3} | {:<5.3}\n", normalize_penalty(hands[0] as f64, 0.0, len as f64), normalize_penalty(hands[1] as f64, 0.0, len as f64)),
        format!(
            "bad score total: {0:<10.2}; good score total: {1:<10.2}; bad score scaled: {2:<10.4}\n",
            bad_score_total,
            good_score_total,
            bad_score_total / (len as f64)
        ),
        //format!("base {}\n",penalties[0]),
        format!(
            "\n{:<30} | {:^7} | {:^7} | {:^8} | {:<10}\n",
            "Name", "% times", "Avg", "% Total", "Total"
        ),
        "----------------------------------------------------------------------\n",
        penalties
            .into_iter()
            .map(|penalty| {
                if penalty.show || show_all {
                    format!(
                        "{:<30} | {:<7.2} | {:<7.3} | {:<8.3} | {:<10.0}\n",
                        penalty.name,
                        (100.0 * penalty.times as f64 / (len as f64)),
                        penalty.total / (len as f64),
                        100.0 * penalty.total / bad_score_total,
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
            fingers[0] as f64 * 100.0 / len as f64 ,
            fingers[1] as f64 * 100.0 / len as f64 ,
            fingers[2] as f64 * 100.0 / len as f64 ,
            fingers[3] as f64 * 100.0 / len as f64 ,
            fingers[7] as f64 * 100.0 / len as f64 ,
            fingers[6] as f64 * 100.0 / len as f64 ,
            fingers[5] as f64 * 100.0 / len as f64 ,
            fingers[4] as f64 * 100.0 / len as f64 
        ),

        format!("{:^5.1}| {:^5.1}\n", penalty.hands[0] as f64 * 100.0 / len as f64 , penalty.hands[1] as f64 * 100.0 / len as f64 ),
        "##########################################################################\n"
    );
}

pub fn print_result_score<'a>(item: evaluator_penalty_small::BestLayoutsEntry) {
    let layout = &item.layout;
    let total = item.penalty.total;
    let bad_score_total = item.penalty.bad_score_total;
    let good_score_total = item.penalty.good_score_total;
    let len = item.penalty.len;
    let penalties = &item.penalty.penalties;
    let penalty = &item.penalty;
    let fingers = &penalty.fingers;
    let hands = &penalty.hands;
    let show_all = false;
    let positions = item.penalty.pos;
    let position_penalties = item.penalty.pos_pen;
    let mut position_working = [0; NUM_OF_KEYS];
    position_penalties.into_iter().enumerate().for_each(|(i, penalty)|{
        position_working[i] = (penalty * 100.0) as i128;
    });
    position_working.sort();

    let max_position = position_working[NUM_OF_KEYS-1];
    let min_position_penalty = position_working[0] as f64 / 100.0;
    let range_position_penalty = max_position as f64/100.0 - min_position_penalty;

    print!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}",
        format!("\n{}\n", layout),
        format!("{}\n{}\n{}\n{}\n{}\n{}\n",
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"", normalize_penalty(position_penalties[0], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[1], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[2], min_position_penalty, range_position_penalty), 		normalize_penalty(position_penalties[3], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[4], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[5], min_position_penalty, range_position_penalty),""
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"",normalize_penalty(position_penalties[6], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[7], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[8], min_position_penalty, range_position_penalty), 		normalize_penalty(position_penalties[9], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[10], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[11], min_position_penalty, range_position_penalty),""
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	normalize_penalty(position_penalties[12], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[13], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[14], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[15], min_position_penalty, range_position_penalty), 	normalize_penalty(position_penalties[16], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[17], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[18], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[19], min_position_penalty, range_position_penalty), 
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	normalize_penalty(position_penalties[20], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[21], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[22], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[23], min_position_penalty, range_position_penalty), 	normalize_penalty(position_penalties[24], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[25], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[26], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[27], min_position_penalty, range_position_penalty), 
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"","","",normalize_penalty(position_penalties[28], min_position_penalty, range_position_penalty), 	normalize_penalty(position_penalties[29], min_position_penalty, range_position_penalty), "","",""
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"",normalize_penalty(position_penalties[30], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[31], min_position_penalty, range_position_penalty),normalize_penalty(position_penalties[32], min_position_penalty, range_position_penalty), 	normalize_penalty(position_penalties[33], min_position_penalty, range_position_penalty),normalize_penalty(position_penalties[34], min_position_penalty, range_position_penalty), normalize_penalty(position_penalties[35], min_position_penalty, range_position_penalty),""
), 
		),
		format!("\n{}\n{}\n{}\n{}\n{}\n{}\n",
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"", normalize_count(positions[0], len), normalize_count(positions[1], len), normalize_count(positions[2], len), 		normalize_count(positions[3], len), normalize_count(positions[4], len), normalize_count(positions[5], len),""
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"",normalize_count(positions[6], len), normalize_count(positions[7], len), normalize_count(positions[8], len), 		normalize_count(positions[9], len), normalize_count(positions[10], len), normalize_count(positions[11], len),""
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	normalize_count(positions[12], len), normalize_count(positions[13], len), normalize_count(positions[14], len), normalize_count(positions[15], len), 	normalize_count(positions[16], len), normalize_count(positions[17], len), normalize_count(positions[18], len), normalize_count(positions[19], len), 
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	normalize_count(positions[20], len), normalize_count(positions[21], len), normalize_count(positions[22], len), normalize_count(positions[23], len), 	normalize_count(positions[24], len), normalize_count(positions[25], len), normalize_count(positions[26], len), normalize_count(positions[27], len), 
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"","","",normalize_count(positions[28], len), 	normalize_count(positions[29], len), "","",""
), 
format!(
	"{:<5.3} {:<5.3} {:<5.3} {:<5.3} | {:<5.3} {:<5.3} {:<5.3} {:<5.3}",
	"",normalize_count(positions[30], len), normalize_count(positions[31], len),normalize_count(positions[32], len), 	normalize_count(positions[33], len),normalize_count(positions[34], len), normalize_count(positions[35], len),""
), 
		),
        format!("hands: {:<5.3} | {:<5.3}\n", normalize_penalty(hands[0] as f64, 0.0, len as f64), normalize_penalty(hands[1] as f64, 0.0, len as f64)),
        format!(
            "total: {0:<10.2}; total scaled: {1:<10.4}\n",
            total,
            total / (len as f64)
        ),
        format!(
            "bad score total: {0:<10.2}; good score total: {1:<10.2}; bad score scaled: {2:<10.4}\n",
            bad_score_total,
            good_score_total,
            bad_score_total / (len as f64)
        ),
        //format!("base {}\n",penalties[0]),
        format!(
            "\n{:<30} | {:^7} | {:^7} | {:^8} | {:<10}\n",
            "Name", "% times", "Avg", "% Total", "Total"
        ),
        "----------------------------------------------------------------------\n",
        penalties
            .into_iter()
            .map(|penalty| {
                if penalty.show || show_all {
                    format!(
                        "{:<30} | {:<7.2} | {:<7.3} | {:<8.3} | {:<10.0}\n",
                        penalty.name,
                        (100.0 * penalty.times as f64 / (len as f64)),
                        penalty.total / (len as f64),
                        100.0 * penalty.total / bad_score_total,
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
            fingers[0] as f64 * 100.0 / len as f64 ,
            fingers[1] as f64 * 100.0 / len as f64 ,
            fingers[2] as f64 * 100.0 / len as f64 ,
            fingers[3] as f64 * 100.0 / len as f64 ,
            fingers[7] as f64 * 100.0 / len as f64 ,
            fingers[6] as f64 * 100.0 / len as f64 ,
            fingers[5] as f64 * 100.0 / len as f64 ,
            fingers[4] as f64 * 100.0 / len as f64 
        ),

        format!("{:^5.1}| {:^5.1}\n", penalty.hands[0] as f64 * 100.0 / len as f64 , penalty.hands[1] as f64 * 100.0 / len as f64 ),
        "##########################################################################\n"
    );
}

fn generate(
    filepath: &String,
    normalize_length: usize
) {
    let corpus= read_json_array_list(&filepath);

    let timestamp = Utc::now().to_string();
    let timestamp = timestamp.replace(":", "-");

    let ngram_list = generate_ngram_list(corpus.clone(), normalize_length);
    let processed_filename = [filepath, "_normalized","_", normalize_length.to_string().as_str(), "-", &timestamp].join("");
    save_ngram_list(&processed_filename, ngram_list);

    let ngram_relation_list = generate_ngram_relation_list(corpus, normalize_length);
    let processed_filename = [filepath, "_normalized_relation","_", normalize_length.to_string().as_str(), "-", &timestamp].join("");
    save_ngram_list_relation_mapping(&processed_filename, ngram_relation_list);
}

fn run_ref(
    filepath: &String,
    split_char: String,
    normalize_length: usize 
){
    //let corpus = read_text(&filepath.to_string());
	// let run_ref_ = |ngrams|{  // making typechecker happy

	// 	let ref_test = |s:&str, l:&layout::Layout|{
	// 		println!("Reference: {}", s);
	// 		let init_pos_map = l.get_position_map();
	// 		let penalty= penalty::calculate_penalty(ngrams, &l);
	// 		 simulator::print_result(&penalty);
	// 		//println!("");
	// 	};
	// 	ref_test("BASE", &layout::BASE);
	// };

    // let swap_list: SwapCharList = SwapCharList { map: HashMap::new() };

    let existing_ngram_list= read_ngram_list(&filepath.to_string());

    let processed_ngrams: Vec<(Vec<char>, usize)> = existing_ngram_list.map
        .into_iter()
        .map(|item| (item.0.chars().collect(), item.1))
        .collect();

        //let layout_string = "fk  wxctlibqspvhuraodg mz yj  e  n ".to_string();
        let layout_string = "aty### beqxvuikhgmdw#cj##lzpo#fns#r".to_string();

        //layout assignment: 6678809.956041669
//all_assigned_char_position_hashmap: {'h': 24, 'b': 16, 'q': 20, 'g': 31, 'p': 0, 'v': 6, 'w': 21, 'e': 26, 'o': 34, 'l': 23, 'j': 8, 'd': 12, 'f': 35, 's': 27, 't': 9, 'a': 33, 'm': 19, ' ': 3, 'y': 25, 'u': 17, 'x': 29, 'z': 22, 'n': 14, 'c': 7, 'r': 18, 'k': 15, 'i': 13}

//layout assignment: 6262990.552083353
//all_assigned_char_position_hashmap: {'p': 4, 'e': 33, 'y': 35, 'a': 18, 's': 31, 'c': 27, 'q': 7, 'r': 26, 'i': 29, 'o': 34, 'w': 5, 't': 9, 'v': 10, 'n': 16, 'b': 17, 'k': 13, 'm': 14, 'z': 21, 'f': 6, 'l': 8, 'd': 15, 'u': 25, ' ': 3, 'x': 11, 'j': 23, 'h': 24, 'g': 19}

        let layout = Layout::from_lower_string(&layout_string[..]); 

        //let layout = layout::BASE;

        let best_layout = penalty::calculate_penalty(&processed_ngrams, &layout);
        simulator::print_result(&best_layout);
    //run_ref_(&corpus_manager::prepare_ngram_list(&corpus, swap_list, &split_char , normalize_length))

	// match  quartads {
	// 	Some(quartads) => run_ref_(quartads),
	// 	None => run_ref_(&corpus_manager::prepare_ngram_list(corpus, swap_list, &split_char , normalize_length)),
	// }

}

fn evaluate_layout(
    filepath: &String
){
    //let corpus = read_text(&filepath.to_string());
	// let run_ref_ = |ngrams|{  // making typechecker happy

	// 	let ref_test = |s:&str, l:&layout::Layout|{
	// 		println!("Reference: {}", s);
	// 		let init_pos_map = l.get_position_map();
	// 		let penalty= penalty::calculate_penalty(ngrams, &l);
	// 		 simulator::print_result(&penalty);
	// 		//println!("");
	// 	};
	// 	ref_test("BASE", &layout::BASE);
	// };

    // let swap_list: SwapCharList = SwapCharList { map: HashMap::new() };

    let existing_ngram_list= read_ngram_relation_mapping(&filepath);

        //let layout_string = "fk  wxctlibqspvhuraodg mz yj  e  n ".to_string();
        let layout_string = "aty### beqxvuikhgmdw#cj##lzpo#fns#r".to_string();
        let layout_string = "q##e###############################".to_string();

        //layout assignment: 6678809.956041669
//all_assigned_char_position_hashmap: {'h': 24, 'b': 16, 'q': 20, 'g': 31, 'p': 0, 'v': 6, 'w': 21, 'e': 26, 'o': 34, 'l': 23, 'j': 8, 'd': 12, 'f': 35, 's': 27, 't': 9, 'a': 33, 'm': 19, ' ': 3, 'y': 25, 'u': 17, 'x': 29, 'z': 22, 'n': 14, 'c': 7, 'r': 18, 'k': 15, 'i': 13}

//layout assignment: 6262990.552083353
//all_assigned_char_position_hashmap: {'p': 4, 'e': 33, 'y': 35, 'a': 18, 's': 31, 'c': 27, 'q': 7, 'r': 26, 'i': 29, 'o': 34, 'w': 5, 't': 9, 'v': 10, 'n': 16, 'b': 17, 'k': 13, 'm': 14, 'z': 21, 'f': 6, 'l': 8, 'd': 15, 'u': 25, ' ': 3, 'x': 11, 'j': 23, 'h': 24, 'g': 19}

        let layout = Layout::from_lower_string(&layout_string[..]); 

        //let layout = layout::BASE;


        evaluator::evaluate_layout(existing_ngram_list, layout);
    //run_ref_(&corpus_manager::prepare_ngram_list(&corpus, swap_list, &split_char , normalize_length))

	// match  quartads {
	// 	Some(quartads) => run_ref_(quartads),
	// 	None => run_ref_(&corpus_manager::prepare_ngram_list(corpus, swap_list, &split_char , normalize_length)),
	// }

}

fn evaluate_layout_score(
    filepath: &String
){
    //let corpus = read_text(&filepath.to_string());
	// let run_ref_ = |ngrams|{  // making typechecker happy

	// 	let ref_test = |s:&str, l:&layout::Layout|{
	// 		println!("Reference: {}", s);
	// 		let init_pos_map = l.get_position_map();
	// 		let penalty= penalty::calculate_penalty(ngrams, &l);
	// 		 simulator::print_result(&penalty);
	// 		//println!("");
	// 	};
	// 	ref_test("BASE", &layout::BASE);
	// };

    // let swap_list: SwapCharList = SwapCharList { map: HashMap::new() };

    let ngram_list= read_ngram_list(&filepath);
    let processed_ngrams: Vec<(Vec<char>, usize)> = ngram_list
    .map
    .into_iter()
    .map(|item| (item.0.chars().collect(), item.1))
    .collect();

        //let layout_string = "fk  wxctlibqspvhuraodg mz yj  e  n ".to_string();
        //let layout_string = "aty### beqxvuikhgmdw#cj##lzpo#fns#r".to_string();

        //let layout_string = "upfzqxyeowsgr_thdnialbcmj vk        ".to_string();

        let layout_string = "uyszqx thpkloienfmcvdgarj#bw########".to_string();

        let layout = Layout::from_lower_string(&layout_string[..]); 

        //let layout = layout::BASE;
        let mut position_combinations = evaluate_position_combinations();

        println!("position_combinations: {:?}", position_combinations.len());
    
        let position_penalties_hashmap:  HashMap<String, evaluator_penalty_small::Penalty<{ layout::NUM_OF_KEYS }>> = evaluate_position_penalty_hashmap(
            position_combinations.clone()
        );

        let penalty = evaluator::evaluate_layout_score(&processed_ngrams, &layout, &position_penalties_hashmap);

        let result = evaluator_penalty_small::BestLayoutsEntry {
            layout: layout.clone(),
            penalty,
        };
        print_result_score(result.clone());
    //run_ref_(&corpus_manager::prepare_ngram_list(&corpus, swap_list, &split_char , normalize_length))

	// match  quartads {
	// 	Some(quartads) => run_ref_(quartads),
	// 	None => run_ref_(&corpus_manager::prepare_ngram_list(corpus, swap_list, &split_char , normalize_length)),
	// }

}

fn refine(s: &str, layout: &layout::Layout, debug: bool, top: usize, swaps: usize, split_char: &String) {
    let init_pos_map = layout::BASE.get_position_map();
    let swap_list: SwapCharList = SwapCharList { map: HashMap::new() };
    let quartads = corpus_manager::prepare_ngram_list(&s.to_string(), swap_list, split_char, 4);
    let len = s.len();

    //simulator::refine(&quartads, len, layout, &penalties, debug, top, swaps);
}

fn print_usage(progname: &String, opts: Options) {
    let brief = format!("Usage: {} (run|run-ref) <corpus> [OPTIONS]", progname);
    print!("{}", opts.usage(&brief));
}

fn numopt<T>(s: Option<String>, default: T) -> T
where
    T: std::str::FromStr + std::fmt::Display,
{
    match s {
        None => default,
        Some(num) => match num.parse::<T>() {
            Ok(n) => n,
            Err(_) => {
                println!(
                    "Error: invalid option value {}. Using default value {}.",
                    num, default
                );
                default
            }
        },
    }
}

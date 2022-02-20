extern crate getopts;

mod annealing;
mod corpus_manager;
mod file_manager;
mod layout;
mod penalty;
mod simulator;
mod timer;

use getopts::Options;
use std::fs::File;
use std::io::Read;
use std::{collections::HashMap, env};
use timer::{FuncTimer, FuncTimerDisplay, Timer, TimerState};

use crate::corpus_manager::{NgramList, SwapCharList, prepare_ngram_list, save_ngram_list, merge_ngram_lists, read_ngram_list, parse_ngram_list, normalize_ngram_list};
use crate::file_manager::{read_text, read_layout, save_benchmark};

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
    let layout = read_layout(&layout_filename);

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

    match command.as_ref() {
        "prepare" => prepare(corpus_filename, split_char),
        "run" => run(corpus_filename, &layout, debug, top, swaps, load_processed, split_char, ftimer),
        "merge" => merge(corpus_filename),
        "parse" => parse(corpus_filename, split_char),
        "normalize" => normalize(corpus_filename, normalize_length),
        // "run-ref" => run_ref(ngList, None),
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
    simulator::simulate(&ngram_list, layout, debug, top, swaps, timer);
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
    let ngram_list = parse_ngram_list(&corpus.to_string(), &split_char, true, 4);
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

// fn run_ref(corpus: NgramList,quartads:Option<&NgramList>)
// {
// 	let run_ref_ = |quartads|{  // making typechecker happy

// 		let ref_test = |s:&str, l:&layout::Layout|{
// 			println!("Reference: {}", s);
// 			let init_pos_map = l.get_position_map();
// 			let penalty= penalty::calculate_penalty(quartads, &l);
// 			simulator::print_result(&penalty);
// 			println!("");
// 		};
// 		ref_test("BASE", &layout::BASE);
// 	};

// 	match  quartads {
// 		Some(quartads) => run_ref_(quartads),
// 		None => run_ref_(&corpus_manager::prepare_ngram_list(s, 4)),
// 	}

// }

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

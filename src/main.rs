extern crate getopts;

mod layout;
mod penalty;
mod annealing;
mod simulator;
mod timer;
mod file_manager;

use std::{env, collections::HashMap};
use std::fs::File;
use std::io::Read;
use getopts::Options;
use penalty::QuartadList;
use timer::{FuncTimer, FuncTimerDisplay, Timer, TimerState};

use crate::file_manager::{save_benchmark, read_corpus, read_layout}; 

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
fn main()
{
	let ftimer =&mut FuncTimer::new();
	ftimer.start(String::from("main"));

	let mut opts = Options::new();
	opts.optflag("h", "help", "print this help menu");
	opts.optflag("d", "debug", "show debug logging");
	opts.optopt("t", "top", "number of top layouts to print (default: 1)", "TOP_LAYOUTS");
	opts.optopt("s", "swaps-per-iteration", "maximum number of swaps per iteration (default: 3)", "SWAPS");

	let args: Vec<String> = env::args().collect();
	let progname = &args[0];
	if args.len() < 2 {
		print_usage(progname, opts);
		return;
	}
	let command = &args[1];
	let matches = match opts.parse(&args[2..]) {
		Ok(m) => { m }
		Err(f) => { panic!("{}",f.to_string()) }
	};

	// --help
	if matches.opt_present("h") {
		print_usage(progname, opts);
		return;
	}

	// Read corpus.
	let corpus_filename = match matches.free.get(0) {
		Some(f) => f,
		None => {
			println!("Could not read corpus");
			print_usage(progname, opts);
			return;
		},
	};

	let mut corpus = read_corpus(corpus_filename);

	// Read layout, if applicable.
	let mut layout_filename = String::new();
	layout_filename = match matches.free.get(1) {
		Some(f) => f.to_string(),
		None => "".to_string(),
	};

	// Read layout, if applicable.
	let layout = read_layout(&layout_filename);

	// Parse options.
	let debug = matches.opt_present("d");
	let top   = numopt(matches.opt_str("t"), 1usize);
	let swaps = numopt(matches.opt_str("s"), 2usize);

	match command.as_ref() {
		"run" => run(&corpus[..], &layout, debug, top, swaps, ftimer),
		"run-ref" => run_ref(&corpus[..], None),
		_ => print_usage(progname, opts),
		//"refine" => ,//refine(&corpus[..], layout, debug, top, swaps),
	};
	ftimer.stop(String::from("main"));

	let timer_display = FuncTimerDisplay::new(ftimer);

	save_benchmark(&timer_display);

	print!("{}", timer_display);
}

fn run(s: &str, layout: &layout::Layout, debug: bool, top: usize, swaps: usize, timer: &mut HashMap<String, TimerState>)
{
	timer.start(String::from("run"));
	//let init_pos_map = layout.get_position_map();
	let quartads = penalty::prepare_quartad_list(s);
	
	//run_ref(s, Some(&quartads));
	simulator::simulate(&quartads, layout, debug, top, swaps, timer);
	timer.stop(String::from("run"));
}

fn run_ref(s: &str,quartads:Option<&QuartadList> )
{
	let run_ref_ = |quartads|{  // making typechecker happy
		
		let ref_test = |s:&str, l:&layout::Layout|{
			println!("Reference: {}", s);
			let init_pos_map = l.get_position_map();
			let penalty= penalty::calculate_penalty(quartads, &l);
			simulator::print_result(&penalty);
			println!("");
		};
		ref_test("BASE", &layout::BASE);
	};
	
	match  quartads {
		Some(quartads) => run_ref_(quartads),
		None => run_ref_(&penalty::prepare_quartad_list(s)),
	}
	
	
}

fn refine(s: &str, layout: &layout::Layout, debug: bool, top: usize, swaps: usize)
{
	let init_pos_map = layout::BASE.get_position_map();
	let quartads = penalty::prepare_quartad_list(s);
	let len = s.len();

	//simulator::refine(&quartads, len, layout, &penalties, debug, top, swaps);
}

fn print_usage(progname: &String, opts: Options)
{
	let brief = format!("Usage: {} (run|run-ref) <corpus> [OPTIONS]", progname);
	print!("{}", opts.usage(&brief));
}

fn numopt<T>(s: Option<String>, default: T)
-> T
where T: std::str::FromStr + std::fmt::Display
{
	match s {
		None => default,
		Some(num) => match num.parse::<T>() {
			Ok(n) => n,
			Err(_) => {
				println!("Error: invalid option value {}. Using default value {}.", num, default);
				default
			},
		},
	}
}

extern crate getopts;

mod layout;
mod penalty;
mod annealing;
mod simulator;


use std::env;
use std::fs::File;
use std::io::Read;
use getopts::Options;
use penalty::QuartadList;



//  made thumbs their own hand, 
//  as they dont really matter from strain perspective when analysing alternation/rolls/etc



/* TODO:
		add penalities for uneven finger load
		add penalities for uneven hand load
		unicode support
		cache layout-position-map and apply swaps directly to it, so it doesnt have to realocate every cycle

*/

//cargo run -- run corpus/books.short.txt
fn main()
{
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
		Err(f) => { panic!(f.to_string()) }
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
			print_usage(progname, opts);
			return;
		},
	};
	let mut f = match File::open(corpus_filename) {
		Ok(f) => f,
		Err(e) => {
			println!("Error: {}", e);
			panic!("could not read corpus");
		},
	};
	let mut corpus = String::new();
	match f.read_to_string(&mut corpus) {
		Ok(_) => (),
		Err(e) => {
			println!("Error: {}", e);
			panic!("could not read corpus");
		}
	};


	// Read layout, if applicable.
	let _layout;
	let layout = match matches.free.get(1) {
		None => &layout::QWERTY_LAYOUT,
		Some(layout_filename) => {
			let mut f = match File::open(layout_filename) {
				Ok(f) => f,
				Err(e) => {
					println!("Error: {}", e);
					panic!("could not read layout");
				}
			};
			let mut layout_str = String::new();
			match f.read_to_string(&mut layout_str) {
				Ok(_) => (),
				Err(e) => {
					println!("Error: {}", e);
					panic!("could not read layout");
				}
			};
			_layout = layout::Layout::from_string(&layout_str[..]);
			&_layout
		},
	};

	// Parse options.
	let debug = matches.opt_present("d");
	let top   = numopt(matches.opt_str("t"), 1usize);
	let swaps = numopt(matches.opt_str("s"), 2usize);

	match command.as_ref() {
		"run" => run(&corpus[..], layout, debug, top, swaps),
		_ => print_usage(progname, opts),
		//"run-ref" => run_ref(&corpus[..], &None),
		//"refine" => ,//refine(&corpus[..], layout, debug, top, swaps),
	};
}

fn run(s: &str, layout: &layout::Layout, debug: bool, top: usize, swaps: usize)
{
	let init_pos_map = layout.get_position_map();
	let quartads = penalty::prepare_quartad_list(s, &init_pos_map);
	let len:usize = (&quartads.map).into_iter().map(|(_,i)|*i).sum::<i64>() as usize;
	
	run_ref(s, &quartads);
	simulator::simulate(&quartads, len, layout, debug, top, swaps);
	
}

fn run_ref(s: &str,quartads: &QuartadList )
{
	let len = (&quartads.map).into_iter().map(|(_,i)|i).sum::<i64>() as usize;

	let ref_test = |s:&str, l:&layout::Layout|{
		println!("Reference: {}", s);
		let init_pos_map = l.get_position_map();
		
		let penalty= penalty::calculate_penalty(&quartads, &l);
	
		simulator::print_result(&penalty, len);
		println!("");
	};
	ref_test("QWERTY", &layout::QWERTY_LAYOUT);
	ref_test("DVORAK", &layout::DVORAK_LAYOUT);
	ref_test("MTGAP", &layout::MTGAP_LAYOUT);
	ref_test("COLEMAK", &layout::COLEMAK_LAYOUT);
	ref_test("QGMLWY", &layout::QGMLWY_LAYOUT);
	ref_test("ARENSITO", &layout::ARENSITO_LAYOUT);
	ref_test("MALTRON", &layout::MALTRON_LAYOUT);
	ref_test("RSTHD", &layout::RSTHD);
	ref_test("CAPEWELL", &layout::CAPEWELL_LAYOUT);
	//ref_test("DA_BEST", &layout::DABEST);
	//ref_test("X1", &layout::X1);

	
	
}

fn refine(s: &str, layout: &layout::Layout, debug: bool, top: usize, swaps: usize)
{
	let init_pos_map = layout::QWERTY_LAYOUT.get_position_map();
	let quartads = penalty::prepare_quartad_list(s, &init_pos_map);
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
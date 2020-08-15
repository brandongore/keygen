/// Applies the math in annealing.rs to keyboard layouts.


extern crate rand;
extern crate rayon;
extern crate num_cpus;

//use self::rand::{random, thread_rng};
use self::rand::*;
use std::cmp::Ordering;
//use std::collections::;
use self::rayon::prelude::*;
use std::iter;
use std::*;

use layout;
use penalty;
use annealing;

#[derive(Clone)]
pub struct BestLayoutsEntry<'a>
{
	pub layout:  layout::Layout,
	pub penalty: f64,
	pub penalties:Vec<penalty::KeyPenaltyResult<'a>>
}
impl<'a> Ord for BestLayoutsEntry<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
		match self.penalty.partial_cmp(&other.penalty) {
			Some(ord) => ord,
			None => Ordering::Equal
		}
    }
}
impl<'a> PartialEq for BestLayoutsEntry<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.penalty == other.penalty
    }
}
impl<'a> Eq for BestLayoutsEntry<'a> {}
impl<'a> PartialOrd for BestLayoutsEntry<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.penalty.partial_cmp(&other.penalty)
    }
}
impl<'a> BestLayoutsEntry<'a>
{
	fn new(layout:layout::Layout, pens: (f64,Vec<penalty::KeyPenaltyResult<'a>>)) -> BestLayoutsEntry<'a>{
		Self{
			layout: layout,
			penalty: pens.0,
			penalties: pens.1
		}
	}
}
struct LL<T> {
	item: T,
	len: usize,
	next:Option<Box<LL<T>>>
}

pub fn simulate<'a>(
	quartads:    &penalty::QuartadList<'a>,
	len: 		 usize,
	init_layout: &layout::Layout,
	penalties:   &Vec<penalty::KeyPenalty<'a>>,
	debug:        bool,
	top_layouts:  usize,
	num_swaps:    usize)
{
	const CYCLES:i32 = 	205000;
	const ITERATIONS:i32 = 5;
	let threads = num_cpus::get();
	let BEST_LAYOUTS_KEPT : usize= threads*3;


	let initial_penalty = || penalty::calculate_penalty(&quartads, init_layout, penalties, true);
	let mut best_layouts: Vec<BestLayoutsEntry> = 
		(0..BEST_LAYOUTS_KEPT)
		.map(|_| BestLayoutsEntry::new(init_layout.clone(), initial_penalty()))
		.collect();

	
		// in each iteration each thread takes a random layout and tries to optimalize it for 5000 cycles; 
	//results are appended to bestLayouts, which is then sorted and truntcated back to best ten
	for it_num in 1..ITERATIONS+1 {
		println!("iteration: {}", it_num);
		let iteration: Vec<BestLayoutsEntry>= 
		(0..threads)
		.map(|i|&best_layouts[best_layouts.len() - 1 - i as usize])
		.collect::<Vec<&BestLayoutsEntry>>()
		.into_par_iter()
		.map(|entry| {
			let mut accepted_layout = entry.clone();
			let mut bestLayout: BestLayoutsEntry= entry.clone();
			
			for i in 1..CYCLES+1 {
				
				let mut curr_layout = accepted_layout.clone();
				curr_layout.layout.shuffle(random::<usize>() % num_swaps + 1);
				
				// Calculate penalty.
				let (penalty, penalties) = penalty::calculate_penalty(&quartads, &curr_layout.layout, penalties, true);
				curr_layout.penalty = penalty;
				curr_layout.penalties = penalties;
				
				if curr_layout.penalty < bestLayout.penalty{
					bestLayout = BestLayoutsEntry::new(curr_layout.layout.clone(), (curr_layout.penalty, curr_layout.penalties.clone()));
				}
				// Probabilistically accept worse transitions; always accept better
				// transitions.
				if annealing::accept_transition((penalty - accepted_layout.penalty)/(len as f64),  i as usize) {
					
					accepted_layout = curr_layout.clone();
					
					
				}
				if i % 1000 == 0 {
					
					print_result(&bestLayout, len);
				}
			}
			print_result(&entry, len);
			bestLayout
		}).collect();
		for entry in iteration{
			//print_result(&entry.layout, entry.penalty, &entry.penalties, len);
			best_layouts.push(entry);
		}
		best_layouts.sort_unstable();
		best_layouts.truncate(BEST_LAYOUTS_KEPT as usize);
	}	
	println!("................................................");
	for entry in best_layouts{
		print_result(&entry, len);
	}
}
	
	
	
	/*
	pub fn refine<'a>(
		quartads:    &penalty::QuartadList<'a>,
	len:          usize,
	init_layout: &layout::Layout,
	penalties:   &Vec<penalty::KeyPenalty<'a>>,
	debug:        bool,
	top_layouts:  usize,
	num_swaps:    usize)
{
	let penalty = penalty::calculate_penalty(&quartads, len, init_layout, penalties, true);

	println!("Initial layout:");
	print_result(init_layout, &penalty);

	let mut curr_layout = init_layout.clone();
	let mut curr_penalty = penalty.1;

	loop {
		// Test every layout within `num_swaps` swaps of the initial layout.
		let mut best_layouts: Box<LL<BestLayoutsEntry>> = Box::new(LL{
			item : BestLayoutsEntry{
				layout: init_layout.clone(),
				penalty: penalty::calculate_penalty(&quartads, len, &init_layout, penalties, false).1
			},
			len:1,
			next : None
		});
		let permutations = layout::LayoutPermutations::new(init_layout, num_swaps);
		for (i, layout) in permutations.enumerate() {
			let penalty = penalty::calculate_penalty(&quartads, len, &layout, penalties, false);

			if debug {
				println!("Iteration {}: {}", i, penalty.1);
			}

			// Insert this layout into best layouts.
			let new_entry = BestLayoutsEntry {
				layout: layout,
				penalty: penalty.1,
			};
			best_layouts = list_insert_ordered(best_layouts, new_entry);

			// Limit best layouts list length.
			while best_layouts.len > top_layouts {
				best_layouts = best_layouts.next.unwrap();
			}
		}

		let mut lay= Some(best_layouts);
		// Print the top layouts.
		while let Some(ll )= lay{
			let entry = ll.item;
			let ref layout = entry.layout;
			let penalty = penalty::calculate_penalty(&quartads, len, &layout, penalties, true);
			println!("");
			print_result(&layout, &penalty);
			lay = ll.next;
		}

		// Keep going until swapping doesn't get us any more improvements.
		let best = best_layouts.item;
		if curr_penalty <= best.penalty {
			break;
		} else {
			curr_layout = best.layout;
			curr_penalty = best.penalty;
		}
	}

	println!("");
	println!("Ultimate winner:");
	println!("{}", curr_layout);
}
*/
pub fn print_result<'a>(
	item:&BestLayoutsEntry,
	len: usize)
{
	let layout = &item.layout;
	let total= item.penalty;
	let penalties = &item.penalties;
	let show_all = false;
	print!("{}{}{}{}{}{}",
		format!("\n{}\n", layout),
		format!("total: {0:<10.2}; scaled: {1:<10.4}\n", total, total/(len as f64)),
		//format!("base {}\n",penalties[0]),
		format!("\n{:<30} | {:<6} | {:<7} | {:<8} | {:<10}\n", "Name","% times", "Avg","% Total", "Total"),
		"----------------------------------------------------------------------\n",
		penalties
			.into_iter()
			.map(|penalty|{
				if penalty.show || show_all {
					format!("{:<30} | {:<6.2} | {:<7.3} | {:<8.4} | {:<10.0}\n", penalty.name,(100.0 * penalty.times / (len as f64)), penalty.total/ (len as f64), 100.0 * penalty.total/total, penalty.total)
				} else {"".to_string()}
			})
			.collect::<Vec<_>>()
			.join(""),
		"----------------------------------------------------------------------\n"
	);
}
/*
// Take ownership of the list and give it back as a hack to make the borrow checker happy :^)
fn list_insert_ordered(list: &mut Box<LL<BestLayoutsEntry>>, entry: BestLayoutsEntry)
{
	let mut cur = list;
	loop {
		if cur.item.cmp(&entry) == Ordering::Less{
			//std::mem::swap(&mut entry, cur.item)
			let tmp = BestLayoutsEntry{
				layout:  cur.item.layout.clone(), 
				penalty: cur.item.penalty
			};
			cur.item = entry;
			let entry = tmp;
			
			let mut node = Box::new(LL { 
				item : entry,
				len:cur.len,
				next : None
			});
			let rest = cur.next;
			node.next = rest;
			cur.next = Some(node);
			cur.len+=1;

			break;
		}
	}
	
}*/
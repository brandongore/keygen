extern crate num_cpus;
/// Applies the math in annealing.rs to keyboard layouts.
extern crate rand;
extern crate rayon;

use crate::annealing;
use crate::corpus_manager;
use crate::file_manager::save_run_state;
use crate::layout;
use crate::penalty;
use crate::timer::FuncTimer;
use crate::timer::Timer;
use crate::timer::TimerState;

//use self::rand::{random, thread_rng};
use self::rand::*;
use std::collections::HashMap;
//use std::collections::;
use self::rayon::prelude::*;
use std::*;

use penalty::*;

pub fn simulate<'a>(
    quartads: &corpus_manager::NgramList,
    init_layout: &layout::Layout,
    debug: bool,
    top_layouts: usize,
    num_swaps: usize,
    timer: &mut HashMap<String, TimerState>,
) {
    const CYCLES: i32 = 1;//205000;
    const ITERATIONS: i32 = 1;
    let threads = 1;//num_cpus::get();
    let BEST_LAYOUTS_KEPT: usize = 1;//threads * 2;
    let timer = &mut FuncTimer::new();

    // let mut initial_penalty = || penalty::calculate_penalty(&quartads, init_layout, timer);
     let mut best_layouts: Vec<BestLayoutsEntry> = Vec::new();
    //     (0..BEST_LAYOUTS_KEPT).map(|_| initial_penalty()).collect();

    // in each iteration each thread takes a random layout and tries to optimalize it for 5000 cycles;
    //results are appended to bestLayouts, which is then sorted and truntcated back to best ten
    for it_num in 1..ITERATIONS + 1 {
        timer.start(String::from(format!("iteration{}", it_num)));
        //prevent bad indexing when length of best layouts is 1
        let mut normalize_index: i32 = if best_layouts.len() > 1 { 1 } else { 0 };
        
        if best_layouts.len() > 1 { normalize_index = 1 } else { normalize_index = 0};
        println!("iteration: {}", it_num);
        let iteration: Vec<BestLayoutsEntry> = (0..threads)
            .map(|i| &best_layouts[best_layouts.len() -1 -(i * normalize_index as usize) as usize])
            .collect::<Vec<&BestLayoutsEntry>>()
            .into_par_iter()
            .map(|entry| {
                let mut accepted_layout = entry.clone();
                let mut bestLayout: BestLayoutsEntry = entry.clone();

                let printFrequency = thread_rng().gen::<i32>() % 5000 + 5000;

                for cycle in 1..CYCLES + 1 {
                    let mut curr_layout = accepted_layout.clone();
                    curr_layout
                        .layout
                        .shuffle(random::<usize>() % num_swaps + 1);

                    // Calculate penalty.
                    let timer2 = &mut FuncTimer::new();
                    // curr_layout = penalty::calculate_penalty(&quartads, &curr_layout.layout, timer2);

                    if curr_layout.penalty.total < bestLayout.penalty.total {
                        bestLayout = curr_layout.clone();
                    }
                    // Probabilistically accept worse transitions; always accept better
                    // transitions.
                    if annealing::accept_transition(
                        (curr_layout.penalty.total - accepted_layout.penalty.total) / (accepted_layout.penalty.total as f64),
                        cycle as usize,
                    ) {
                        accepted_layout = curr_layout.clone();
                    }
                    if cycle % printFrequency  == 0 {
                        //print_result(&bestLayout);
                    }
                }
                //print_result(&entry);
                bestLayout
            })
            .collect();
        for entry in iteration {
            //print_result(&entry.layout, entry.penalty, &entry.penalties, len);
            best_layouts.push(entry);
        }
        best_layouts.sort_unstable();
        best_layouts.truncate(BEST_LAYOUTS_KEPT as usize);
        timer.stop(String::from(format!("iteration{}", it_num)));
    }
    save_run_state(&best_layouts);

    println!("................................................");

    for entry in best_layouts {
        println!("************************************************");
        print_result(&entry);
        println!("************************************************");
    }
}

/*
    pub fn refine<'a>(
        quartads:    &penalty::NgramList<'a>,
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
pub fn print_result<'a>(item: &BestLayoutsEntry) {
    let layout = &item.layout;
    let total = item.penalty.total;
    let len = item.penalty.len;
    let penalties = &item.penalty.penalties;
    let penalty = &item.penalty;
    let fingers = &penalty.fingers;
    let show_all = false;
    print!(
        "{}{}{}{}{}{}{}{}{}",
        format!("\n{}\n", layout),
        format!(
            "total: {0:<10.2}; scaled: {1:<10.4}\n",
            total,
            total / (len as f64)
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
                        100.0 * penalty.total / total,
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

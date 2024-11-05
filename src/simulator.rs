extern crate num_cpus;
/// Applies the math in annealing.rs to keyboard layouts.
extern crate rand;
extern crate rayon;

use crate::annealing;
use crate::corpus_manager;
use crate::corpus_manager::NgramList;
use crate::file_manager::save_run_state;
use crate::layout;
use crate::layout::Layout;
use crate::layout::NUM_OF_KEYS;
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
    quartads: NgramList,
    init_layout: &layout::Layout,
    debug: bool,
    top_layouts: usize,
    num_swaps: usize,
    timer: &mut HashMap<String, TimerState>,
) {
    const CYCLES: i32 = 150000;
    const ITERATIONS: i32 = 5;
    let threads = num_cpus::get();
    let BEST_LAYOUTS_KEPT: usize = threads * 2;
    
    let processed_ngrams: Vec<(Vec<char>, usize)> = quartads
    .map
    .into_iter()
    .map(|item| (item.0.chars().collect(), item.1))
    .collect();

    let mut initial_penalty = |shuffle:bool| {
        let mut layout = init_layout.clone();
        if shuffle {
            layout.shuffle(random::<usize>() % num_swaps + 2);
        }
        penalty::calculate_penalty(&processed_ngrams, &layout)
    };
    println!("initial:\r\n");
    print_result(&initial_penalty(false));

    let mut best_layouts: Vec<BestLayoutsEntry> = //Vec::new();
         (0..BEST_LAYOUTS_KEPT).map(|_| initial_penalty(true)).collect();

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

                let printFrequency = thread_rng().gen::<i32>() % 100000 + 100000;

                for cycle in 1..CYCLES + 1 {
                    let mut curr_layout = accepted_layout.clone();
                    curr_layout
                        .layout
                        .shuffle(random::<usize>() % num_swaps + 1);

                    // Calculate penalty.
                    curr_layout = penalty::calculate_penalty(&processed_ngrams, &curr_layout.layout);

                    if better_than_average_including_bad(&curr_layout.penalty, &bestLayout.penalty) {
                        bestLayout = curr_layout.clone();
                    }

                    // if curr_layout.penalty.bad_score_total < bestLayout.penalty.bad_score_total {
                    //     bestLayout = curr_layout.clone();
                    // }
                    // Probabilistically accept worse transitions; always accept better
                    // transitions.
                    if annealing::accept_transition(
                        (curr_layout.penalty.total - accepted_layout.penalty.total) / (accepted_layout.penalty.total as f64),
                        cycle as usize,
                    ) {
                        accepted_layout = curr_layout.clone();
                    }
                     if cycle % printFrequency  == 0 {
                    //     println!("BESTBESTBESTBESTBESTBESTBESTBESTBESTBESTBEST");
                           print_result(&bestLayout);
                    //     println!("BESTBESTBESTBESTBESTBESTBESTBESTBESTBESTBEST");
                    //     println!("CURRCURRCURRCURRCURRCURRCURRCURRCURRCURRCURR");
                    //     print_result(&curr_layout);
                    //     println!("CURRCURRCURRCURRCURRCURRCURRCURRCURRCURRCURR");
                     }
                }
                //print_result(&entry);
                bestLayout
            })
            .collect();
        for entry in iteration {
            //print_result(&entry.layout, entry.penalty, &entry.penalties, len);
            if(!entry.layout.0.to_string().eq(&init_layout.0.to_string())){
                best_layouts.push(entry);
            }
        }
        //best_layouts.sort_by(|a,b| a.penalty.penalties[1].total.partial_cmp(&b.penalty.penalties[1].total).unwrap());
        best_layouts.sort_unstable();
        best_layouts.truncate(BEST_LAYOUTS_KEPT as usize);
        timer.stop(String::from(format!("iteration{}", it_num)));
    }

    save_run_state(&best_layouts);

    //println!("................................................");

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
    let total = item.penalty.total;
    let absolute_total = bad_score_total.abs() + good_score_total.abs();
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
        position_working[i] = (penalty * 100.0) as u128;
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
        format!("{:^5.1}| {:^5.1}\n", penalty.hands[0] as f64 * 100.0 / len as f64 , penalty.hands[1] as f64 * 100.0 / len as f64 ),
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
                        100.0 * penalty.total / absolute_total,
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
            fingers[8] as f64 * 100.0 / len as f64 ,
            fingers[7] as f64 * 100.0 / len as f64 ,
            fingers[6] as f64 * 100.0 / len as f64 ,
            fingers[5] as f64 * 100.0 / len as f64 
        ),
        format!(
            "\n\t\t{:^5.1}\t|\t{:^5.1}\t\t\n",
            fingers[4] as f64 * 100.0 / len as f64 ,
            fingers[9] as f64 * 100.0 / len as f64
        ),
        "##########################################################################\n"
    );
}


// pub fn print_result<'a>(item: &BestLayoutsEntry) {
//     let layout = &item.layout;
//     let total = item.penalty.total;
//     let len = item.penalty.len;
//     let penalties = &item.penalty.penalties;
//     let penalty = &item.penalty;
//     let fingers = &penalty.fingers;
//     let show_all = false;
//     print!(
//         "{}{}{}{}{}{}{}{}{}",
//         format!("\n{}\n", layout),
//         format!(
//             "total: {0:<10.2}; scaled: {1:<10.4}\n",
//             total,
//             total / (len as f64)
//         ),
//         //format!("base {}\n",penalties[0]),
//         format!(
//             "\n{:<30} | {:^7} | {:^7} | {:^8} | {:<10}\n",
//             "Name", "% times", "Avg", "% Total", "Total"
//         ),
//         "----------------------------------------------------------------------\n",
//         penalties
//             .into_iter()
//             .map(|penalty| {
//                 if penalty.show || show_all {
//                     format!(
//                         "{:<30} | {:<7.2} | {:<7.3} | {:<8.3} | {:<10.0}\n",
//                         penalty.name,
//                         (100.0 * penalty.times as f64 / (len as f64)),
//                         penalty.total / (len as f64),
//                         100.0 * penalty.total / total,
//                         penalty.total
//                     )
//                 } else {
//                     "".to_string()
//                 }
//             })
//             .collect::<Vec<_>>()
//             .join(""),
//         "----------------------------------------------------------------------\n",
//         format!(
//             "\n{:^5.1} {:^5.1} {:^5.1} {:^5.1} | {:^5.1} {:^5.1} {:^5.1} {:^5.1}\n",
//             fingers[0] as f64 * 100.0 / len as f64 ,
//             fingers[1] as f64 * 100.0 / len as f64 ,
//             fingers[2] as f64 * 100.0 / len as f64 ,
//             fingers[3] as f64 * 100.0 / len as f64 ,
//             fingers[7] as f64 * 100.0 / len as f64 ,
//             fingers[6] as f64 * 100.0 / len as f64 ,
//             fingers[5] as f64 * 100.0 / len as f64 ,
//             fingers[4] as f64 * 100.0 / len as f64 
//         ),

//         format!("{:^5.1}| {:^5.1}\n", penalty.hands[0] as f64 * 100.0 / len as f64 , penalty.hands[1] as f64 * 100.0 / len as f64 ),
//         "##########################################################################\n"
//     );
// }
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

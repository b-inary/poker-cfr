mod cfr;
mod game_kuhn;
mod game_push_fold;

use game_kuhn::KuhnNode;
use std::cmp::{max, min};
use std::collections::BTreeMap;

fn main() {
    kuhn();
    push_fold(10.0);
}

fn kuhn() {
    let kuhn_node = KuhnNode::new();
    let strategy = cfr::train(&kuhn_node, 100000)
        .into_iter()
        .map(|(key, value)| (KuhnNode::public_info_set_str(&key), value))
        .collect::<BTreeMap<_, _>>();

    println!("[Kuhn poker] (left: check/fold%, right: bet/call%)");
    for (key, value) in strategy {
        println!("- {}", key);
        for i in 0..3 {
            println!(
                "    {}: {:.2}%, {:.2}%",
                ["J", "Q", "K"][i],
                100.0 * value[0][i],
                100.0 * value[1][i],
            );
        }
    }
}

fn push_fold(eff_stack: f64) {
    let push_fold_node = game_push_fold::PushFoldNode::new(eff_stack);
    let strategy = cfr::train(&push_fold_node, 10000);
    let pusher = &strategy[&vec![]];
    let caller = &strategy[&vec![1]];

    let mut push_rate = vec![vec![0.0; 13]; 13];
    let mut call_rate = vec![vec![0.0; 13]; 13];

    let mut k = 0;
    for i in 0..51 {
        for j in (i + 1)..52 {
            let rank1 = i / 4;
            let rank2 = j / 4;
            let suit1 = i % 4;
            let suit2 = j % 4;
            if suit1 == suit2 {
                push_rate[min(rank1, rank2)][max(rank1, rank2)] += pusher[1][k];
                call_rate[min(rank1, rank2)][max(rank1, rank2)] += caller[1][k];
            } else {
                push_rate[max(rank1, rank2)][min(rank1, rank2)] += pusher[1][k];
                call_rate[max(rank1, rank2)][min(rank1, rank2)] += caller[1][k];
            }
            k += 1;
        }
    }

    for i in 0..13 {
        for j in 0..13 {
            let count = if i == j {
                6.0
            } else if i < j {
                4.0
            } else {
                12.0
            };
            push_rate[i][j] /= count;
            call_rate[i][j] /= count;
        }
    }

    println!();
    println!(
        "[Push/Fold heads-up hold'em] (effective stack = {}bb)",
        eff_stack
    );
    println!("Pusher (small blind):");
    println!(" |   A     K     Q     J     T     9     8     7     6     5     4     3     2");
    println!("-+------------------------------------------------------------------------------");
    for i in 0..13 {
        let rank1 = 12 - i;
        print!(
            "{}|",
            ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"][rank1]
        );
        for j in 0..13 {
            let rank2 = 12 - j;
            if push_rate[rank2][rank1] >= 0.9995 {
                print!(" 100.%");
            } else if push_rate[rank2][rank1] < 0.0005 {
                print!("   -  ");
            } else {
                print!(" {:>4.1}%", 100.0 * push_rate[rank2][rank1]);
            }
        }
        println!();
    }

    println!();
    println!("Caller (big blind):");
    println!(" |   A     K     Q     J     T     9     8     7     6     5     4     3     2");
    println!("-+------------------------------------------------------------------------------");
    for i in 0..13 {
        let rank1 = 12 - i;
        print!(
            "{}|",
            ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"][rank1]
        );
        for j in 0..13 {
            let rank2 = 12 - j;
            if call_rate[rank2][rank1] >= 0.9995 {
                print!(" 100.%");
            } else if call_rate[rank2][rank1] < 0.0005 {
                print!("   -  ");
            } else {
                print!(" {:>4.1}%", 100.0 * call_rate[rank2][rank1]);
            }
        }
        println!();
    }
}

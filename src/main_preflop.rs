#[allow(dead_code)]
mod cfr;

mod game_node;
mod game_preflop;

use game_preflop::PreflopNode;
use std::cmp::{max, min};

fn main() {
    preflop(10.0, 1000);
}

fn preflop(eff_stack: f64, num_iter: usize) {
    let push_fold_node = PreflopNode::new(eff_stack);
    let (strategy, ev, exploitability) = cfr::train_mt(&push_fold_node, num_iter, true);
    let bn_strategy = &strategy[&vec![]];

    let mut bn_rate = vec![vec![vec![0.0; 13]; 13]; 5];
    let mut bb_rate = vec![vec![vec![vec![0.0; 13]; 13]; 5]; 4];
    let mut bn_limp_rate = vec![vec![vec![vec![0.0; 13]; 13]; 5]; 3];
    let mut overall_bn_rate = vec![0.0; 5];
    let mut overall_bb_rate = vec![vec![0.0; 5]; 4];
    let mut overall_bn_limp_rate = vec![vec![0.0; 5]; 3];

    for action in 0..5 {
        let mut k = 0;
        for i in 0..51 {
            for j in (i + 1)..52 {
                let rank1 = i / 4;
                let rank2 = j / 4;
                let suit1 = i % 4;
                let suit2 = j % 4;
                if suit1 == suit2 {
                    bn_rate[action][min(rank1, rank2)][max(rank1, rank2)] += bn_strategy[action][k];
                } else {
                    bn_rate[action][max(rank1, rank2)][min(rank1, rank2)] += bn_strategy[action][k];
                }
                overall_bn_rate[action] += bn_strategy[action][k] / (52.0 * 51.0 / 2.0);
                k += 1;
            }
        }
    }

    for bn_action in 0..4 {
        let bb_strategy = &strategy[&vec![(bn_action + 1) as u8]];
        let len = bb_strategy.len();
        for bb_action in 0..len {
            let mut k = 0;
            for i in 0..51 {
                for j in (i + 1)..52 {
                    let rank1 = i / 4;
                    let rank2 = j / 4;
                    let suit1 = i % 4;
                    let suit2 = j % 4;
                    if suit1 == suit2 {
                        bb_rate[bn_action][bb_action][min(rank1, rank2)][max(rank1, rank2)] +=
                            bb_strategy[bb_action][k];
                    } else {
                        bb_rate[bn_action][bb_action][max(rank1, rank2)][min(rank1, rank2)] +=
                            bb_strategy[bb_action][k];
                    }
                    overall_bb_rate[bn_action][bb_action] +=
                        bb_strategy[bb_action][k] / (52.0 * 51.0 / 2.0);
                    k += 1;
                }
            }
        }
    }

    for bb_action in 0..3 {
        let bn_strategy = &strategy[&vec![1, (bb_action + 2) as u8]];
        let len = bn_strategy.len();
        for bn_action in 0..len {
            let mut k = 0;
            for i in 0..51 {
                for j in (i + 1)..52 {
                    let rank1 = i / 4;
                    let rank2 = j / 4;
                    let suit1 = i % 4;
                    let suit2 = j % 4;
                    if suit1 == suit2 {
                        bn_limp_rate[bb_action][bn_action][min(rank1, rank2)][max(rank1, rank2)] +=
                            bn_strategy[bn_action][k];
                    } else {
                        bn_limp_rate[bb_action][bn_action][max(rank1, rank2)][min(rank1, rank2)] +=
                            bn_strategy[bn_action][k];
                    }
                    overall_bn_limp_rate[bb_action][bn_action] +=
                        bn_strategy[bn_action][k] / (52.0 * 51.0 / 2.0);
                    k += 1;
                }
            }
        }
    }

    for action in 0..5 {
        for i in 0..13 {
            for j in 0..13 {
                let count = if i == j {
                    6.0
                } else if i < j {
                    4.0
                } else {
                    12.0
                };
                bn_rate[action][i][j] /= count;
            }
        }
    }

    for bn_action in 0..4 {
        for bb_action in 0..5 {
            for i in 0..13 {
                for j in 0..13 {
                    let count = if i == j {
                        6.0
                    } else if i < j {
                        4.0
                    } else {
                        12.0
                    };
                    bb_rate[bn_action][bb_action][i][j] /= count;
                }
            }
        }
    }

    for bb_action in 0..3 {
        for bn_action in 0..5 {
            for i in 0..13 {
                for j in 0..13 {
                    let count = if i == j {
                        6.0
                    } else if i < j {
                        4.0
                    } else {
                        12.0
                    };
                    bn_limp_rate[bb_action][bn_action][i][j] /= count;
                }
            }
        }
    }

    println!();
    println!(
        "[Pre-flop only heads-up hold'em] (effective stack = {}bb)",
        eff_stack
    );
    println!("- Exploitability: {:+.3e}[bb]", exploitability);
    println!();

    println!("BN (small blind):");
    println!("- EV = {:+.4}[bb]", ev);
    for action in 0..5 {
        println!();
        println!(
            "[{}%]",
            ["Fold", "Limp", "3x Bet", "4x Bet", "All in"][action]
        );
        println!("Overall rate = {:.2}%", 100.0 * overall_bn_rate[action]);
        println!(" |   A     K     Q     J     T     9     8     7     6     5     4     3     2");
        println!(
            "-+------------------------------------------------------------------------------"
        );
        for i in 0..13 {
            let rank1 = 12 - i;
            print!(
                "{}|",
                ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"][rank1]
            );
            for j in 0..13 {
                let rank2 = 12 - j;
                match ((bn_rate[action][rank2][rank1] + 0.1) * 5.0) as usize {
                    0 => print!(" \x1b[37m  -  \x1b[37m"),
                    1 => print!(" \x1b[91m*    \x1b[37m"),
                    2 => print!(" \x1b[95m**   \x1b[37m"),
                    3 => print!(" \x1b[94m***  \x1b[37m"),
                    4 => print!(" \x1b[96m**** \x1b[37m"),
                    5 => print!(" \x1b[92m*****\x1b[37m"),
                    _ => unreachable!(),
                }
            }
            println!();
        }
    }
    println!();

    println!("BB (big blind):");
    println!("- EV = {:+.4}[bb]", -ev);
    for bn_action in 0..4 {
        for bb_action in 0..5 {
            println!();
            println!(
                "[{} => {}%]",
                ["Limp", "3x Bet", "4x Bet", "All in"][bn_action],
                ["Fold", "Call", "3x Bet", "4x Bet", "All in"][bb_action],
            );
            println!(
                "Overall rate = {:.2}%",
                100.0 * overall_bb_rate[bn_action][bb_action]
            );
            println!(
                " |   A     K     Q     J     T     9     8     7     6     5     4     3     2"
            );
            println!(
                "-+------------------------------------------------------------------------------"
            );
            for i in 0..13 {
                let rank1 = 12 - i;
                print!(
                    "{}|",
                    ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"][rank1]
                );
                for j in 0..13 {
                    let rank2 = 12 - j;
                    match ((bb_rate[bn_action][bb_action][rank2][rank1] + 0.1) * 5.0) as usize {
                        0 => print!(" \x1b[37m  -  \x1b[37m"),
                        1 => print!(" \x1b[91m*    \x1b[37m"),
                        2 => print!(" \x1b[95m**   \x1b[37m"),
                        3 => print!(" \x1b[94m***  \x1b[37m"),
                        4 => print!(" \x1b[96m**** \x1b[37m"),
                        5 => print!(" \x1b[92m*****\x1b[37m"),
                        _ => unreachable!(),
                    }
                }
                println!();
            }
        }
    }
    println!();

    println!("BN (small blind) [Limp]:");
    for bb_action in 0..3 {
        for bn_action in 0..5 {
            println!();
            println!(
                "[Limp => {} => {}%]",
                ["3x Bet", "4x Bet", "All in"][bb_action],
                ["Fold", "Call", "3x Bet", "4x Bet", "All in"][bn_action],
            );
            println!(
                "Overall rate = {:.2}%",
                100.0 * overall_bn_limp_rate[bb_action][bn_action]
            );
            println!(
                " |   A     K     Q     J     T     9     8     7     6     5     4     3     2"
            );
            println!(
                "-+------------------------------------------------------------------------------"
            );
            for i in 0..13 {
                let rank1 = 12 - i;
                print!(
                    "{}|",
                    ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"][rank1]
                );
                for j in 0..13 {
                    let rank2 = 12 - j;
                    match ((bn_limp_rate[bb_action][bn_action][rank2][rank1] + 0.1) * 5.0) as usize
                    {
                        0 => print!(" \x1b[37m  -  \x1b[37m"),
                        1 => print!(" \x1b[91m*    \x1b[37m"),
                        2 => print!(" \x1b[95m**   \x1b[37m"),
                        3 => print!(" \x1b[94m***  \x1b[37m"),
                        4 => print!(" \x1b[96m**** \x1b[37m"),
                        5 => print!(" \x1b[92m*****\x1b[37m"),
                        _ => unreachable!(),
                    }
                }
                println!();
            }
        }
    }
}

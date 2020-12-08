mod cfr;
mod game_kuhn;
mod game_node;
mod game_preflop;
mod game_push_fold;

use game_kuhn::KuhnNode;
use game_preflop::PreflopNode;
use game_push_fold::PushFoldNode;
use std::cmp::{max, min};
use std::collections::BTreeMap;

fn main() {
    kuhn(100000);
    push_fold(10.0, 10000);
    preflop(10.0, 1000);
}

fn kuhn(num_iter: usize) {
    let kuhn_node = KuhnNode::new();
    let (strategy, ev, exploitability) = cfr::train(&kuhn_node, num_iter, false);
    let strategy = strategy
        .into_iter()
        .map(|(key, value)| (KuhnNode::public_info_set_str(&key), value))
        .collect::<BTreeMap<_, _>>();

    println!();
    println!("[Kuhn poker]");
    println!("- Exploitability: {:+.3e}", exploitability);
    println!("- EV of first player: {:+.4}", ev);
    println!("- EV of second player: {:+.4}", -ev);
    println!();
    println!("(left: check/fold%, right: bet/call%)");
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

fn push_fold(eff_stack: f64, num_iter: usize) {
    let push_fold_node = PushFoldNode::new(eff_stack);
    let (strategy, ev, exploitability) = cfr::train(&push_fold_node, num_iter, false);
    let pusher = &strategy[&vec![]];
    let caller = &strategy[&vec![1]];

    let mut push_rate = vec![vec![0.0; 13]; 13];
    let mut call_rate = vec![vec![0.0; 13]; 13];
    let mut overall_push_rate = 0.0;
    let mut overall_call_rate = 0.0;

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
            overall_push_rate += pusher[1][k];
            overall_call_rate += caller[1][k];
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

    overall_push_rate /= 52.0 * 51.0 / 2.0;
    overall_call_rate /= 52.0 * 51.0 / 2.0;

    println!();
    println!(
        "[Push/Fold heads-up hold'em] (effective stack = {}bb)",
        eff_stack
    );
    println!("- Exploitability: {:+.3e}[bb]", exploitability);
    println!();
    println!("Pusher (small blind):");
    println!("- EV = {:+.4}[bb]", ev);
    println!("- Overall push rate = {:.2}%", 100.0 * overall_push_rate);
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
    println!("Caller (big blind): ");
    println!("- EV = {:+.4}[bb]", -ev);
    println!("- Overall call rate = {:.2}%", 100.0 * overall_call_rate);
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

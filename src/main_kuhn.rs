#[allow(dead_code)]
mod cfr;

mod game_kuhn;
mod game_node;

use game_kuhn::KuhnNode;
use std::collections::BTreeMap;

fn main() {
    kuhn(100000);
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

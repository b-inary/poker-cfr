mod cfr;
mod game_kuhn;

use std::collections::BTreeMap;

fn main() {
    let kuhn_node = game_kuhn::KuhnNode::new();
    let strategy = cfr::train(&kuhn_node, 100000);

    // display information of KuhnNode
    let strategy = strategy.into_iter().collect::<BTreeMap<_, _>>();
    for (key, value) in strategy {
        println!("[{}]", key);
        for i in 0..3 {
            println!(
                "{}: {:.2}%, {:.2}%",
                ["J", "Q", "K"][i],
                100.0 * value[0][i],
                100.0 * value[1][i],
            );
        }
    }
}

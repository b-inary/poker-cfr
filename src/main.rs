mod cfr;
mod game_kuhn;

fn main() {
    let kuhn_node = game_kuhn::KuhnNode::new();
    cfr::train(&kuhn_node, 100000);
}

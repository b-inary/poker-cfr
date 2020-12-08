#[allow(dead_code)]
mod cfr;

mod game_node;
mod game_preflop;

use bincode::{deserialize, serialize};
use game_preflop::PreflopNode;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Result, Write};
use std::path::Path;

fn main() -> Result<()> {
    let eff_stack = 10.0;
    let num_iter = 1000;

    let path = format!("output/preflop-{}-{}.bin", eff_stack, num_iter);
    let file_exists = Path::new(&path).exists();

    println!(
        "[Pre-flop only heads-up hold'em] (effective stack = {}bb)",
        eff_stack
    );

    let (strategy, ev, exploitability) = if file_exists {
        let mut infile = File::open(&path)?;
        let mut buf = Vec::new();
        infile.read_to_end(&mut buf)?;
        deserialize::<(HashMap<Vec<u8>, Vec<Vec<f64>>>, f64, f64)>(&buf).unwrap()
    } else {
        let push_fold_node = PreflopNode::new(eff_stack);
        cfr::train_mt(&push_fold_node, num_iter, true)
    };

    println!("- Exploitability: {:+.3e}[bb]", exploitability);
    println!("- EV of SB: {:+.4}[bb]", ev);
    println!("- EV of BB: {:+.4}[bb]", -ev);

    if !file_exists {
        let encoded = serialize(&(strategy, ev, exploitability)).unwrap();
        let mut outfile = File::create(&path)?;
        outfile.write_all(&encoded)?;
        println!("Wrote results to '{}'", &path);
    }

    Ok(())
}

#[allow(dead_code)]
mod cfr;

mod game_node;
mod game_preflop;

use bincode::{deserialize, serialize};
use clap::Clap;
use game_preflop::PreflopNode;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Result, Write};
use std::path::Path;

#[derive(Clap)]
#[clap(version = "0.1.0", author = "Wataru Inariba <oinari17@gmail.com>")]
struct Opts {
    #[clap(short, long, default_value = "10.0")]
    stack: f64,
    #[clap(short, long, default_value = "1000")]
    iteration: usize,
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    let path = format!("output/preflop-{}-{}.bin", opts.stack, opts.iteration);
    let file_exists = Path::new(&path).exists();

    println!(
        "[Pre-flop only heads-up hold'em] (effective stack = {}bb)",
        opts.stack
    );

    let (strategy, ev, exploitability) = if file_exists {
        let mut infile = File::open(&path)?;
        let mut buf = Vec::new();
        infile.read_to_end(&mut buf)?;
        deserialize::<(HashMap<Vec<u8>, Vec<Vec<f64>>>, f64, f64)>(&buf).unwrap()
    } else {
        let push_fold_node = PreflopNode::new(opts.stack);
        cfr::train_mt(&push_fold_node, opts.iteration, true)
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

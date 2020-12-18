#[allow(dead_code)]
mod cfr;

mod game_node;
mod game_preflop;

use bincode::{deserialize, serialize};
use clap::Clap;
use game_node::PublicInfoSet;
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

    let (_, ev, exploitability) = if file_exists {
        let mut infile = File::open(&path)?;
        let mut buf = Vec::new();
        infile.read_to_end(&mut buf)?;
        deserialize::<(HashMap<PublicInfoSet, Vec<Vec<Vec<f64>>>>, f64, f64)>(&buf).unwrap()
    } else {
        let push_fold_node = PreflopNode::new(opts.stack);
        let (raw_strategy, ev, exploitability) = cfr::train_mt(
            &push_fold_node,
            opts.iteration,
            true,
            Some((
                |iter| format!("output/preflop-{}-{}.bin", opts.stack, iter),
                summarize_strategy,
            )),
        );
        let converted = raw_strategy
            .iter()
            .map(|(key, value)| (key.clone(), summarize_strategy(value)))
            .collect::<HashMap<_, _>>();
        let encoded = serialize(&(converted, ev, exploitability)).unwrap();
        let mut outfile = File::create(&path)?;
        outfile.write_all(&encoded)?;
        println!("Wrote results to '{}'", &path);
        (HashMap::new(), ev, exploitability)
    };

    println!("- Exploitability: {:+.3e}[bb]", exploitability);
    println!("- EV of SB: {:+.4}[bb]", ev);
    println!("- EV of BB: {:+.4}[bb]", -ev);

    Ok(())
}

fn summarize_strategy(strategy: &Vec<Vec<f64>>) -> Vec<Vec<Vec<f64>>> {
    let num_actions = strategy.len();
    let mut summarized = vec![vec![vec![0.0; 13]; 13]; num_actions];

    for action in 0..num_actions {
        let mut k = 0;
        for i in 0..51 {
            for j in (i + 1)..52 {
                if i % 4 == j % 4 {
                    summarized[action][i / 4][j / 4] += strategy[action][k];
                } else {
                    summarized[action][j / 4][i / 4] += strategy[action][k];
                }
                k += 1;
            }
        }

        for i in 0..13 {
            for j in 0..13 {
                let count = [12.0, 4.0, 6.0][(i <= j) as usize + (i == j) as usize];
                summarized[action][i][j] /= count;
            }
        }
    }

    summarized
}

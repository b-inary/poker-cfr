#[allow(dead_code)]
mod cfr;
mod game_node;
mod game_preflop;

use bincode::deserialize;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute, queue, style, terminal,
};
use game_node::PublicInfoSet;
use ordered_float::NotNan;
use regex::Regex;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};

type OutputType = (HashMap<PublicInfoSet, Vec<Vec<Vec<f64>>>>, f64, f64);
type EvType = HashMap<PublicInfoSet, Vec<Vec<f64>>>;

fn main() {
    println!("Loading files. Please wait...");
    let outputs = get_outputs().unwrap();
    if outputs.is_empty() {
        println!("Error: 'output' directory is empty.");
        println!("Please run:");
        println!();
        println!("$ cargo run --release --bin preflop");
        println!();
        return;
    }
    interactive_display(&outputs).unwrap();
}

fn get_outputs() -> std::io::Result<Vec<(NotNan<f64>, (OutputType, EvType, EvType))>> {
    let output_path = "output/";
    let re = Regex::new(r"^preflop-(\d+.?\d*)-(\d+).bin$").unwrap();
    let mut outputs = BTreeMap::new();
    let mut iterations = BTreeMap::new();

    for entry in fs::read_dir(output_path)? {
        let path = entry?.path();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

        if let Some(cs) = re.captures(&file_name) {
            let stack = cs.get(1).unwrap().as_str().parse::<NotNan<f64>>().unwrap();
            let iteration = cs.get(2).unwrap().as_str().parse::<usize>().unwrap();

            if !iterations.contains_key(&stack) || iteration > iterations[&stack] {
                let mut infile = File::open(path)?;
                let mut buf = Vec::new();
                infile.read_to_end(&mut buf).unwrap();
                let decoded = deserialize::<OutputType>(&buf).unwrap();
                let ev0 = calc_ev(&decoded.0, 0, stack.into_inner());
                let ev1 = calc_ev(&decoded.0, 1, stack.into_inner());
                outputs.insert(stack, (decoded, ev0, ev1));
                iterations.insert(stack, iteration);
            }
        }
    }

    let outputs = outputs.into_iter().collect::<Vec<_>>();
    Ok(outputs)
}

fn interactive_display(
    outputs: &Vec<(NotNan<f64>, (OutputType, EvType, EvType))>,
) -> crossterm::Result<()> {
    let mut stdout = std::io::stdout();

    terminal::enable_raw_mode()?;

    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        event::EnableMouseCapture,
        cursor::Hide,
    )?;

    let (terminal_width, terminal_height) = terminal::size()?;
    let mut files_top_index = 0;
    let mut indices = vec![0];
    let mut num_indices = vec![outputs.len()];
    let mut output: &OutputType = &(HashMap::new(), 0.0, 0.0);
    let mut ev0_table: &EvType = &HashMap::new();
    let mut ev1_table: &EvType = &HashMap::new();
    let mut display_mode = 0;
    let mut multiple_select: HashSet<usize> = HashSet::new();

    loop {
        queue!(
            stdout,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0),
            style::Print(
                "'q': Quit / 't': Toggle content / Space: Multi-select / Arrow keys: Move cursor"
            ),
        )?;

        let (col_display_start, offset) = if 23 * indices.len() - 15 > terminal_width as usize {
            queue!(stdout, cursor::MoveTo(0, 5), style::Print("(omitted)"))?;
            let col_display_start = indices.len() - (terminal_width as usize - 10) / 23;
            (col_display_start, 23 * col_display_start - 10)
        } else {
            (1, 15)
        };

        if col_display_start == 1 {
            for (i, (stack, _)) in outputs.iter().enumerate() {
                if i < files_top_index || files_top_index + 6 < i {
                    continue;
                }
                queue!(stdout, cursor::MoveTo(0, (i - files_top_index + 2) as u16))?;
                if files_top_index > 0 && i == files_top_index {
                    queue!(stdout, style::Print("    ^  "))?;
                    continue;
                }
                if i + 1 < outputs.len() && i == files_top_index + 6 {
                    queue!(stdout, style::Print("    v  "))?;
                    continue;
                }
                if i == indices[0] {
                    queue!(stdout, style::SetAttribute(style::Attribute::Bold))?;
                }
                if i == indices[0] && indices.len() == 1 {
                    queue!(stdout, style::SetAttribute(style::Attribute::Underlined))?;
                }
                queue!(
                    stdout,
                    style::Print(format!(
                        "{} {:>3}bb",
                        [' ', '*'][(i == indices[0]) as usize],
                        stack
                    )),
                    style::SetAttribute(style::Attribute::Reset),
                )?;
            }
        }

        let mut cur_rate = [vec![vec![1.0; 13]; 13], vec![vec![1.0; 13]; 13]];

        for i in 1..indices.len() {
            let key = indices[1..i].iter().map(|x| *x as u8).collect::<Vec<_>>();
            let strategy = &output.0[&key];
            let avg_rate = compute_average_rate(&cur_rate[i % 2], &strategy);

            num_indices[i] = strategy.len();

            if col_display_start <= i {
                let arrow_y = indices[i - 1] - [0, files_top_index][(i == 1) as usize];
                queue!(
                    stdout,
                    cursor::MoveTo((23 * i - offset) as u16, arrow_y as u16 + 2),
                    style::SetAttribute(style::Attribute::Bold),
                    style::Print("=>"),
                    style::SetAttribute(style::Attribute::Reset),
                )?;

                for j in 0..num_indices[i] {
                    let action = if j >= 2 && j + 1 == num_indices[i] {
                        "All-in"
                    } else if (i, j) == (2, 1) && indices[1] == 1 {
                        "Check"
                    } else {
                        ["Fold", "Call", "Bet 2.5x", "Bet 3x", "Bet 3.5x", "Bet 4x"][j]
                    };
                    if j == indices[i] {
                        queue!(stdout, style::SetAttribute(style::Attribute::Bold))?;
                    }
                    if j == indices[i] && i + 1 == indices.len() {
                        queue!(stdout, style::SetAttribute(style::Attribute::Underlined))?;
                    }
                    queue!(
                        stdout,
                        cursor::MoveTo((23 * i - offset + 2) as u16, j as u16 + 2),
                        style::Print(format!(
                            "{}{}{}[{:>5.2}%] {}",
                            [' ', '(']
                                [(i + 1 == indices.len() && multiple_select.contains(&j)) as usize],
                            [' ', '*'][(j == indices[i]
                                || (i + 1 == indices.len()
                                    && multiple_select.contains(&indices[i])
                                    && multiple_select.contains(&j)))
                                as usize],
                            [' ', ')']
                                [(i + 1 == indices.len() && multiple_select.contains(&j)) as usize],
                            100.0 * avg_rate[j],
                            action
                        )),
                        style::SetAttribute(style::Attribute::Reset),
                    )?;
                }
            }

            if i + 1 == indices.len() && multiple_select.contains(&indices[i]) {
                for j in 0..13 {
                    for k in 0..13 {
                        let mut tmp = 0.0;
                        for idx in &multiple_select {
                            tmp += strategy[*idx][j][k];
                        }
                        cur_rate[i % 2][j][k] *= tmp;
                    }
                }
            } else {
                for j in 0..13 {
                    for k in 0..13 {
                        cur_rate[i % 2][j][k] *= strategy[indices[i]][j][k];
                    }
                }
            }
        }

        if indices.len() >= 2 {
            let player = (indices.len() - 1) % 2;
            let opponent = 1 - player;
            let denom = calc_denom(&cur_rate[opponent]);

            let mut ev = vec![vec![0.0; 13]; 13];
            let mut key = indices[1..].iter().map(|x| *x as u8).collect::<Vec<_>>();
            if multiple_select.contains(indices.last().unwrap()) {
                for idx in &multiple_select {
                    *key.last_mut().unwrap() = *idx as u8;
                    let tmp = &[ev1_table, ev0_table][player][&key];
                    for j in 0..13 {
                        for k in 0..13 {
                            ev[j][k] += tmp[j][k];
                        }
                    }
                }
            } else {
                ev = [ev1_table, ev0_table][player][&key].clone();
            }

            for j in 0..13 {
                for k in 0..13 {
                    ev[j][k] /= cur_rate[player][j][k] * denom[j][k];
                }
            }

            queue!(
                stdout,
                cursor::MoveTo(0, 10),
                style::Print(" |   A     K     Q     J     T     9     8     7     6     5     4     3     2"),
                cursor::MoveTo(0, 11),
                style::Print("-+------------------------------------------------------------------------------"),
            )?;

            for i in 0..13 {
                let rank1 = 12 - i;
                queue!(stdout, cursor::MoveTo(0, 12 + i as u16))?;
                if i % 2 == 1 {
                    queue!(
                        stdout,
                        style::SetBackgroundColor(style::Color::AnsiValue(235)),
                    )?;
                }
                queue!(
                    stdout,
                    style::Print(format!(
                        "{}|",
                        ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"][rank1]
                    ))
                )?;

                for j in 0..13 {
                    let rank2 = 12 - j;

                    match display_mode {
                        0 => {
                            match (cur_rate[(indices.len() + 1) % 2][rank2][rank1] * 3.0 + 0.5)
                                as usize
                            {
                                0 => queue!(
                                    stdout,
                                    style::SetForegroundColor(style::Color::Reset),
                                    style::Print("   -  ")
                                )?,
                                1 => queue!(
                                    stdout,
                                    style::SetForegroundColor(style::Color::Yellow),
                                    style::Print("   *  ")
                                )?,
                                2 => queue!(
                                    stdout,
                                    style::SetForegroundColor(style::Color::Cyan),
                                    style::Print("  * * ")
                                )?,
                                3 => queue!(
                                    stdout,
                                    style::SetForegroundColor(style::Color::Green),
                                    style::Print("  *** ")
                                )?,
                                _ => unreachable!(),
                            }
                        }

                        1 => {
                            match (cur_rate[(indices.len() + 1) % 2][rank2][rank1] * 3.0 + 0.5)
                                as usize
                            {
                                0 => queue!(
                                    stdout,
                                    style::SetForegroundColor(style::Color::Reset),
                                    style::Print("   -  ")
                                )?,
                                _ => {
                                    if ev[rank2][rank1] <= -10.0 {
                                        queue!(
                                            stdout,
                                            style::SetForegroundColor(style::Color::Red),
                                            style::Print(format!(" {:.1}", ev[rank2][rank1]))
                                        )?
                                    } else if ev[rank2][rank1] < 0.0 {
                                        queue!(
                                            stdout,
                                            style::SetForegroundColor(style::Color::Magenta),
                                            style::Print(format!(" {:.2}", ev[rank2][rank1]))
                                        )?
                                    } else if ev[rank2][rank1] < 10.0 {
                                        queue!(
                                            stdout,
                                            style::SetForegroundColor(style::Color::Cyan),
                                            style::Print(format!(" {:+.2}", ev[rank2][rank1]))
                                        )?
                                    } else {
                                        queue!(
                                            stdout,
                                            style::SetForegroundColor(style::Color::Green),
                                            style::Print(format!(" {:+.1}", ev[rank2][rank1]))
                                        )?
                                    }
                                }
                            }
                        }

                        _ => unreachable!(),
                    }
                }

                queue!(
                    stdout,
                    style::SetForegroundColor(style::Color::Reset),
                    style::SetBackgroundColor(style::Color::Reset),
                )?;
            }

            if terminal_height > 26 {
                queue!(
                    stdout,
                    cursor::MoveTo(0, 26),
                    style::Print(format!(
                        "- EV: {:+.4}[bb] (SB) / {:+.4}[bb] (BB)",
                        output.1, -output.1
                    )),
                )?;
            }

            if terminal_height > 27 {
                queue!(
                    stdout,
                    cursor::MoveTo(0, 27),
                    style::Print(format!("- Exploitability: {:+.3e}[bb]", output.2)),
                )?;
            }
        }

        // flush queue
        stdout.flush().unwrap();

        // read pressed key
        match event::read()? {
            // quit
            Event::Key(key_ev) if key_ev == KeyCode::Char('q').into() => break,

            // toggle display
            Event::Key(key_ev) if key_ev == KeyCode::Char('t').into() => {
                display_mode = (display_mode + 1) % 2;
            }

            // multiple select
            Event::Key(key_ev) if key_ev == KeyCode::Char(' ').into() => {
                if indices.len() >= 2 {
                    let index = *indices.last().unwrap();
                    if !multiple_select.insert(index) {
                        multiple_select.remove(&index);
                    }
                }
            }

            Event::Key(key_ev) if key_ev == KeyCode::Esc.into() => {
                multiple_select.clear();
            }

            // Up key
            Event::Key(key_ev) if key_ev == KeyCode::Up.into() => {
                let len = indices.len();
                let index = indices.last_mut().unwrap();
                let num_index = num_indices.last().unwrap();
                if *index == 0 {
                    *index = *num_index - 1;
                    if len == 1 && *num_index > 7 {
                        files_top_index = *num_index - 7;
                    }
                } else {
                    *index -= 1;
                    if len == 1 && *index > 0 && *index == files_top_index {
                        files_top_index -= 1;
                    }
                }
            }

            // Down key
            Event::Key(key_ev) if key_ev == KeyCode::Down.into() => {
                let len = indices.len();
                let index = indices.last_mut().unwrap();
                let num_index = num_indices.last().unwrap();
                if *index + 1 == *num_index {
                    *index = 0;
                    if len == 1 {
                        files_top_index = 0;
                    }
                } else {
                    *index += 1;
                    if len == 1 && *index + 1 < *num_index && *index == files_top_index + 6 {
                        files_top_index += 1;
                    }
                }
            }

            // Enter or Right key
            Event::Key(key_ev)
                if key_ev == KeyCode::Enter.into() || key_ev == KeyCode::Right.into() =>
            {
                let last_idx = *indices.last().unwrap();
                if indices.len() == 1 {
                    output = &outputs[indices[0]].1 .0;
                    ev0_table = &outputs[indices[0]].1 .1;
                    ev1_table = &outputs[indices[0]].1 .2;
                }
                if indices.len() == 1 || last_idx >= 2 || (indices.len() == 2 && last_idx == 1) {
                    indices.push(0);
                    num_indices.push(0); // temporal value
                    multiple_select.clear();
                }
            }

            // Backspace or Left key
            Event::Key(key_ev)
                if key_ev == KeyCode::Backspace.into() || key_ev == KeyCode::Left.into() =>
            {
                if indices.len() > 1 {
                    indices.pop();
                    num_indices.pop();
                    multiple_select.clear();
                }
            }

            // ignore other keys
            _ => (),
        }
    }

    execute!(
        stdout,
        cursor::Show,
        event::DisableMouseCapture,
        terminal::LeaveAlternateScreen,
    )?;

    terminal::disable_raw_mode()?;
    Ok(())
}

fn compute_average_rate(cur_rate: &Vec<Vec<f64>>, strategy: &Vec<Vec<Vec<f64>>>) -> Vec<f64> {
    let num_actions = strategy.len();
    let mut ret = Vec::new();
    for action in 0..num_actions {
        let mut tmp = 0.0;
        for i in 0..13 {
            for j in 0..13 {
                let count = [12.0, 4.0, 6.0][(i <= j) as usize + (i == j) as usize];
                tmp += cur_rate[i][j] * strategy[action][i][j] * count;
            }
        }
        ret.push(tmp / (52. * 51. / 2.));
    }
    ret
}

fn restore_strategy(summarized: &Vec<Vec<Vec<f64>>>) -> Vec<Vec<f64>> {
    let num_actions = summarized.len();
    let mut strategy = vec![vec![0.0; 52 * 51 / 2]; num_actions];

    for action in 0..num_actions {
        let mut k = 0;
        for i in 0..51 {
            for j in (i + 1)..52 {
                if i % 4 == j % 4 {
                    strategy[action][k] = summarized[action][i / 4][j / 4];
                } else {
                    strategy[action][k] = summarized[action][j / 4][i / 4];
                }
                k += 1;
            }
        }
    }

    strategy
}

fn summarize_ev(ev: &Vec<f64>) -> Vec<Vec<f64>> {
    let mut summarized = vec![vec![0.0; 13]; 13];

    let mut k = 0;
    for i in 0..51 {
        for j in (i + 1)..52 {
            if i % 4 == j % 4 {
                summarized[i / 4][j / 4] += ev[k];
            } else {
                summarized[j / 4][i / 4] += ev[k];
            }
            k += 1;
        }
    }

    summarized
}

fn calc_ev(
    summarized_strategy: &HashMap<PublicInfoSet, Vec<Vec<Vec<f64>>>>,
    player: usize,
    stack: f64,
) -> HashMap<PublicInfoSet, Vec<Vec<f64>>> {
    let strategy = summarized_strategy
        .iter()
        .map(|(key, value)| (key.clone(), restore_strategy(value)))
        .collect::<HashMap<_, _>>();
    let node = game_preflop::PreflopNode::new(stack);
    let ones = vec![1.0; 52 * 51 / 2];
    let ev = std::sync::Mutex::new(HashMap::new());
    cfr::compute_ev_detail(&node, player, &ones, &ones, &strategy, &ev);
    let ev = ev
        .lock()
        .unwrap()
        .iter()
        .map(|(key, value)| (key.clone(), summarize_ev(value)))
        .collect();
    ev
}

fn calc_denom(opp_rate: &Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    let mut ret = vec![vec![0.0; 13]; 13];

    for i in 0..51 {
        for j in (i + 1)..52 {
            let rate = if i % 4 == j % 4 {
                opp_rate[i / 4][j / 4]
            } else {
                opp_rate[j / 4][i / 4]
            };
            for m in 0..51 {
                for n in (m + 1)..52 {
                    if i == m || i == n || j == m || j == n {
                        continue;
                    } else if m % 4 == n % 4 {
                        ret[m / 4][n / 4] += rate;
                    } else {
                        ret[n / 4][m / 4] += rate;
                    }
                }
            }
        }
    }

    for i in 0..13 {
        for j in 0..13 {
            ret[i][j] *= (2. * 2.) / (52. * 51. * 50. * 49.);
        }
    }

    ret
}

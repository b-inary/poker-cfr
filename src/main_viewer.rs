mod game_node;

use bincode::deserialize;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute, queue, style, terminal,
};
use game_node::PublicInfoSet;
use ordered_float::NotNan;
use regex::Regex;
use std::cmp::{max, min};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{Read, Write};

type OutputType = (HashMap<PublicInfoSet, Vec<Vec<f64>>>, f64, f64);

fn main() {
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

fn get_outputs() -> std::io::Result<Vec<(NotNan<f64>, OutputType)>> {
    let output_path = "output/";
    let re = Regex::new(r"preflop-(\d+.?\d*)-(\d+).bin").unwrap();
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
                outputs.insert(stack, decoded);
                iterations.insert(stack, iteration);
            }
        }
    }

    let outputs = outputs.into_iter().collect::<Vec<_>>();
    Ok(outputs)
}

fn interactive_display(outputs: &Vec<(NotNan<f64>, OutputType)>) -> crossterm::Result<()> {
    let mut stdout = std::io::stdout();

    terminal::enable_raw_mode()?;

    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        event::EnableMouseCapture,
        cursor::Hide,
    )?;

    let (terminal_width, terminal_height) = terminal::size()?;
    let mut first_index_top = 0;
    let mut indices = vec![0];
    let mut num_indices = vec![outputs.len()];
    let mut output: &OutputType = &(HashMap::new(), 0.0, 0.0);

    loop {
        queue!(
            stdout,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0),
            style::Print("Press 'q' to quit."),
        )?;

        let (start, offset) = if 23 * indices.len() - 15 > terminal_width as usize {
            queue!(stdout, cursor::MoveTo(0, 5), style::Print("(omitted)"))?;
            let start = indices.len() - (terminal_width as usize - 10) / 23;
            (start, 23 * start - 9)
        } else {
            (1, 15)
        };

        if start == 1 {
            for (i, (stack, _)) in outputs.iter().enumerate() {
                if i < first_index_top || first_index_top + 6 < i {
                    continue;
                }
                queue!(stdout, cursor::MoveTo(0, (i - first_index_top + 2) as u16))?;
                if first_index_top > 0 && i == first_index_top {
                    queue!(stdout, style::Print("    ^  "))?;
                    continue;
                }
                if i + 1 < outputs.len() && i == first_index_top + 6 {
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

            let action_rate = analyze_strategy(strategy);
            let avg_rate = compute_average_rate(&cur_rate[i % 2], &action_rate);

            num_indices[i] = strategy.len();

            if start <= i {
                queue!(
                    stdout,
                    cursor::MoveTo((23 * i - offset) as u16, indices[i - 1] as u16 + 2),
                    style::SetAttribute(style::Attribute::Bold),
                    style::Print("=>"),
                    style::SetAttribute(style::Attribute::Reset),
                )?;

                for j in 0..num_indices[i] {
                    let action = if j >= 2 && j + 1 == num_indices[i] {
                        "All-in"
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
                        cursor::MoveTo((23 * i - offset + 3) as u16, j as u16 + 2),
                        style::Print(format!(
                            "{} [{:>5.2}%] {}",
                            [' ', '*'][(j == indices[i]) as usize],
                            100.0 * avg_rate[j],
                            action
                        )),
                        style::SetAttribute(style::Attribute::Reset),
                    )?;
                }
            }

            for j in 0..13 {
                for k in 0..13 {
                    cur_rate[i % 2][j][k] *= action_rate[indices[i]][j][k];
                }
            }
        }

        if indices.len() >= 2 {
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

                    match ((cur_rate[(indices.len() + 1) % 2][rank2][rank1] + 0.1) * 5.0) as usize {
                        0 => queue!(
                            stdout,
                            style::SetForegroundColor(style::Color::Reset),
                            style::Print("   -  ")
                        )?,
                        1 => queue!(
                            stdout,
                            style::SetForegroundColor(style::Color::Red),
                            style::Print(" *    ")
                        )?,
                        2 => queue!(
                            stdout,
                            style::SetForegroundColor(style::Color::Magenta),
                            style::Print(" **   ")
                        )?,
                        3 => queue!(
                            stdout,
                            style::SetForegroundColor(style::Color::Yellow),
                            style::Print(" ***  ")
                        )?,
                        4 => queue!(
                            stdout,
                            style::SetForegroundColor(style::Color::Cyan),
                            style::Print(" **** ")
                        )?,
                        5 => queue!(
                            stdout,
                            style::SetForegroundColor(style::Color::Green),
                            style::Print(" *****")
                        )?,
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

            // Up key
            Event::Key(key_ev) if key_ev == KeyCode::Up.into() => {
                let len = indices.len();
                let index = indices.last_mut().unwrap();
                let num_index = num_indices.last().unwrap();
                if *index == 0 {
                    *index = *num_index - 1;
                    if len == 1 && *num_index > 7 {
                        first_index_top = *num_index - 7;
                    }
                } else {
                    *index -= 1;
                    if len == 1 && *index > 0 && *index == first_index_top {
                        first_index_top -= 1;
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
                        first_index_top = 0;
                    }
                } else {
                    *index += 1;
                    if len == 1 && *index + 1 < *num_index && *index == first_index_top + 6 {
                        first_index_top += 1;
                    }
                }
            }

            // Enter or Right key
            Event::Key(key_ev)
                if key_ev == KeyCode::Enter.into() || key_ev == KeyCode::Right.into() =>
            {
                let last_idx = *indices.last().unwrap();
                if indices.len() == 1 {
                    output = &outputs[indices[0]].1;
                }
                if indices.len() == 1 || last_idx >= 2 || (indices.len() == 2 && last_idx == 1) {
                    indices.push(0);
                    num_indices.push(0); // temporal value
                }
            }

            // Backspace or Left key
            Event::Key(key_ev)
                if key_ev == KeyCode::Backspace.into() || key_ev == KeyCode::Left.into() =>
            {
                if indices.len() > 1 {
                    indices.pop();
                    num_indices.pop();
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

fn analyze_strategy(strategy: &Vec<Vec<f64>>) -> Vec<Vec<Vec<f64>>> {
    let num_actions = strategy.len();
    let mut action_rate = vec![vec![vec![0.0; 13]; 13]; num_actions];

    for action in 0..num_actions {
        let mut k = 0;
        for i in 0..51 {
            for j in (i + 1)..52 {
                let rank1 = i / 4;
                let rank2 = j / 4;
                let suit1 = i % 4;
                let suit2 = j % 4;
                let minrank = min(rank1, rank2);
                let maxrank = max(rank1, rank2);
                if suit1 == suit2 {
                    action_rate[action][minrank][maxrank] += strategy[action][k];
                } else {
                    action_rate[action][maxrank][minrank] += strategy[action][k];
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
                action_rate[action][i][j] /= count;
            }
        }
    }

    action_rate
}

fn compute_average_rate(cur_rate: &Vec<Vec<f64>>, action_rate: &Vec<Vec<Vec<f64>>>) -> Vec<f64> {
    let num_actions = action_rate.len();
    let mut ret = Vec::new();
    for action in 0..num_actions {
        let mut tmp = 0.0;
        for i in 0..13 {
            for j in 0..13 {
                let count = if i == j {
                    6.0
                } else if i < j {
                    4.0
                } else {
                    12.0
                };
                tmp += cur_rate[i][j] * action_rate[action][i][j] * count;
            }
        }
        ret.push(tmp / (52. * 51. / 2.));
    }
    ret
}

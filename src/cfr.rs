use crate::game_node::*;
use bincode::serialize;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;

/// Vector-scalar multiplication.
#[inline]
fn mul_scalar(vec: &mut Vec<f64>, scalar: f64) {
    for el in vec {
        *el *= scalar;
    }
}

/// Force each element to be non-negative.
#[inline]
fn nonneg_vector(vec: &mut Vec<f64>) {
    for el in vec {
        *el = el.max(0.0);
    }
}

/// Element-wise vector addition.
#[inline]
fn add_vector(lhs: &mut Vec<f64>, rhs: &Vec<f64>) {
    for (l, r) in lhs.iter_mut().zip(rhs) {
        *l += r;
    }
}

/// Element-wise vector subtraction.
#[inline]
fn sub_vector(lhs: &mut Vec<f64>, rhs: &Vec<f64>) {
    for (l, r) in lhs.iter_mut().zip(rhs) {
        *l -= r;
    }
}

/// Element-wise vector multiplication.
#[inline]
fn mul_vector(lhs: &mut Vec<f64>, rhs: &Vec<f64>) {
    for (l, r) in lhs.iter_mut().zip(rhs) {
        *l *= r;
    }
}

/// Element-wise vector division. When denominator is zero, `default` value is used.
#[inline]
fn div_vector(lhs: &mut Vec<f64>, rhs: &Vec<f64>, default: f64) {
    for (l, r) in lhs.iter_mut().zip(rhs) {
        if *r == 0.0 {
            *l = default;
        } else {
            *l /= r;
        }
    }
}

/// Element-wise max operation.
#[inline]
fn max_vector(lhs: &mut Vec<f64>, rhs: &Vec<f64>) {
    for (l, r) in lhs.iter_mut().zip(rhs) {
        *l = l.max(*r);
    }
}

/// Computes inner product.
#[inline]
fn dot(lhs: &Vec<f64>, rhs: &Vec<f64>) -> f64 {
    let mut ret = 0.0;
    for (l, r) in lhs.iter().zip(rhs) {
        ret += l * r;
    }
    ret
}

/// Builds default tree.
fn build_tree(node: &impl GameNode, tree: &mut HashMap<PublicInfoSet, Vec<Vec<f64>>>) {
    if node.is_terminal_node() {
        return;
    }

    tree.insert(
        node.public_info_set().clone(),
        vec![vec![0.0; node.private_info_set_len()]; node.num_actions()],
    );

    for action in node.actions() {
        build_tree(&node.play(action), tree);
    }
}

/// Builds default tree (multi-threaded version).
fn build_tree_mt(node: &impl GameNode, tree: &mut HashMap<PublicInfoSet, Mutex<Vec<Vec<f64>>>>) {
    if node.is_terminal_node() {
        return;
    }

    tree.insert(
        node.public_info_set().clone(),
        Mutex::new(vec![
            vec![0.0; node.private_info_set_len()];
            node.num_actions()
        ]),
    );

    for action in node.actions() {
        build_tree_mt(&node.play(action), tree);
    }
}

/// Performs counterfactual regret minimization.
/// Returns: counterfactual value
fn cfr(
    node: &impl GameNode,
    iter: usize,
    player: usize,
    pi: &Vec<f64>,
    pmi: &Vec<f64>,
    cum_cfr: &mut HashMap<PublicInfoSet, Vec<Vec<f64>>>,
    cum_sgm: &mut HashMap<PublicInfoSet, Vec<Vec<f64>>>,
) -> Vec<f64> {
    // terminal node
    if node.is_terminal_node() {
        return node.evaluate(player, pmi);
    }

    // initialize counterfactual value
    let mut cfvalue = vec![0.0; node.private_info_set_len()];

    // get current public information set
    let public_info_set = node.public_info_set();

    // compute current sigma
    let sigma = regret_matching(&cum_cfr[public_info_set]);

    if node.current_player() == player {
        let mut cfvalue_action = Vec::with_capacity(node.num_actions());

        for action in node.actions() {
            let mut pi = pi.clone();
            mul_vector(&mut pi, &sigma[action]);
            let mut tmp = cfr(&node.play(action), iter, player, &pi, pmi, cum_cfr, cum_sgm);
            cfvalue_action.push(tmp.clone());
            mul_vector(&mut tmp, &sigma[action]);
            add_vector(&mut cfvalue, &tmp);
        }

        // update cumulative regrets and sigmas
        for action in node.actions() {
            let r = &mut cum_cfr.get_mut(public_info_set).unwrap()[action];
            let s = &mut cum_sgm.get_mut(public_info_set).unwrap()[action];
            let mut pi = pi.clone();
            add_vector(r, &cfvalue_action[action]);
            sub_vector(r, &cfvalue);

            // Regret-matching+
            nonneg_vector(r);

            // CFR+
            mul_scalar(&mut pi, iter as f64);

            mul_vector(&mut pi, &sigma[action]);
            add_vector(s, &pi);
        }
    } else {
        for action in node.actions() {
            let mut pmi = pmi.clone();
            mul_vector(&mut pmi, &sigma[action]);
            let tmp = cfr(&node.play(action), iter, player, pi, &pmi, cum_cfr, cum_sgm);
            add_vector(&mut cfvalue, &tmp);
        }
    }

    cfvalue
}

/// Performs counterfactual regret minimization (multi-threaded version).
/// Returns: counterfactual value
fn cfr_mt(
    node: &impl GameNode,
    iter: usize,
    player: usize,
    pi: &Vec<f64>,
    pmi: &Vec<f64>,
    cum_cfr: &HashMap<PublicInfoSet, Mutex<Vec<Vec<f64>>>>,
    cum_sgm: &HashMap<PublicInfoSet, Mutex<Vec<Vec<f64>>>>,
) -> Vec<f64> {
    // terminal node
    if node.is_terminal_node() {
        return node.evaluate(player, pmi);
    }

    // get current public information set
    let public_info_set = node.public_info_set();

    // compute current sigma
    let sigma = regret_matching(&cum_cfr[public_info_set].lock().unwrap());

    let cfvalue;
    if node.current_player() == player {
        let mut cfvalue_action = Vec::with_capacity(node.num_actions());
        for _ in node.actions() {
            cfvalue_action.push(Mutex::new(Vec::new()));
        }

        cfvalue = node
            .actions()
            .into_par_iter()
            .map(|action| {
                let mut pi = pi.clone();
                mul_vector(&mut pi, &sigma[action]);
                let mut tmp = cfr_mt(&node.play(action), iter, player, &pi, pmi, cum_cfr, cum_sgm);
                *cfvalue_action[action].lock().unwrap() = tmp.clone();
                mul_vector(&mut tmp, &sigma[action]);
                tmp
            })
            .reduce(
                || vec![0.0; node.private_info_set_len()],
                |mut v, w| {
                    add_vector(&mut v, &w);
                    v
                },
            );

        // update cumulative regrets and sigmas
        let mut cum_cfr = cum_cfr[public_info_set].lock().unwrap();
        let mut cum_sgm = cum_sgm[public_info_set].lock().unwrap();
        for action in node.actions() {
            let r = &mut cum_cfr[action];
            let mut pi = pi.clone();
            add_vector(r, &cfvalue_action[action].lock().unwrap());
            sub_vector(r, &cfvalue);

            // Regret-matching+
            nonneg_vector(r);

            // CFR+
            mul_scalar(&mut pi, iter as f64);

            mul_vector(&mut pi, &sigma[action]);
            add_vector(&mut cum_sgm[action], &pi);
        }
    } else {
        cfvalue = node
            .actions()
            .into_par_iter()
            .map(|action| {
                let mut pmi = pmi.clone();
                mul_vector(&mut pmi, &sigma[action]);
                cfr_mt(&node.play(action), iter, player, pi, &pmi, cum_cfr, cum_sgm)
            })
            .reduce(
                || vec![0.0; node.private_info_set_len()],
                |mut v, w| {
                    add_vector(&mut v, &w);
                    v
                },
            );
    }

    cfvalue
}

/// Performs regret matching.
fn regret_matching(cum_cfr: &Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    let num_actions = cum_cfr.len();
    let private_info_set_len = cum_cfr[0].len();

    let mut denom = vec![0.0; private_info_set_len];
    for cum_cfr_action in cum_cfr {
        let mut tmp = cum_cfr_action.clone();
        nonneg_vector(&mut tmp);
        add_vector(&mut denom, &tmp);
    }

    let mut result = Vec::with_capacity(num_actions);
    for cum_cfr_action in cum_cfr {
        let mut tmp = cum_cfr_action.clone();
        nonneg_vector(&mut tmp);
        div_vector(&mut tmp, &denom, 1.0 / num_actions as f64);
        result.push(tmp);
    }

    result
}

/// Computes `player`'s EV.
fn compute_ev(
    node: &impl GameNode,
    player: usize,
    pi: &Vec<f64>,
    pmi: &Vec<f64>,
    sigma: &HashMap<PublicInfoSet, Vec<Vec<f64>>>,
) -> f64 {
    if node.is_terminal_node() {
        return dot(&node.evaluate(player, pmi), &pi);
    }

    let strategy = &sigma[node.public_info_set()];

    if node.current_player() == player {
        node.actions()
            .into_par_iter()
            .map(|action| {
                let mut pi = pi.clone();
                mul_vector(&mut pi, &strategy[action]);
                compute_ev(&node.play(action), player, &pi, pmi, sigma)
            })
            .sum::<f64>()
    } else {
        node.actions()
            .into_par_iter()
            .map(|action| {
                let mut pmi = pmi.clone();
                mul_vector(&mut pmi, &strategy[action]);
                compute_ev(&node.play(action), player, pi, &pmi, sigma)
            })
            .sum::<f64>()
    }
}

/// Computes `player`'s EV and stores result to `ev`.
pub fn compute_ev_detail(
    node: &impl GameNode,
    player: usize,
    pi: &Vec<f64>,
    pmi: &Vec<f64>,
    sigma: &HashMap<PublicInfoSet, Vec<Vec<f64>>>,
    ev: &Mutex<HashMap<PublicInfoSet, Vec<f64>>>,
) -> Vec<f64> {
    let public_info_set = node.public_info_set();

    if node.is_terminal_node() {
        let mut pi = pi.clone();
        mul_vector(&mut pi, &node.evaluate(player, pmi));
        ev.lock()
            .unwrap()
            .insert(public_info_set.clone(), pi.clone());
        return pi;
    }

    let strategy = &sigma[public_info_set];

    let result = node
        .actions()
        .into_par_iter()
        .map(|action| {
            if node.current_player() == player {
                let mut pi = pi.clone();
                mul_vector(&mut pi, &strategy[action]);
                compute_ev_detail(&node.play(action), player, &pi, pmi, sigma, ev)
            } else {
                let mut pmi = pmi.clone();
                mul_vector(&mut pmi, &strategy[action]);
                compute_ev_detail(&node.play(action), player, pi, &pmi, sigma, ev)
            }
        })
        .reduce(
            || vec![0.0; node.private_info_set_len()],
            |mut v, w| {
                add_vector(&mut v, &w);
                v
            },
        );

    ev.lock()
        .unwrap()
        .insert(public_info_set.clone(), result.clone());

    result
}

/// Computes best response.
fn compute_best_response(
    node: &impl GameNode,
    player: usize,
    pmi: &Vec<f64>,
    sigma: &HashMap<PublicInfoSet, Vec<Vec<f64>>>,
) -> Vec<f64> {
    if node.is_terminal_node() {
        return node.evaluate(player, pmi);
    }

    if node.current_player() == player {
        node.actions()
            .into_par_iter()
            .map(|action| compute_best_response(&node.play(action), player, pmi, sigma))
            .reduce(
                || vec![f64::MIN; node.private_info_set_len()],
                |mut v, w| {
                    max_vector(&mut v, &w);
                    v
                },
            )
    } else {
        let strategy = &sigma[node.public_info_set()];
        node.actions()
            .into_par_iter()
            .map(|action| {
                let mut pmi = pmi.clone();
                mul_vector(&mut pmi, &strategy[action]);
                compute_best_response(&node.play(action), player, &pmi, sigma)
            })
            .reduce(
                || vec![0.0; node.private_info_set_len()],
                |mut v, w| {
                    add_vector(&mut v, &w);
                    v
                },
            )
    }
}

/// Computes exploitability.
fn compute_exploitability(
    root: &impl GameNode,
    sigma: &HashMap<PublicInfoSet, Vec<Vec<f64>>>,
) -> f64 {
    let ones = vec![1.0; root.private_info_set_len()];
    let br0 = compute_best_response(root, 0, &ones, sigma);
    let br1 = compute_best_response(root, 1, &ones, sigma);
    br0.iter().sum::<f64>() + br1.iter().sum::<f64>()
}

/// Computes average strategy.
fn compute_average_strategy(
    cum_sigma: &HashMap<PublicInfoSet, Vec<Vec<f64>>>,
) -> HashMap<PublicInfoSet, Vec<Vec<f64>>> {
    let mut ret = HashMap::new();

    for (key, value) in cum_sigma {
        let num_actions = value.len();
        let private_info_set_len = value[0].len();

        let mut denom = vec![0.0; private_info_set_len];
        for cum_sigma_action in value {
            add_vector(&mut denom, cum_sigma_action);
        }

        let mut result = Vec::with_capacity(num_actions);
        for cum_sigma_action in value {
            let mut tmp = cum_sigma_action.clone();
            div_vector(&mut tmp, &denom, 0.0);
            result.push(tmp);
        }

        ret.insert(key.clone(), result);
    }

    ret
}

/// Computes average strategy (multi-threaded version).
fn compute_average_strategy_mt(
    cum_sigma: &HashMap<PublicInfoSet, Mutex<Vec<Vec<f64>>>>,
) -> HashMap<PublicInfoSet, Vec<Vec<f64>>> {
    let mut ret = HashMap::new();

    for (key, value) in cum_sigma {
        let value = value.lock().unwrap();
        let num_actions = value.len();
        let private_info_set_len = value[0].len();

        let mut denom = vec![0.0; private_info_set_len];
        for cum_sigma_action in value.iter() {
            add_vector(&mut denom, &cum_sigma_action);
        }

        let mut result = Vec::with_capacity(num_actions);
        for cum_sigma_action in value.iter() {
            let mut tmp = cum_sigma_action.clone();
            div_vector(&mut tmp, &denom, 0.0);
            result.push(tmp);
        }

        ret.insert(key.clone(), result);
    }

    ret
}

/// Performs training.
/// Returns: (obtained strategy, player-0's EV, exploitability)
pub fn train(
    root: &impl GameNode,
    num_iter: usize,
    show_progress: bool,
) -> (HashMap<PublicInfoSet, Vec<Vec<f64>>>, f64, f64) {
    let ones = vec![1.0; root.private_info_set_len()];
    let mut cum_cfr = HashMap::new();
    let mut cum_sgm = HashMap::new();
    build_tree(root, &mut cum_cfr);
    build_tree(root, &mut cum_sgm);

    for iter in 0..num_iter {
        if show_progress {
            print!("\riteration: {} / {}", iter + 1, num_iter);
        }
        std::io::stdout().flush().unwrap();
        for player in 0..2 {
            cfr(root, iter, player, &ones, &ones, &mut cum_cfr, &mut cum_sgm);
        }
    }
    if show_progress {
        println!();
    }

    let avg_sigma = compute_average_strategy(&cum_sgm);
    let ev = compute_ev(root, 0, &ones, &ones, &avg_sigma);
    let exploitability = compute_exploitability(root, &avg_sigma);
    (avg_sigma, ev, exploitability)
}

/// Performs training (multi-threaded version).
/// Returns: (obtained strategy, player-0's EV, exploitability)
pub fn train_mt<T: serde::Serialize>(
    root: &impl GameNode,
    num_iter: usize,
    show_progress: bool,
    save_file_opt: Option<(impl Fn(usize) -> String, impl Fn(&Vec<Vec<f64>>) -> T)>,
) -> (HashMap<PublicInfoSet, Vec<Vec<f64>>>, f64, f64) {
    let ones = vec![1.0; root.private_info_set_len()];
    let mut cum_cfr = HashMap::new();
    let mut cum_sgm = HashMap::new();
    build_tree_mt(root, &mut cum_cfr);
    build_tree_mt(root, &mut cum_sgm);

    for iter in 0..num_iter {
        if show_progress {
            print!("\riteration: {} / {}", iter + 1, num_iter);
        }
        std::io::stdout().flush().unwrap();

        for player in 0..2 {
            cfr_mt(root, iter, player, &ones, &ones, &cum_cfr, &cum_sgm);
        }

        if (iter + 1) % 1000 == 0 {
            let avg_sigma = compute_average_strategy_mt(&cum_sgm);
            let exploitability = compute_exploitability(root, &avg_sigma);
            print!(" (exploitability = {:+.3e}[bb])", exploitability);
            std::io::stdout().flush().unwrap();

            if let Some((outpath_fn, convert_fn)) = &save_file_opt {
                if iter >= 1000 {
                    let prevpath = outpath_fn(iter - 999);
                    let _ = std::fs::remove_file(prevpath);
                }
                let outpath = outpath_fn(iter + 1);
                let converted = avg_sigma
                    .iter()
                    .map(|(key, value)| (key.clone(), convert_fn(value)))
                    .collect::<HashMap<_, _>>();
                let ev = compute_ev(root, 0, &ones, &ones, &avg_sigma);
                let encoded = serialize(&(converted, ev, exploitability)).unwrap();
                let mut outfile = File::create(&outpath).unwrap();
                outfile.write_all(&encoded).unwrap();
            }
        }
    }

    if show_progress {
        println!();
    }

    if let Some((outpath_fn, _)) = &save_file_opt {
        if num_iter >= 1000 {
            let prevpath = outpath_fn(num_iter - num_iter % 1000);
            let _ = std::fs::remove_file(prevpath);
        }
    }

    let avg_sigma = compute_average_strategy_mt(&cum_sgm);
    let ev = compute_ev(root, 0, &ones, &ones, &avg_sigma);
    let exploitability = compute_exploitability(root, &avg_sigma);
    (avg_sigma, ev, exploitability)
}

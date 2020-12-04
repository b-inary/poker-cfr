use std::collections::HashMap;

pub type PublicInfoSet = Vec<usize>;

pub trait GameNode {
    /// Returns whether the current node is a terminal node.
    fn is_terminal_node(&self) -> bool;

    /// Returns the current player's index.
    fn current_player(&self) -> usize;

    /// Returns the number of possible actions.
    fn num_actions(&self) -> usize;

    /// Returns a set of valid actions.
    fn actions(&self) -> std::ops::Range<usize>;

    /// Plays `action` and returns a node after `action` played.
    fn play(&self, action: usize) -> Self;

    /// Returns the public information set.
    fn public_info_set(&self) -> &PublicInfoSet;

    /// Returns the length of private information set.
    fn private_info_set_len(&self) -> usize;

    /// Computes player's counterfactual values according to `pmi`.
    fn evaluate(&self, player: usize, pmi: &Vec<f64>) -> Vec<f64>;
}

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

/// Performs counterfactual regret minimization.
/// Returns: counterfactual value
fn cfr_rec(
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

    // create default entries when newly visited
    if !cum_cfr.contains_key(public_info_set) {
        let default = vec![vec![0.0; node.private_info_set_len()]; node.num_actions()];
        cum_cfr.insert(public_info_set.clone(), default.clone());
        cum_sgm.insert(public_info_set.clone(), default.clone());
    }

    // compute current sigma
    let sigma = regret_matching(&cum_cfr[public_info_set]);

    if node.current_player() == player {
        let mut cfvalue_action = Vec::with_capacity(node.num_actions());

        for action in node.actions() {
            let mut pi = pi.clone();
            mul_vector(&mut pi, &sigma[action]);
            let mut tmp = cfr_rec(&node.play(action), iter, player, &pi, pmi, cum_cfr, cum_sgm);
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
            let tmp = cfr_rec(&node.play(action), iter, player, pi, &pmi, cum_cfr, cum_sgm);
            add_vector(&mut cfvalue, &tmp);
        }
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

/// Computes  `player`'s EV.
fn compute_ev(
    node: &impl GameNode,
    player: usize,
    pi: &Vec<f64>,
    pmi: &Vec<f64>,
    sigma: &HashMap<Vec<usize>, Vec<Vec<f64>>>,
) -> f64 {
    if node.is_terminal_node() {
        return dot(&node.evaluate(player, pmi), &pi);
    }

    let mut ev = 0.0;
    let public_info_set = node.public_info_set();
    let strategy = &sigma[public_info_set];

    if node.current_player() == player {
        for action in node.actions() {
            let mut pi = pi.clone();
            mul_vector(&mut pi, &strategy[action]);
            ev += compute_ev(&node.play(action), player, &pi, pmi, sigma);
        }
    } else {
        for action in node.actions() {
            let mut pmi = pmi.clone();
            mul_vector(&mut pmi, &strategy[action]);
            ev += compute_ev(&node.play(action), player, pi, &pmi, sigma);
        }
    }

    ev
}

/// Computes best response.
fn compute_best_response(
    node: &impl GameNode,
    player: usize,
    pmi: &Vec<f64>,
    sigma: &HashMap<Vec<usize>, Vec<Vec<f64>>>,
) -> Vec<f64> {
    if node.is_terminal_node() {
        return node.evaluate(player, pmi);
    }

    let mut best_response;
    let public_info_set = node.public_info_set();

    if node.current_player() == player {
        best_response = vec![f64::MIN; node.private_info_set_len()];
        for action in node.actions() {
            let tmp = compute_best_response(&node.play(action), player, pmi, sigma);
            max_vector(&mut best_response, &tmp);
        }
    } else {
        best_response = vec![0.0; node.private_info_set_len()];
        let strategy = &sigma[public_info_set];
        for action in node.actions() {
            let mut pmi = pmi.clone();
            mul_vector(&mut pmi, &strategy[action]);
            let tmp = compute_best_response(&node.play(action), player, &pmi, sigma);
            add_vector(&mut best_response, &tmp);
        }
    }

    best_response
}

/// Computes exploitability.
fn compute_exploitability(root: &impl GameNode, sigma: &HashMap<Vec<usize>, Vec<Vec<f64>>>) -> f64 {
    let ones = vec![1.0; root.private_info_set_len()];
    let br0 = compute_best_response(root, 0, &ones, sigma);
    let br1 = compute_best_response(root, 1, &ones, sigma);
    br0.iter().sum::<f64>() + br1.iter().sum::<f64>()
}

/// Performs training.
/// Returns: (obtained strategy, player-0's EV, exploitability)
pub fn train(
    root: &impl GameNode,
    num_iter: usize,
) -> (HashMap<PublicInfoSet, Vec<Vec<f64>>>, f64, f64) {
    let mut cum_cfr = HashMap::new();
    let mut cum_sgm = HashMap::new();
    let ones = vec![1.0; root.private_info_set_len()];

    for iter in 0..num_iter {
        for player in 0..2 {
            cfr_rec(root, iter, player, &ones, &ones, &mut cum_cfr, &mut cum_sgm);
        }
    }

    let mut average_strategy = HashMap::new();

    for (key, value) in &cum_sgm {
        let num_actions = value.len();
        let private_info_set_len = value[0].len();

        let mut denom = vec![0.0; private_info_set_len];
        for cum_sgm_action in value {
            add_vector(&mut denom, cum_sgm_action);
        }

        let mut result = Vec::with_capacity(num_actions);
        for cum_sgm_action in value {
            let mut tmp = cum_sgm_action.clone();
            div_vector(&mut tmp, &denom, 0.0);
            result.push(tmp);
        }

        average_strategy.insert(key.clone(), result);
    }

    let ev = compute_ev(root, 0, &ones, &ones, &average_strategy);
    let exploitability = compute_exploitability(root, &average_strategy);
    (average_strategy, ev, exploitability)
}

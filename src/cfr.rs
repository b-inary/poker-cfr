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

    /// Evaluates player's payoffs according to `pmi`.
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

    // get encoded information set string
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
        let mut cfvalue_action = Vec::new();

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
    let mut result = Vec::new();
    let mut denom = vec![0.0; private_info_set_len];
    for cum_cfr_action in cum_cfr {
        let mut tmp = cum_cfr_action.clone();
        nonneg_vector(&mut tmp);
        add_vector(&mut denom, &tmp);
    }
    for cum_cfr_action in cum_cfr {
        let mut tmp = cum_cfr_action.clone();
        nonneg_vector(&mut tmp);
        div_vector(&mut tmp, &denom, 1.0 / num_actions as f64);
        result.push(tmp);
    }
    result
}

/// Performs training.
/// Returns: obtained strategy
pub fn train(root: &impl GameNode, num_iter: usize) -> HashMap<PublicInfoSet, Vec<Vec<f64>>> {
    let mut cum_cfr = HashMap::new();
    let mut cum_sgm = HashMap::new();
    let pi = vec![1.0; root.private_info_set_len()];
    for iter in 0..num_iter {
        for player in 0..2 {
            cfr_rec(root, iter, player, &pi, &pi, &mut cum_cfr, &mut cum_sgm);
        }
    }

    let mut average_strategy = HashMap::new();
    for (key, value) in &cum_sgm {
        let mut result = Vec::new();
        let mut denom = vec![0.0; value[0].len()];
        for cum_sgm_action in value {
            add_vector(&mut denom, cum_sgm_action);
        }
        for cum_sgm_action in value {
            let mut tmp = cum_sgm_action.clone();
            div_vector(&mut tmp, &denom, 0.0);
            result.push(tmp);
        }
        average_strategy.insert(key.clone(), result);
    }

    average_strategy
}

pub type PublicInfoSet = Vec<u8>;

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

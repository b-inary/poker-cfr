use crate::cfr::*;

#[derive(Clone, Debug)]
pub struct KuhnNode {
    public_info_set: PublicInfoSet,
}

impl GameNode for KuhnNode {
    #[inline]
    fn is_terminal_node(&self) -> bool {
        match self.public_info_set.as_slice() {
            [0, 1] => false,
            [_, _] => true,
            [_, _, _] => true,
            _ => false,
        }
    }

    #[inline]
    fn current_player(&self) -> usize {
        self.public_info_set.len() % 2
    }

    #[inline]
    fn num_actions(&self) -> usize {
        2
    }

    #[inline]
    fn actions(&self) -> std::ops::Range<usize> {
        0..self.num_actions()
    }

    #[inline]
    fn play(&self, action: usize) -> Self {
        let mut ret = self.clone();
        ret.public_info_set.push(action);
        ret
    }

    #[inline]
    fn public_info_set(&self) -> &PublicInfoSet {
        &self.public_info_set
    }

    #[inline]
    fn private_info_set_len(&self) -> usize {
        3
    }

    #[inline]
    fn evaluate(&self, player: usize, pmi: &Vec<f64>) -> Vec<f64> {
        let mut ret = Vec::new();
        for i in 0..self.private_info_set_len() {
            let mut cfvalue = 0.0;
            for j in 0..self.private_info_set_len() {
                if i == j {
                    continue;
                }
                cfvalue += self.payoff(player, i, j) * pmi[j];
            }
            ret.push(cfvalue);
        }
        ret
    }
}

impl KuhnNode {
    #[inline]
    pub fn new() -> Self {
        Self {
            public_info_set: Vec::new(),
        }
    }

    #[inline]
    pub fn public_info_set_str(info_set: &PublicInfoSet) -> String {
        match info_set.as_slice() {
            [] => "(Empty)",
            [0] => "Check",
            [1] => "Bet",
            [0, 1] => "Check => Bet",
            _ => unreachable!(),
        }
        .into()
    }

    #[inline]
    fn payoff(&self, player: usize, my_card: usize, opp_card: usize) -> f64 {
        if let [0, 0] = self.public_info_set.as_slice() {
            // check => check
            if my_card > opp_card {
                1.0
            } else {
                -1.0
            }
        } else if self.public_info_set.last() == Some(&0) {
            // last player folded
            if self.current_player() == player {
                1.0
            } else {
                -1.0
            }
        } else {
            // last player called
            if my_card > opp_card {
                2.0
            } else {
                -2.0
            }
        }
    }
}

use crate::cfr::*;

#[derive(Clone, Debug)]
pub struct KuhnNode {
    public_info_set: Vec<PublicInfoSet>,
}

impl GameNode for KuhnNode {
    #[inline]
    fn is_terminal_node(&self) -> bool {
        match self.public_info_set.last() {
            Some(action) if action == "Bet" => false,
            Some(action) if action == "Call" => true,
            Some(action) if action == "Check" => self.public_info_set.len() == 2,
            Some(action) if action == "Fold" => true,
            None => false,
            _ => unreachable!(),
        }
    }

    #[inline]
    fn is_community_card_node(&self) -> bool {
        false
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
    fn community_card_actions(&self) -> std::iter::Enumerate<std::slice::Iter<'_, f64>> {
        unreachable!()
    }

    #[inline]
    fn play(&self, action: usize) -> Self {
        let mut ret = self.clone();
        ret.public_info_set.push(match self.public_info_set.last() {
            Some(prev_action) if prev_action == "Bet" => ["Call", "Fold"][action].into(),
            Some(prev_action) if prev_action == "Check" => ["Bet", "Check"][action].into(),
            None => ["Bet", "Check"][action].into(),
            _ => unreachable!(),
        });
        ret
    }

    #[inline]
    fn public_info_set(&self) -> PublicInfoSet {
        self.public_info_set.join("->")
    }

    #[inline]
    fn private_info_set_size(&self) -> usize {
        3
    }

    #[inline]
    fn evaluate(&self, player: usize, pmi: &Vec<f64>) -> Vec<f64> {
        let mut ret = Vec::new();
        for i in 0..self.private_info_set_size() {
            let mut cfvalue = 0.0;
            for j in 0..self.private_info_set_size() {
                if i == j {
                    continue;
                }
                cfvalue += self.payoff(player, i, j) * pmi[j] / 6.0;
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
    fn payoff(&self, player: usize, my_card: usize, opp_card: usize) -> f64 {
        match self.public_info_set.last() {
            Some(action) if action == "Call" => {
                if my_card > opp_card {
                    2.0
                } else {
                    -2.0
                }
            }
            Some(action) if action == "Check" => {
                if my_card > opp_card {
                    1.0
                } else {
                    -1.0
                }
            }
            Some(action) if action == "Fold" => {
                if self.current_player() == player {
                    1.0
                } else {
                    -1.0
                }
            }
            _ => unreachable!(),
        }
    }
}

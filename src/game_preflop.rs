use crate::game_node::*;
use bincode::deserialize;
use once_cell::sync::Lazy;
use std::fs::File;
use std::io::Read;

static EQUITY_TABLE: Lazy<Vec<u32>> = Lazy::new(|| {
    let path = "static/heads_up_pre_flop_equity.bin";
    let mut infile = File::open(path).expect(&format!("could not open '{}'", path));
    let mut buf = Vec::new();
    infile.read_to_end(&mut buf).unwrap();
    deserialize::<Vec<u32>>(&buf).unwrap()
});

// 0 => Fold, 1 => Call, 2 => 3x bet, 3 => 4x bet, 4 => All in
#[derive(Clone, Debug)]
pub struct PreflopNode {
    prev_bet: f64,
    cur_bet: f64,
    eff_stack: f64,
    public_info_set: PublicInfoSet,
}

impl GameNode for PreflopNode {
    #[inline]
    fn is_terminal_node(&self) -> bool {
        match self.public_info_set.last() {
            Some(&0) => true,
            Some(&1) => self.public_info_set.len() >= 2,
            _ => false,
        }
    }

    #[inline]
    fn current_player(&self) -> usize {
        self.public_info_set.len() % 2
    }

    #[inline]
    fn num_actions(&self) -> usize {
        let ratio = self.eff_stack / self.cur_bet;
        let mut ret = 2;
        ret += (ratio > 1.0) as usize;
        ret += (ratio > 3.0) as usize;
        ret += (ratio > 4.0) as usize;
        ret
    }

    #[inline]
    fn actions(&self) -> std::ops::Range<usize> {
        0..self.num_actions()
    }

    #[inline]
    fn play(&self, action: usize) -> Self {
        let mut ret = self.clone();
        if action > 0 {
            ret.prev_bet = ret.cur_bet;
            ret.cur_bet *= match action {
                2 => 3.0,
                3 => 4.0,
                4 => self.eff_stack,
                _ => 1.0,
            };
            ret.cur_bet.max(self.eff_stack);
        }
        ret.public_info_set.push(action as u8);
        ret
    }

    #[inline]
    fn public_info_set(&self) -> &PublicInfoSet {
        &self.public_info_set
    }

    #[inline]
    fn private_info_set_len(&self) -> usize {
        52 * 51 / 2
    }

    #[inline]
    fn evaluate(&self, player: usize, pmi: &Vec<f64>) -> Vec<f64> {
        let prob = (2. * 2.) / (52. * 51. * 50. * 49.);
        let total = 2. * (48. * 47. * 46. * 45. * 44.) / (5. * 4. * 3. * 2.);

        // someone folded
        if self.public_info_set.last() == Some(&0) {
            let pmi_sum = pmi.iter().sum::<f64>();
            let mut pmi_sum_ex = [0.0; 52];

            let mut k = 0;
            for i in 0..51 {
                for j in (i + 1)..52 {
                    pmi_sum_ex[i] += pmi[k];
                    pmi_sum_ex[j] += pmi[k];
                    k += 1;
                }
            }

            let payoff = [self.prev_bet, -self.prev_bet][player ^ self.current_player()];

            let mut k = 0;
            let mut ret = Vec::with_capacity(self.private_info_set_len());
            for i in 0..51 {
                for j in (i + 1)..52 {
                    ret.push(payoff * prob * (pmi_sum - pmi_sum_ex[i] - pmi_sum_ex[j] + pmi[k]));
                    k += 1;
                }
            }

            return ret;
        }

        let mut k = 0;
        let mut ret = Vec::with_capacity(self.private_info_set_len());

        for i in 0..51 {
            for j in (i + 1)..52 {
                let k_start = k;
                let mut cfvalue = 0.0;
                for m in 0..51 {
                    for n in (m + 1)..52 {
                        if i == m || i == n || j == m || j == n {
                            k += 1;
                            continue;
                        }
                        let eq = EQUITY_TABLE[k] as f64 / total;
                        let eq_minus = 1.0 - eq;
                        let ev = self.cur_bet * (eq - eq_minus);
                        cfvalue += ev * pmi[k - k_start];
                        k += 1;
                    }
                }
                ret.push(cfvalue * prob);
            }
        }

        ret
    }
}

impl PreflopNode {
    #[inline]
    pub fn new(eff_stack: f64) -> Self {
        Self {
            prev_bet: 0.5,
            cur_bet: 1.0,
            eff_stack,
            public_info_set: Vec::new(),
        }
    }
}

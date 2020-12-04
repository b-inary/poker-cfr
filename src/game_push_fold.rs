use crate::cfr::*;
use bincode::deserialize;
use once_cell::sync::Lazy;
use std::fs::File;
use std::io::Read;

static EQUITY_TABLE: Lazy<Vec<u32>> = Lazy::new(|| {
    let path = "static/heads_up_pre_flop_equity.bin";
    let mut infile = File::open(path).expect(&format!("could not open '{}'", path));
    let mut buf = Vec::new();
    infile.read_to_end(&mut buf).unwrap();
    let decoded = deserialize::<Vec<u32>>(&buf).unwrap();
    decoded
});

#[derive(Clone, Debug)]
pub struct PushFoldNode {
    eff_stack: f64,
    public_info_set: PublicInfoSet,
}

impl GameNode for PushFoldNode {
    #[inline]
    fn is_terminal_node(&self) -> bool {
        match self.public_info_set.as_slice() {
            [0] => true,
            [_, _] => true,
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

            let payoff = match self.public_info_set.len() {
                1 => [-0.5, 0.5][player],
                _ => [1.0, -1.0][player],
            };

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
                        let ev = self.eff_stack * (eq - eq_minus);
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

impl PushFoldNode {
    #[inline]
    pub fn new(eff_stack: f64) -> Self {
        Self {
            eff_stack,
            public_info_set: Vec::new(),
        }
    }
}

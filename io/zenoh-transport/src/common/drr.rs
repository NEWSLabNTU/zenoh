use itertools::{chain, Itertools};

pub struct DRR {
    curr_slot: usize,
    consumed: bool,
    iter: Box<dyn Iterator<Item = usize> + Sync + Send>,
    quotas: Vec<usize>,
    weights: Vec<usize>,
}

impl DRR {
    pub fn new(weights: &[usize]) -> Self {
        let weights: Vec<_> = weights.to_vec();
        let w_min = weights
            .iter()
            .cloned()
            .min()
            .expect("weights must not be empty");
        assert!(w_min > 0, "weights must be nonzero");

        let mut quotas = vec![0; weights.len()];

        let mut indices: Vec<usize> = (0..weights.len()).collect();
        indices.sort_unstable_by_key(|&idx| (weights[idx], idx));

        let sorted_weights: Vec<_> = indices.iter().map(|&idx| weights[idx]).collect();
        indices.reverse();

        let iter = chain!([0], sorted_weights)
            .tuple_windows()
            .zip((1..=weights.len()).rev())
            .map(|((prev, next), slots)| {
                let rounds = next - prev;
                (slots, rounds)
            })
            .filter(|&(_, rounds)| rounds > 0)
            .flat_map(move |(slots, rounds)| {
                let mut selected: Vec<_> = indices[0..slots].to_vec();
                selected.sort_unstable();
                selected.into_iter().cycle().take(rounds * slots)
            })
            .cycle();
        let mut iter: Box<dyn Iterator<Item = usize> + Sync + Send> = Box::new(iter);

        let curr_slot = iter.next().unwrap();
        quotas[curr_slot] += weights[curr_slot];

        Self {
            iter,
            quotas,
            weights,
            curr_slot,
            consumed: false,
        }
    }

    pub fn next(&mut self) -> Next<'_> {
        if self.quotas[self.curr_slot] == 0 {
            self.step();
        }

        Next { drr: Some(self) }
    }

    fn step(&mut self) {
        self.curr_slot = self.iter.next().unwrap();
        self.quotas[self.curr_slot] += self.weights[self.curr_slot];
        self.consumed = false;
    }
}

pub struct Next<'a> {
    drr: Option<&'a mut DRR>,
}

impl<'a> Next<'a> {
    #[must_use]
    pub fn try_consume(mut self, amount: usize) -> bool {
        let drr = self.drr.take().unwrap();
        let quota = &mut drr.quotas[drr.curr_slot];

        let Some(remain) = quota.checked_sub(amount) else {
            drr.step();
            return false;
        };

        drr.consumed = true;
        *quota = remain;

        true
    }

    pub fn give_up(mut self) {
        let drr = self.drr.take().unwrap();

        if drr.consumed {
            let quota = &mut drr.quotas[drr.curr_slot];
            *quota = 0;
        }

        drr.step();
    }

    pub fn slot(&self) -> usize {
        self.drr.as_ref().unwrap().curr_slot
    }
}

impl<'a> Drop for Next<'a> {
    fn drop(&mut self) {
        let Some(drr) = self.drr.take() else {
            return;
        };

        if drr.consumed {
            let quota = &mut drr.quotas[drr.curr_slot];
            *quota = 0;
        }

        drr.step();
    }
}

#[cfg(test)]
mod tests {
    use super::DRR;

    #[test]
    fn drr_test() {
        let mut drr = DRR::new(&[1, 1, 1]);

        macro_rules! check_consume {
            ($slot:expr, $amount:expr, $result:expr) => {{
                let next = drr.next();
                assert_eq!(next.slot(), $slot);
                assert_eq!(next.try_consume($amount), $result);
            }};
        }

        macro_rules! check_giveup {
            ($slot:expr, $amount:expr) => {{
                let next = drr.next();
                assert_eq!(next.slot(), $slot);
                next.give_up();
            }};
        }

        check_consume!(0, 1, true);
        check_consume!(1, 1, true);
        check_consume!(2, 1, true);

        check_consume!(0, 1, true);
        check_consume!(1, 1, true);
        check_consume!(2, 1, true);

        check_giveup!(0, 1);
        check_consume!(1, 1, true);
        check_consume!(2, 1, true);

        check_consume!(0, 1, true);
        check_consume!(0, 1, true);
        check_consume!(1, 1, true);
        check_consume!(2, 1, true);
    }
}

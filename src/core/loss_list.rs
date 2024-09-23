use crate::utils::{SequenceNumber, SequenceRange};

#[derive(Debug)]
pub struct LossBuffer {
    lost_ranges: Vec<SequenceRange>,
}

impl Default for LossBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl LossBuffer {
    pub fn new() -> Self {
        let lost_ranges = Vec::new();
        Self { lost_ranges }
    }
    pub fn insert(&mut self, insert_range: SequenceRange) {
        self.lost_ranges = SequenceRange::combine_sequences(insert_range, self.lost_ranges.clone());
    }

    pub fn remove(&mut self, number: SequenceNumber) {
        let (overlap, mut safe): (Vec<_>, Vec<_>) = self
            .lost_ranges
            .clone()
            .into_iter()
            .partition(|range| range.start < number && range.stop > number);
        let spliced = overlap
            .iter()
            .flat_map(|range| {
                let mut lower = number;
                let mut upper = number;
                lower.dec();
                upper.inc();
                vec![
                    SequenceRange {
                        start: range.start,
                        stop: lower,
                    },
                    SequenceRange {
                        start: upper,
                        stop: range.stop,
                    },
                ]
            })
            .filter(|range| range.start <= range.stop)
            .collect::<Vec<_>>();
        safe.extend_from_slice(&spliced);
        self.lost_ranges = safe;
    }

    pub fn remove_range(&mut self, remove_range: SequenceRange) {
        let (overlap, mut safe): (Vec<_>, Vec<_>) =
            self.lost_ranges.clone().into_iter().partition(|range| {
                range.overlaps(remove_range)
            });
        let spliced = overlap
            .iter()
            .flat_map(|range| {
                let mut lower = remove_range.start;
                let mut upper = remove_range.stop;
                lower.dec();
                upper.inc();
                vec![
                    SequenceRange {
                        start: range.start,
                        stop: lower,
                    },
                    SequenceRange {
                        start: upper,
                        stop: range.stop,
                    },
                ]
            })
            .filter(|range| range.start <= range.stop)
            .collect::<Vec<_>>();
        safe.extend_from_slice(&spliced);
        self.lost_ranges = safe;
    }

    pub fn remove_confirmed(&mut self, confirmed: SequenceNumber) {
        self.lost_ranges
            .iter_mut()
            .for_each(|range| range.start = confirmed);
        self.lost_ranges.retain(|range| range.start <= range.stop);
    }

    pub fn size(&self) -> usize {
        self.lost_ranges.len()
    }

    pub fn find(&self, find_range: SequenceRange) -> Option<SequenceRange> {
        self.lost_ranges.clone().into_iter().find(|range| {
            range.overlaps(find_range)
        })
    }

    pub fn pop(&mut self) -> Option<SequenceRange> {
        self.lost_ranges
            .sort_unstable_by(|a, b| a.start.cmp(&b.start).reverse());
        self.lost_ranges.pop()
    }

    pub fn first(&mut self) -> Option<SequenceRange> {
        self.lost_ranges
            .sort_unstable_by(|a, b| a.start.cmp(&b.start).reverse());
        self.lost_ranges.first().copied()
    }

    pub fn encode(&self, mss: u16) -> Vec<SequenceRange> {
        let mut count = 0;
        let ranges = self.lost_ranges.iter();
        let mut out_ranges = Vec::new();
        let limit = mss / 2;
        for range in ranges {
            let cc = 2;
            if count + cc < limit {
                count += cc;
                out_ranges.push(*range)
            }
        }
        out_ranges
    }
}

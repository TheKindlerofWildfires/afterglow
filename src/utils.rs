use crate::serial::Serial;
use std::hash::Hash;
use std::ops::{AddAssign, SubAssign};
use std::{
    num::Wrapping,
    ops::{Add, Sub},
};
use std::fmt::Debug;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SequenceNumber(pub u16);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageNumber(pub u16);

impl SequenceNumber {
    const MAX: u16 = 0x7fff;
    pub const MAX_SEQ_NO: Self = SequenceNumber(Self::MAX);
    pub fn new(base: u16) -> Self {
        Self(base & Self::MAX)
    }
    pub fn inc(&mut self) {
        self.0 = (Wrapping(self.0).add(Wrapping(1))).0 & Self::MAX;
    }
    pub fn dec(&mut self) {
        self.0 = (Wrapping(self.0).sub(Wrapping(1))).0 & Self::MAX;
    }
    pub fn diff(&self, other: Self) -> u16 {
        (Wrapping(self.0).sub(Wrapping(other.0))).0 & Self::MAX
    }
    pub fn length(&self, other: &Self) -> u16 {
        if self.0 <= other.0 {
            other.0 - self.0 + 1
        } else {
            other.0 - self.0 + Self::MAX + 2
        }
    }
    pub fn probe_start(&self) -> bool {
        self.0 ^ 0xf == 0x0
    }
    pub fn probe_stop(&self) -> bool {
        self.0 ^ 0xf == 0x1
    }
}

impl Add for SequenceNumber{
    type Output = u16;

    fn add(self, other: Self) -> Self::Output {
        (Wrapping(self.0).add(Wrapping(other.0))).0 & Self::MAX
    }
}

impl AddAssign for SequenceNumber{
    fn add_assign(&mut self, other: Self) {
        self.0 = (Wrapping(self.0).sub(Wrapping(other.0))).0 & Self::MAX;

    }
}
impl Debug for SequenceNumber{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SequenceNumber")
            .field(&format_args!("{:x}", self.0))
            .finish()
    }
}

impl Sub for SequenceNumber{
    type Output = u16;

    fn sub(self, other: Self) -> Self::Output {
        (Wrapping(self.0).sub(Wrapping(other.0))).0 & Self::MAX
    }
}

impl SubAssign for SequenceNumber{
    fn sub_assign(&mut self, other: Self) {
        self.0 = (Wrapping(self.0).sub(Wrapping(other.0))).0 & Self::MAX;

    }
}

impl Serial for SequenceNumber {
    fn serialize(&self) -> Vec<u8> {
        self.0.serialize()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let out = u16::deserialize(bytes, start) & Self::MAX;
        Self(out)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SequenceRange {
    pub start: SequenceNumber,
    pub stop: SequenceNumber,
}

impl SequenceRange {
    pub fn contains(&self, number: SequenceNumber) -> bool {
        self.start < number && self.stop > number
    }
    pub fn combine_sequences(
        insert_range: SequenceRange,
        ranges: Vec<SequenceRange>,
    ) -> Vec<SequenceRange> {
        let (mut overlap, mut safe): (Vec<_>, Vec<_>) = ranges.into_iter().partition(|range| {
            (range.start < insert_range.start && range.stop > insert_range.start)
                || (range.stop > insert_range.stop && range.start < insert_range.start)
                || (range.start > insert_range.start && range.stop < insert_range.stop)
        });

        //sort the overlaps
        overlap.sort_unstable_by(|a, b| a.start.cmp(&b.start));

        //merge the overlaps
        let mut merged_ranges: Vec<SequenceRange> = Vec::new();
        for range in overlap {
            if let Some(last_range) = merged_ranges.last_mut() {
                if range.start <= last_range.stop {
                    last_range.stop = last_range.stop.max(range.stop);
                } else {
                    merged_ranges.push(range)
                }
            } else {
                merged_ranges.push(range)
            }
        }
        //recombine the list
        safe.extend_from_slice(&merged_ranges);
        safe
    }
}

impl Serial for SequenceRange {
    fn serialize(&self) -> Vec<u8> {
        let mut bytes = self.start.serialize();
        bytes.extend_from_slice(&self.stop.serialize());
        bytes
    }

    fn deserialize(bytes: &[u8], idx: &mut usize) -> Self {
        let start = SequenceNumber::deserialize(bytes, idx);
        let stop = SequenceNumber::deserialize(bytes, idx);
        Self { start, stop }
    }
}

impl MessageNumber {
    const MAX: u16 = 0x1fff;
    pub const ZERO: Self = Self(0);
    pub fn new(base: u16) -> Self {
        Self(base & Self::MAX)
    }
    pub fn inc(&mut self) {
        self.0 = (Wrapping(self.0).add(Wrapping(1))).0 & Self::MAX;
    }
    pub fn dec(&mut self) {
        self.0 = (Wrapping(self.0).sub(Wrapping(1))).0 & Self::MAX;
    }
    pub fn add(&mut self, other: u16) {
        self.0 = (Wrapping(self.0).add(Wrapping(other))).0 & Self::MAX;
    }
    pub fn length(&self, other: &Self) -> u16 {
        if self.0 <= other.0 {
            other.0 - self.0 + 1
        } else {
            other.0 - self.0 + Self::MAX + 2
        }
    }
}

impl Serial for MessageNumber {
    fn serialize(&self) -> Vec<u8> {
        self.0.serialize()
    }

    fn deserialize(bytes: &[u8], start: &mut usize) -> Self {
        let out = u16::deserialize(bytes, start) & Self::MAX;
        Self(out)
    }
}
pub fn hash(value: usize)->usize{
    let state = Wrapping(value)*Wrapping(747796405)+Wrapping(2891336453);
    let word = ((state>>((state>>28)+Wrapping(4)).0)^state)*Wrapping(277803737);
    ((word>>22)^word).0
}
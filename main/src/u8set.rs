use itertools::Itertools;
use std::fmt::{Debug, Display, Formatter};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct U8BitSet([u64; 4]);

#[allow(unused)]
impl U8BitSet {
    fn limb_and_bit(value: u8) -> (usize, u64) {
        ((value >> 6) as usize, 1u64 << (value & 63))
    }
    pub fn contains(&self, value: u8) -> bool {
        let (limb, bit) = Self::limb_and_bit(value);
        self.0[limb] & bit != 0
    }

    pub fn insert(&mut self, value: u8) {
        let (limb, bit) = Self::limb_and_bit(value);
        self.0[limb] |= bit;
    }

    pub fn extend(&mut self, other: &U8BitSet) {
        self.0
            .iter_mut()
            .zip(other.0)
            .for_each(|(self_limb, other_limb)| *self_limb |= other_limb)
    }

    pub fn union(&self, other: &U8BitSet) -> U8BitSet {
        let mut union = *self;
        union.extend(other);
        union
    }

    pub fn remove_not_in(&mut self, other: &U8BitSet) {
        self.0
            .iter_mut()
            .zip(other.0)
            .for_each(|(self_limb, other_limb)| *self_limb &= other_limb)
    }

    pub fn intersect(&self, other: &U8BitSet) -> U8BitSet {
        let mut intersection = *self;
        intersection.remove_not_in(other);
        intersection
    }

    pub fn len(&self) -> usize {
        self.0.iter().copied().map(u64::count_ones).sum::<u32>() as usize
    }
    pub fn new() -> Self {
        Self([0, 0, 0, 0])
    }
    pub fn all_u8s() -> Self {
        Self([u64::MAX, u64::MAX, u64::MAX, u64::MAX])
    }
    pub fn clear(&mut self) {
        self.0 = [0, 0, 0, 0];
    }
}

impl Display for U8BitSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.into_iter().map(|x| u8::to_string(&x)).join(","))
    }
}

impl Debug for U8BitSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

pub struct U8BitIter {
    set: U8BitSet,
    index: usize,
}

impl Iterator for U8BitIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index <= u8::MAX as usize {
            if self.set.contains(self.index as u8) {
                let current = self.index;
                self.index += 1;
                return Some(current as u8);
            } else {
                self.index += 1;
            }
        }
        None
    }
}

impl IntoIterator for U8BitSet {
    type Item = u8;
    type IntoIter = U8BitIter;

    fn into_iter(self) -> Self::IntoIter {
        U8BitIter {
            set: self,
            index: 0,
        }
    }
}

impl FromIterator<u8> for U8BitSet {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        let mut result = U8BitSet::new();
        iter.into_iter().for_each(|val| result.insert(val));
        result
    }
}

use std::num::Zero;
use std::ops::{Add, Sub, Neg};
use std::collections::HashMap;
use std::hash::Hash;

use core::{Operation, StateRDT};

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct GCounter<K: Hash + Eq + Clone, T: Ord + Add<T, Output=T> + Copy> {
    my_id: K,
    counters: HashMap<K, T>,
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct SetGCounterOperation<K, T> {
    id: K,
    value: T,
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct PNCounter<K: Hash + Eq + Clone, T: Ord + Add<T, Output=T> + Sub<T, Output=T> + Copy> {
    my_id: K,
    pos_counters: HashMap<K, T>,
    neg_counters: HashMap<K, T>,
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct SetPNCounterOperation<K, T> {
    id: K,
    pos_value: T,
    neg_value: T,
}

impl<K: Hash + Eq + Clone, T: Zero + Ord + Copy + Add<T, Output=T>> GCounter<K, T> {
    pub fn new(my_id: K) -> Self {
        GCounter {
            counters: HashMap::new(),
            my_id: my_id,
        }
    }

    pub fn value(&self) -> T {
        self.counters.values().cloned().sum()
    }

    pub fn add(&mut self, value: T) -> Option<SetGCounterOperation<K, T>> {
        if value < Zero::zero() {
            return None;
        }

        let op = SetGCounterOperation {
            id: self.my_id.clone(),
            value: self.value() + value,
        };

        op.apply(self);

        Some(op)
    }
}

impl<K: Hash + Eq + Clone, T: Neg<Output=T> + Zero + Ord + Copy + Add<T, Output=T> + Sub<T, Output=T>> PNCounter<K, T> {
    pub fn new(my_id: K) -> Self {
        PNCounter {
            my_id: my_id,
            pos_counters: HashMap::new(),
            neg_counters: HashMap::new(),
        }
    }

    pub fn value(&self) -> T {
        let pos: T = self.pos_counters.values().cloned().sum();
        let neg: T = self.neg_counters.values().cloned().sum();

        pos - neg
    }

    pub fn add(&mut self, value: T) -> Option<SetPNCounterOperation<K, T>> {
        let op = if value >= Zero::zero() {
            SetPNCounterOperation {
                id: self.my_id.clone(),
                pos_value: value,
                neg_value: Zero::zero(),
            }
        }
        else {
            SetPNCounterOperation {
                id: self.my_id.clone(),
                pos_value: Zero::zero(),
                neg_value: -value
            }
        };

        op.apply(self);

        Some(op)
    }
}

impl<K: Hash + Eq + Clone, T: Zero + Ord + Copy + Add<T, Output=T>> Operation<GCounter<K, T>> for SetGCounterOperation<K, T> {
    fn apply(&self, target: &mut GCounter<K, T>) {
        let cur_value = target.counters.get(&self.id).cloned().unwrap_or(Zero::zero());

        target.counters.insert(
            self.id.clone(),
            *vec![self.value, cur_value].iter().max().unwrap());
    }
}

impl<K: Hash + Eq + Clone, T: Zero + Ord + Copy + Add<T, Output=T>> StateRDT for GCounter<K, T> {
    fn merge(&mut self, other: &Self) {
        for (id, &value) in &other.counters {
            let cur_value = self.counters.get(id).cloned().unwrap_or(Zero::zero());

            self.counters.insert(
                id.clone(),
                *vec![cur_value, value].iter().max().unwrap());
        }
    }
}

impl<K: Hash + Eq + Clone, T: Zero + Ord + Copy + Add<T, Output=T> + Sub<T, Output=T>> Operation<PNCounter<K, T>> for SetPNCounterOperation<K, T> {
    fn apply(&self, target: &mut PNCounter<K, T>) {
        let cur_pos_value = target.pos_counters.get(&self.id).cloned().unwrap_or(Zero::zero());
        let cur_neg_value = target.neg_counters.get(&self.id).cloned().unwrap_or(Zero::zero());

        target.pos_counters.insert(
            self.id.clone(),
            *vec![self.pos_value, cur_pos_value].iter().max().unwrap());
        target.neg_counters.insert(
            self.id.clone(),
            *vec![self.neg_value, cur_neg_value].iter().max().unwrap());
    }
}

impl<K: Hash + Eq + Clone, T: Zero + Ord + Copy + Add<T, Output=T> + Sub<T, Output=T>> StateRDT for PNCounter<K, T> {
    fn merge(&mut self, other: &Self) {
        for (id, &pos_value) in &other.pos_counters {
            let cur_pos_value = self.pos_counters.get(id).cloned().unwrap_or(Zero::zero());

            self.pos_counters.insert(
                id.clone(),
                *vec![cur_pos_value, pos_value].iter().max().unwrap());
        }

        for (id, &neg_value) in &other.neg_counters {
            let cur_neg_value = self.neg_counters.get(id).cloned().unwrap_or(Zero::zero());

            self.neg_counters.insert(
                id.clone(),
                *vec![cur_neg_value, neg_value].iter().max().unwrap());
        }
    }
}

#[cfg(test)]
mod test {
    use super::{GCounter, PNCounter};
    use core::{Operation, StateRDT};

    #[test]
    fn make_g_counter() {
        let counter : GCounter<&'static str, i32> = GCounter::new("h1");

        assert_eq!(counter.value(), 0);
    }

    #[test]
    fn increment_g_counter() {
        let mut counter = GCounter::new("h1");

        counter.add(10).unwrap();

        assert_eq!(counter.value(), 10);
    }

    #[test]
    fn decrement_g_counter() {
        let mut counter = GCounter::new("h1");

        assert!(counter.add(-10).is_none());
        assert_eq!(counter.value(), 0);
    }

    #[test]
    fn apply_g_counter_increments() {
        let mut c1 = GCounter::new("h1");
        let mut c2 = GCounter::new("h2");

        let op1 = c1.add(5).unwrap();
        let op2 = c2.add(7).unwrap();

        op2.apply(&mut c1);
        op1.apply(&mut c2);

        assert_eq!(c1.value(), 12);
        assert_eq!(c2.value(), 12);
    }

    #[test]
    fn merge_g_counter_state() {
        let mut c1 = GCounter::new("h1");
        let mut c2 = GCounter::new("h2");

        c1.add(5);
        c2.add(7);

        c1.merge(&c2);
        c2.merge(&c1);

        assert_eq!(c1.value(), 12);
        assert_eq!(c2.value(), 12);
    }


    #[test]
    fn make_pn_counter() {
        let counter : PNCounter<&'static str, i32> = PNCounter::new("h1");

        assert_eq!(counter.value(), 0);
    }

    #[test]
    fn increment_pn_counter() {
        let mut counter = PNCounter::new("h1");

        counter.add(10).unwrap();

        assert_eq!(counter.value(), 10);
    }

    #[test]
    fn decrement_pn_counter() {
        let mut counter = PNCounter::new("h1");

        counter.add(-10).unwrap();

        assert_eq!(counter.value(), -10);
    }

    #[test]
    fn apply_pn_counter_increments() {
        let mut c1 = PNCounter::new("h1");
        let mut c2 = PNCounter::new("h2");

        let op1 = c1.add(5).unwrap();
        let op2 = c2.add(-7).unwrap();

        op2.apply(&mut c1);
        op1.apply(&mut c2);

        assert_eq!(c1.value(), -2);
        assert_eq!(c2.value(), -2);
    }

    #[test]
    fn merge_pn_counter_state() {
        let mut c1 = PNCounter::new("h1");
        let mut c2 = PNCounter::new("h2");

        c1.add(5);
        c2.add(-7);

        c1.merge(&c2);
        c2.merge(&c1);

        assert_eq!(c1.value(), -2);
        assert_eq!(c2.value(), -2);
    }
}

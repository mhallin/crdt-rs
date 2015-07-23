use std::default::Default;
use std::ops::Add;
use std::collections::HashMap;
use std::hash::Hash;

use core::{Operation, StateRDT};

#[derive(Debug)]
pub struct OpCounter<T: Add<T, Output=T> + Copy> {
    counter: T,
}

#[derive(Debug)]
pub struct IncrementOperation<T: Add<T, Output=T> + Copy> {
    value: T,
}

#[derive(Debug)]
pub struct GCounter<K: Hash + Eq + Clone, T: Add<T, Output=T> + Copy> {
    my_id: K,
    counters: HashMap<K, T>,
}

#[derive(Debug)]
pub struct SetCounterOperation<K, T> {
    id: K,
    value: T,
}

impl<T: Add<T, Output=T> + Copy + Default> OpCounter<T> {
    pub fn new() -> Self {
        OpCounter { counter: Default::default() }
    }

    pub fn value(&self) -> T {
        self.counter
    }

    pub fn add(&mut self, value: T) -> IncrementOperation<T> {
        let op = IncrementOperation { value: value };

        op.apply(self);

        op
    }
}

impl<K: Hash + Eq + Clone, T: Ord + Copy + Default + Add<T, Output=T>> GCounter<K, T> {
    pub fn new(my_id: K) -> Self {
        GCounter {
            counters: HashMap::new(),
            my_id: my_id,
        }
    }

    pub fn value(&self) -> T {
        let mut val = Default::default();

        for v in self.counters.values() {
            val = val + *v;
        }

        val
    }

    pub fn add(&mut self, value: T) -> SetCounterOperation<K, T> {
        let op = SetCounterOperation {
            id: self.my_id.clone(),
            value: self.value() + value,
        };

        op.apply(self);

        op
    }
}

impl<T: Add<T, Output=T> + Copy> Operation<OpCounter<T>> for IncrementOperation<T> {
    fn apply(&self, target: &mut OpCounter<T>) {
        target.counter = target.counter + self.value;
    }
}

impl<K: Hash + Eq + Clone, T: Ord + Copy + Default + Add<T, Output=T>> Operation<GCounter<K, T>> for SetCounterOperation<K, T> {
    fn apply(&self, target: &mut GCounter<K, T>) {
        let default = Default::default();
        let cur_value = *target.counters.get(&self.id).unwrap_or(&default);

        target.counters.insert(
            self.id.clone(),
            *vec![self.value, cur_value].iter().max().unwrap());
    }
}

impl<K: Hash + Eq + Clone, T: Ord + Copy + Default + Add<T, Output=T>> StateRDT for GCounter<K, T> {
    fn merge(&mut self, other: &Self) {
        let default = Default::default();

        for (id, &value) in &other.counters {
            let cur_value = *self.counters.get(id).unwrap_or(&default);

            self.counters.insert(
                id.clone(),
                *vec![cur_value, value].iter().max().unwrap());
        }
    }
}

#[cfg(test)]
mod test {
    use super::{OpCounter, GCounter};
    use core::{Operation, StateRDT};

    #[test]
    fn make_op_counter() {
        let counter : OpCounter<i32> = OpCounter::new();

        assert_eq!(counter.value(), 0);
    }

    #[test]
    fn increment_op_counter() {
        let mut counter = OpCounter::new();

        counter.add(10);

        assert_eq!(counter.value(), 10);
    }

    #[test]
    fn apply_op_counter_increments() {
        let mut c1 = OpCounter::new();
        let mut c2 = OpCounter::new();

        let op1 = c1.add(5);
        let op2 = c2.add(7);

        op2.apply(&mut c1);
        op1.apply(&mut c2);

        assert_eq!(c1.value(), 12);
        assert_eq!(c2.value(), 12);
    }

    #[test]
    fn make_g_counter() {
        let counter : GCounter<&'static str, i32> = GCounter::new("h1");

        assert_eq!(counter.value(), 0);
    }

    #[test]
    fn increment_g_counter() {
        let mut counter = GCounter::new("h1");

        counter.add(10);

        assert_eq!(counter.value(), 10);
    }

    #[test]
    fn apply_g_counter_increments() {
        let mut c1 = GCounter::new("h1");
        let mut c2 = GCounter::new("h2");

        let op1 = c1.add(5);
        let op2 = c2.add(7);

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
}

use std::collections::HashSet;
use std::hash::Hash;

use core::{Operation, StateRDT};

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct GSet<T: Hash + Eq + Clone> {
    set: HashSet<T>
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct AddGSetOperation<T> {
    value: T
}

impl<T: Hash + Eq + Clone> GSet<T> {
    pub fn new() -> GSet<T> {
        GSet {
            set: HashSet::new(),
        }
    }

    pub fn value<'a>(&'a self) -> &'a HashSet<T> {
        &self.set
    }

    pub fn add(&mut self, value: T) -> Option<AddGSetOperation<T>> {
        if self.set.contains(&value) {
            return None;
        }

        let op = AddGSetOperation { value: value };

        op.apply(self);

        Some(op)
    }
}

impl<T: Hash + Eq + Clone> Operation<GSet<T>> for AddGSetOperation<T> {
    fn apply(&self, target: &mut GSet<T>) {
        target.set.insert(self.value.clone());
    }
}

impl<T: Hash + Eq + Clone> StateRDT for GSet<T> {
    fn merge(&mut self, other: &GSet<T>) {
        self.set = self.set.union(&other.set).cloned().collect();
    }
}

#[cfg(test)]
mod test {
    use super::GSet;

    use std::collections::HashSet;
    use std::iter::FromIterator;

    use core::{Operation, StateRDT};

    #[test]
    fn make_g_set() {
        let set: GSet<i32> = GSet::new();

        assert_eq!(*set.value(), HashSet::new());
    }

    #[test]
    fn add_g_set() {
        let mut set = GSet::new();

        set.add(123).unwrap();

        assert_eq!(*set.value(), HashSet::from_iter(vec![123]));
    }

    #[test]
    fn apply_g_set_add() {
        let mut s1 = GSet::new();
        let mut s2 = GSet::new();

        let op1 = s1.add(123).unwrap();
        let op2 = s2.add(456).unwrap();

        op2.apply(&mut s1);
        op1.apply(&mut s2);

        assert_eq!(*s1.value(), HashSet::from_iter(vec![123, 456]));
        assert_eq!(*s2.value(), HashSet::from_iter(vec![123, 456]));
    }

    #[test]
    fn merge_lww_register_state() {
        let mut s1 = GSet::new();
        let mut s2 = GSet::new();

        s1.add(123).unwrap();
        s2.add(456).unwrap();

        s1.merge(&s2);
        s2.merge(&s1);

        assert_eq!(*s1.value(), HashSet::from_iter(vec![123, 456]));
        assert_eq!(*s2.value(), HashSet::from_iter(vec![123, 456]));
    }
}

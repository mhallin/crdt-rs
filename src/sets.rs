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

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct TwoPhaseSet<T: Hash + Eq + Clone> {
    members: HashSet<T>,
    tombstones: HashSet<T>,
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct Add2PSetOperation<T> {
    value: T,
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct Remove2PSetOperation<T> {
    value: T,
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

impl<T: Hash + Eq + Clone> TwoPhaseSet<T> {
    pub fn new() -> TwoPhaseSet<T> {
        TwoPhaseSet {
            members: HashSet::new(),
            tombstones: HashSet::new(),
        }
    }

    pub fn value(&self) -> HashSet<T> {
        self.members.difference(&self.tombstones).cloned().collect()
    }

    pub fn add(&mut self, value: T) -> Option<Add2PSetOperation<T>> {
        if self.value().contains(&value) {
            return None;
        }

        let op = Add2PSetOperation { value: value };

        op.apply(self);

        Some(op)
    }

    pub fn remove(&mut self, value: T) -> Option<Remove2PSetOperation<T>> {
        if !self.value().contains(&value) {
            return None;
        }

        let op = Remove2PSetOperation { value: value };

        op.apply(self);

        Some(op)
    }
}

impl<T: Hash + Eq + Clone> Operation<TwoPhaseSet<T>> for Add2PSetOperation<T> {
    fn apply(&self, target: &mut TwoPhaseSet<T>) {
        target.members.insert(self.value.clone());
    }
}

impl<T: Hash + Eq + Clone> Operation<TwoPhaseSet<T>> for Remove2PSetOperation<T> {
    fn apply(&self, target: &mut TwoPhaseSet<T>) {
        target.tombstones.insert(self.value.clone());
    }
}

impl<T: Hash + Eq + Clone> StateRDT for TwoPhaseSet<T> {
    fn merge(&mut self, other: &TwoPhaseSet<T>) {
        self.members = self.members.union(&other.members).cloned().collect();
        self.tombstones = self.tombstones.union(&other.tombstones).cloned().collect();
    }
}

#[cfg(test)]
mod test {
    use super::{GSet, TwoPhaseSet};

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
    fn merge_g_set() {
        let mut s1 = GSet::new();
        let mut s2 = GSet::new();

        s1.add(123).unwrap();
        s2.add(456).unwrap();

        s1.merge(&s2);
        s2.merge(&s1);

        assert_eq!(*s1.value(), HashSet::from_iter(vec![123, 456]));
        assert_eq!(*s2.value(), HashSet::from_iter(vec![123, 456]));
    }

    #[test]
    fn make_2p_set() {
        let set: TwoPhaseSet<i32> = TwoPhaseSet::new();

        assert_eq!(set.value(), HashSet::new());
    }

    #[test]
    fn add_2p_set() {
        let mut set = TwoPhaseSet::new();

        set.add(123).unwrap();

        assert_eq!(set.value(), HashSet::from_iter(vec![123]));
    }

    #[test]
    fn remove_2p_set() {
        let mut set = TwoPhaseSet::new();

        set.add(123).unwrap();
        set.add(456).unwrap();
        set.remove(123).unwrap();

        assert_eq!(set.value(), HashSet::from_iter(vec![456]));
    }

    #[test]
    fn apply_2p_set_ops() {
        let mut s1 = TwoPhaseSet::new();
        let mut s2 = TwoPhaseSet::new();

        let op1 = s1.add(123).unwrap();
        let op2 = s2.add(456).unwrap();

        op2.apply(&mut s1);
        op1.apply(&mut s2);

        assert_eq!(s1.value(), HashSet::from_iter(vec![123, 456]));
        assert_eq!(s2.value(), HashSet::from_iter(vec![123, 456]));

        let op3 = s1.remove(456).unwrap();

        op3.apply(&mut s2);

        assert_eq!(s1.value(), HashSet::from_iter(vec![123]));
        assert_eq!(s2.value(), HashSet::from_iter(vec![123]));
    }

    #[test]
    fn merge_2p_set() {
        let mut s1 = TwoPhaseSet::new();
        let mut s2 = TwoPhaseSet::new();

        s1.add(123).unwrap();
        s2.add(456).unwrap();

        s1.merge(&s2);
        s2.merge(&s1);

        assert_eq!(s1.value(), HashSet::from_iter(vec![123, 456]));
        assert_eq!(s2.value(), HashSet::from_iter(vec![123, 456]));

        s1.remove(456).unwrap();

        s2.merge(&s1);

        assert_eq!(s1.value(), HashSet::from_iter(vec![123]));
        assert_eq!(s2.value(), HashSet::from_iter(vec![123]));
    }
}

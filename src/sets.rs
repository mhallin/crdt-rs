use std::collections::{HashSet, HashMap};
use std::hash::Hash;

use uuid::Uuid;

use core::{StateRDT, OperationRDT};

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct GSet<T: Hash + Eq + Clone> {
    set: HashSet<T>
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct AddGSetOperation<T>(T);

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct TwoPhaseSet<T: Hash + Eq + Clone> {
    members: HashSet<T>,
    tombstones: HashSet<T>,
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub enum TwoPhaseSetOperation<T: Hash + Eq + Clone> {
    Add(T),
    Remove(T),
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct ObserveRemoveSet<T: Hash + Eq + Clone> {
    members: HashMap<T, HashSet<Uuid>>,
    tombstones: HashSet<Uuid>,
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub enum ORSetOperation<T> {
    Add(T, Uuid),
    Remove(HashSet<Uuid>),
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

        let op = AddGSetOperation(value);

        self.apply(&op);

        Some(op)
    }
}

impl<T: Hash + Eq + Clone> OperationRDT for GSet<T> {
    type Operation = AddGSetOperation<T>;

    fn apply(&mut self, op: &Self::Operation) {
        let &AddGSetOperation(ref value) = op;

        self.set.insert(value.clone());
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

    pub fn add(&mut self, value: T) -> Option<TwoPhaseSetOperation<T>> {
        if self.value().contains(&value) {
            return None;
        }

        let op = TwoPhaseSetOperation::Add(value);

        self.apply(&op);

        Some(op)
    }

    pub fn remove(&mut self, value: T) -> Option<TwoPhaseSetOperation<T>> {
        if !self.value().contains(&value) {
            return None;
        }

        let op = TwoPhaseSetOperation::Remove(value);

        self.apply(&op);

        Some(op)
    }
}

impl<T: Hash + Eq + Clone> OperationRDT for TwoPhaseSet<T> {
    type Operation = TwoPhaseSetOperation<T>;

    fn apply(&mut self, op: &Self::Operation) {
        use self::TwoPhaseSetOperation::{Add, Remove};

        match op {
            &Add(ref value) => self.members.insert(value.clone()),
            &Remove(ref value) => self.tombstones.insert(value.clone()),
        };
    }
}

impl<T: Hash + Eq + Clone> StateRDT for TwoPhaseSet<T> {
    fn merge(&mut self, other: &TwoPhaseSet<T>) {
        self.members = self.members.union(&other.members).cloned().collect();
        self.tombstones = self.tombstones.union(&other.tombstones).cloned().collect();
    }
}

impl<T: Hash + Eq + Clone> ObserveRemoveSet<T> {
    pub fn new() -> ObserveRemoveSet<T> {
        ObserveRemoveSet {
            members: HashMap::new(),
            tombstones: HashSet::new(),
        }
    }

    pub fn value(&self) -> HashSet<T> {
        self.members
            .iter()
            .filter(|&(_,v)| !v.is_subset(&self.tombstones))
            .map(|(k,_)| k)
            .cloned()
            .collect()
    }

    pub fn add(&mut self, value: T) -> ORSetOperation<T> {
        let op = ORSetOperation::Add(value, Uuid::new_v4());

        self.apply(&op);

        op
    }

    pub fn remove(&mut self, value: T) -> Option<ORSetOperation<T>> {
        if !self.members.contains_key(&value) {
            return None
        }

        let keys = self.members[&value].clone();
        let op = ORSetOperation::Remove(keys);

        self.apply(&op);

        Some(op)
    }
}

impl<T: Hash + Eq + Clone> OperationRDT for ObserveRemoveSet<T> {
    type Operation = ORSetOperation<T>;

    fn apply(&mut self, op: &Self::Operation) {
        use self::ORSetOperation::{Add, Remove};

        match op {
            &Add(ref value, ref id) => {
                let ids = self.members.entry(value.clone()).or_insert(HashSet::new());
                ids.insert(id.clone());
            },
            &Remove(ref uuids) => {
                self.tombstones = self.tombstones.union(uuids).cloned().collect();
            },
        }
    }
}

impl<T: Hash + Eq + Clone> StateRDT for ObserveRemoveSet<T> {
    fn merge(&mut self, other: &Self) {
        use std::collections::hash_map::Entry;

        for (value, ids) in &other.members {
            match self.members.entry(value.clone()) {
                Entry::Vacant(e) => {
                    e.insert(ids.clone());
                },
                Entry::Occupied(mut e) => {
                    let u = e.get().union(&ids).cloned().collect();
                    e.insert(u);
                },
            }
        }

        self.tombstones = self.tombstones.union(&other.tombstones).cloned().collect();
    }
}

#[cfg(test)]
mod test {
    use super::{GSet, TwoPhaseSet, ObserveRemoveSet};

    use std::collections::HashSet;
    use std::iter::FromIterator;

    use core::{StateRDT, OperationRDT};

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

        s1.apply(&op2);
        s2.apply(&op1);

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

        s1.apply(&op2);
        s2.apply(&op1);

        assert_eq!(s1.value(), HashSet::from_iter(vec![123, 456]));
        assert_eq!(s2.value(), HashSet::from_iter(vec![123, 456]));

        let op3 = s1.remove(456).unwrap();

        s2.apply(&op3);

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

    #[test]
    fn make_or_set() {
        let set: ObserveRemoveSet<i32> = ObserveRemoveSet::new();

        assert_eq!(set.value(), HashSet::new());
    }

    #[test]
    fn add_or_set() {
        let mut set = ObserveRemoveSet::new();

        set.add(123);

        assert_eq!(set.value(), HashSet::from_iter(vec![123]));
    }

    #[test]
    fn remove_or_set() {
        let mut set = ObserveRemoveSet::new();

        set.add(123);
        set.add(456);
        set.remove(123).unwrap();

        assert_eq!(set.value(), HashSet::from_iter(vec![456]));
    }

    #[test]
    fn apply_or_set_ops() {
        let mut s1 = ObserveRemoveSet::new();
        let mut s2 = ObserveRemoveSet::new();

        let op1 = s1.add(123);
        let op2 = s2.add(123);
        let op3 = s1.add(456);

        s1.apply(&op2);
        s2.apply(&op1);
        s2.apply(&op3);

        assert_eq!(s1.value(), HashSet::from_iter(vec![123, 456]));
        assert_eq!(s1.value(), HashSet::from_iter(vec![123, 456]));

        let op4 = s1.remove(456).unwrap();
        let op5 = s2.add(456);

        s2.apply(&op4);
        s1.apply(&op5);

        assert_eq!(s1.value(), HashSet::from_iter(vec![123, 456]));
        assert_eq!(s1.value(), HashSet::from_iter(vec![123, 456]));
    }

    #[test]
    fn merge_or_set_ops() {
        let mut s1 = ObserveRemoveSet::new();
        let mut s2 = ObserveRemoveSet::new();

        s1.add(123);
        s2.add(123);
        s1.add(456);

        s1.merge(&s2);
        s2.merge(&s1);

        assert_eq!(s1.value(), HashSet::from_iter(vec![123, 456]));
        assert_eq!(s1.value(), HashSet::from_iter(vec![123, 456]));

        s1.remove(456).unwrap();
        s2.add(456);

        s2.merge(&s1);
        s1.merge(&s2);

        assert_eq!(s1.value(), HashSet::from_iter(vec![123, 456]));
        assert_eq!(s1.value(), HashSet::from_iter(vec![123, 456]));
    }
}

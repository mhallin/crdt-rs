use std::collections::HashMap;
use std::hash::Hash;

use core::{StateRDT, OperationRDT};

use sets::{ObserveRemoveSet, ORSetOperation};

pub struct ObserveRemoveMap<'a, K: Hash + Eq + Clone, V: OperationRDT> {
    keys: ObserveRemoveSet<K>,
    values: HashMap<K, V>,
    value_ctor: Box<Fn() -> V + 'a>,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct ORMapOperation<K, V: OperationRDT> {
    key_op: Option<ORSetOperation<K>>,
    value_op: Option<(K, V::Operation)>,
}

impl<'a, K, V> ObserveRemoveMap<'a, K, V>
    where K: Hash + Eq + Clone,
          V: OperationRDT
{
    pub fn new<F>(ctor: F) -> ObserveRemoveMap<'a, K, V>
        where F: Fn() -> V + 'a
    {
        ObserveRemoveMap {
            keys: ObserveRemoveSet::new(),
            values: HashMap::new(),
            value_ctor: Box::new(ctor),
        }
    }

    pub fn get(&'a self, key: &K) -> Option<&'a V> {
        return if self.keys.value().contains(key) {
            self.values.get(key)
        }
        else {
            None
        }
    }

    pub fn update<F>(&mut self, key: K, update_fn: F) -> Option<ORMapOperation<K, V>>
        where F: FnOnce(&mut V) -> Option<V::Operation>
    {
        let key_op = if self.keys.value().contains(&key) {
            None
        }
        else {
            Some(self.keys.add(key.clone()))
        };

        let value = self.values.entry(key.clone()).or_insert((*self.value_ctor)());
        let value_op = update_fn(value);

        if key_op.is_some() || value_op.is_some() {
            Some(ORMapOperation { key_op: key_op, value_op: Some((key, value_op.unwrap())) })
        }
        else {
            None
        }
    }
}

impl<'a, K, V> OperationRDT for ObserveRemoveMap<'a, K, V>
    where K: Hash + Eq + Clone,
          V: OperationRDT
{
    type Operation = ORMapOperation<K, V>;

    fn apply(&mut self, op: &Self::Operation) {
        if let Some(ref key_op) = op.key_op {
            self.keys.apply(key_op);
        }

        if let Some((ref key, ref value_op)) = op.value_op {
            let value = self.values.entry(key.clone()).or_insert((*self.value_ctor)());
            value.apply(value_op);
        }
    }
}

impl<'a, K, V> StateRDT for ObserveRemoveMap<'a, K, V>
    where K: Hash + Eq + Clone,
          V: OperationRDT + StateRDT
{
    fn merge(&mut self, other: &ObserveRemoveMap<'a, K, V>) {
        self.keys.merge(&other.keys);

        for (key, ref value) in &other.values {
            let my_value = self.values.entry(key.clone()).or_insert((*self.value_ctor)());
            my_value.merge(value);
        }
    }
}

#[cfg(test)]
mod test {
    use super::ObserveRemoveMap;

    use core::{StateRDT, OperationRDT};
    use counters::PNCounter;

    #[test]
    fn make_counter_map() {
        let m: ObserveRemoveMap<&str, PNCounter<&str, i32>> =
            ObserveRemoveMap::new(|| PNCounter::new("h1"));

        assert!(m.get(&"c1").is_none());
    }

    #[test]
    fn add_counter_map() {
        let mut m = ObserveRemoveMap::new(|| PNCounter::new("h1"));

        m.update("c1", |mut c| c.add(5)).unwrap();
        m.update("c2", |mut c| c.add(3)).unwrap();

        assert_eq!(m.get(&"c1").unwrap().value(), 5);
        assert_eq!(m.get(&"c2").unwrap().value(), 3);
    }

    #[test]
    fn apply_counter_map_ops_independent() {
        let mut m1 = ObserveRemoveMap::new(|| PNCounter::new("h1"));
        let mut m2 = ObserveRemoveMap::new(|| PNCounter::new("h2"));

        let op1 = m1.update("c1", |mut c| c.add(5)).unwrap();
        let op2 = m2.update("c2", |mut c| c.add(3)).unwrap();

        m2.apply(&op1);
        m1.apply(&op2);

        assert_eq!(m1.get(&"c1").unwrap().value(), 5);
        assert_eq!(m1.get(&"c2").unwrap().value(), 3);

        assert_eq!(m2.get(&"c1").unwrap().value(), 5);
        assert_eq!(m2.get(&"c2").unwrap().value(), 3);
    }

    #[test]
    fn apply_counter_map_ops_dependent() {
        let mut m1 = ObserveRemoveMap::new(|| PNCounter::new("h1"));
        let mut m2 = ObserveRemoveMap::new(|| PNCounter::new("h2"));

        let op1 = m1.update("c1", |mut c| c.add(5)).unwrap();
        let op2 = m2.update("c1", |mut c| c.add(3)).unwrap();

        m2.apply(&op1);
        m1.apply(&op2);

        assert_eq!(m1.get(&"c1").unwrap().value(), 8);
        assert_eq!(m2.get(&"c1").unwrap().value(), 8);

        let op3 = m1.update("c1", |mut c| c.add(-4)).unwrap();

        m2.apply(&op3);

        assert_eq!(m1.get(&"c1").unwrap().value(), 4);
        assert_eq!(m2.get(&"c1").unwrap().value(), 4);
    }

    #[test]
    fn merge_counter_map_ops_dependent() {
        let mut m1 = ObserveRemoveMap::new(|| PNCounter::new("h1"));
        let mut m2 = ObserveRemoveMap::new(|| PNCounter::new("h2"));

        m1.update("c1", |mut c| c.add(5)).unwrap();
        m2.update("c1", |mut c| c.add(3)).unwrap();

        m2.merge(&m1);
        m1.merge(&m2);

        assert_eq!(m1.get(&"c1").unwrap().value(), 8);
        assert_eq!(m2.get(&"c1").unwrap().value(), 8);

        m1.update("c1", |mut c| c.add(-4)).unwrap();

        m2.merge(&m1);

        assert_eq!(m1.get(&"c1").unwrap().value(), 4);
        assert_eq!(m2.get(&"c1").unwrap().value(), 4);
    }
}

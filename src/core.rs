pub trait Operation<T> {
    fn apply(&self, &mut T);
}

pub trait StateRDT {
    fn merge(&mut self, other: &Self);
}

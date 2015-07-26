pub trait StateRDT {
    fn merge(&mut self, other: &Self);
}

pub trait OperationRDT {
    type Operation;

    fn apply(&mut self, op: &Self::Operation);
}

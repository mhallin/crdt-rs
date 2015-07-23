use std::ops::Add;

use core::Operation;

#[derive(Debug)]
pub struct OpCounter<T: Add<T, Output=T> + Copy> {
    counter: T,
}

#[derive(Debug)]
pub struct IncrementOperation<T: Add<T, Output=T> + Copy> {
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

impl<T: Add<T, Output=T> + Copy> Operation<OpCounter<T>> for IncrementOperation<T> {
    fn apply(&self, target: &mut OpCounter<T>) {
        target.counter = target.counter + self.value;
    }
}

#[cfg(test)]
mod test {
    use super::OpCounter;
    use core::Operation;

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
}

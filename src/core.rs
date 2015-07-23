pub trait Operation<T> {
    fn apply(&self, &mut T);
}

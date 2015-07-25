use std::default::Default;

use chrono::{NaiveDateTime, DateTime, UTC, TimeZone};

use core::{Operation, StateRDT};

#[derive(RustcEncodable, RustcDecodable)]
pub struct LWWRegister<T: Default + Clone> {
    value: T,
    timestamp: DateTime<UTC>,
}

#[derive(RustcEncodable, RustcDecodable)]
pub struct SetLWWRegisterOperation<T: Default + Clone> {
    value: T,
    timestamp: DateTime<UTC>,
}

impl<T: Default + Clone> LWWRegister<T> {
    pub fn new() -> LWWRegister<T> {
        LWWRegister {
            value: Default::default(),
            timestamp: UTC.from_utc_datetime(&NaiveDateTime::from_timestamp(0, 0)),
        }
    }

    pub fn value<'a>(&'a self) -> &'a T {
        &self.value
    }

    pub fn set(&mut self, value: T) -> SetLWWRegisterOperation<T> {
        let op = SetLWWRegisterOperation {
            value: value,
            timestamp: UTC::now(),
        };

        op.apply(self);

        op
    }
}

impl<T: Default + Clone> Operation<LWWRegister<T>> for SetLWWRegisterOperation<T> {
    fn apply(&self, target: &mut LWWRegister<T>) {
        if self.timestamp > target.timestamp {
            target.value = self.value.clone();
            target.timestamp = self.timestamp.clone();
        }
    }
}

impl<T: Default + Clone> StateRDT for LWWRegister<T> {
    fn merge(&mut self, other: &Self) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp.clone();
        }
    }
}

#[cfg(test)]
mod test {
    use super::LWWRegister;
    use core::{Operation, StateRDT};

    #[test]
    fn make_lww_register() {
        let register : LWWRegister<&'static str> = LWWRegister::new();

        assert_eq!(register.value(), &"");
    }

    #[test]
    fn set_lww_register() {
        let mut register = LWWRegister::new();

        register.set("test");

        assert_eq!(register.value(), &"test");
    }

    #[test]
    fn apply_lww_register_set() {
        let mut r1 = LWWRegister::new();
        let mut r2 = LWWRegister::new();

        let op1 = r1.set("first");
        let op2 = r2.set("last");

        op2.apply(&mut r1);
        op1.apply(&mut r2);

        assert_eq!(r1.value(), &"last");
        assert_eq!(r2.value(), &"last");
    }

    #[test]
    fn merge_lww_register_state() {
        let mut r1 = LWWRegister::new();
        let mut r2 = LWWRegister::new();

        r1.set("first");
        r2.set("last");

        r1.merge(&r2);
        r2.merge(&r1);

        assert_eq!(r1.value(), &"last");
        assert_eq!(r2.value(), &"last");
    }
}

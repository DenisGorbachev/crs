use crate::{Ks, Tx};
use fjall::UserKey;
use rkyv::Serialize;
use rkyv::rancor::Fallible;

pub fn insert_archived<S: Fallible + ?Sized, K: Into<UserKey>, V: Serialize<S>>(_tx: &mut Tx, _keyspace: &Ks, _key: K, _value: V) {
    todo!()
}

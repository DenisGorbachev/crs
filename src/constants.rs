use crate::{Db, Ks};
use fjall::KeyspaceCreateOptions;

pub fn reviews_keyspace(db: &Db) -> fjall::Result<Ks> {
    db.keyspace("reviews", KeyspaceCreateOptions::default)
}

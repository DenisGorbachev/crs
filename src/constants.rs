use crate::{Db, Ks};
use fjall::KeyspaceCreateOptions;

pub fn messages_keyspace(db: &Db) -> fjall::Result<Ks> {
    db.keyspace("messages", KeyspaceCreateOptions::default)
}

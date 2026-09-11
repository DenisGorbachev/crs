use crate::{Ks, Tx};
use errgonomic::handle;
use fjall::UserKey;
use rkyv::Serialize;
use rkyv::api::high::{HighSerializer, to_bytes_in};
use rkyv::rancor::Error as RkyvError;
use rkyv::ser::allocator::ArenaHandle;
use thiserror::Error;

pub fn save<K, V>(tx: &mut Tx<'_>, keyspace: &Ks, key: K, value: &V) -> Result<(), SaveError>
where
    K: Into<UserKey>,
    V: for<'a> Serialize<HighSerializer<Vec<u8>, ArenaHandle<'a>, RkyvError>>,
{
    use SaveError::*;
    let key = key.into();
    let bytes = handle!(to_bytes_in::<_, RkyvError>(value, Vec::new()), ToBytesInFailed, key);
    tx.insert(keyspace, key, bytes);
    Ok(())
}

#[derive(Error, Debug)]
pub enum SaveError {
    #[error("failed to serialize the archived value")]
    ToBytesInFailed { source: RkyvError, key: UserKey },
}

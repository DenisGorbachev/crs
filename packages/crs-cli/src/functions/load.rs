use crate::Ks;
use errgonomic::{handle, handle_opt};
use fjall::{Error as FjallError, Readable, UserKey, UserValue};
use rkyv::api::high::{HighDeserializer, HighValidator, from_bytes};
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::Error as RkyvError;
use rkyv::{Archive, Deserialize};
use thiserror::Error;

pub fn load<V>(tx: &(impl Readable + ?Sized), keyspace: &Ks, key: impl Into<UserKey>) -> Result<V, LoadError>
where
    V: Archive,
    V::Archived: for<'a> CheckBytes<HighValidator<'a, RkyvError>> + Deserialize<V, HighDeserializer<RkyvError>>,
{
    use LoadError::*;
    let key = key.into();
    let bytes = handle!(tx.get(keyspace, &key), GetFailed, key);
    let bytes = handle_opt!(bytes, ValueNotFound, key);
    let value = handle!(from_bytes::<V, RkyvError>(&bytes), FromBytesFailed, key, bytes);
    Ok(value)
}

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("failed to read the archived value")]
    GetFailed { source: FjallError, key: UserKey },
    #[error("archived value not found")]
    ValueNotFound { key: UserKey },
    #[error("failed to deserialize the archived value")]
    FromBytesFailed { source: RkyvError, key: UserKey, bytes: UserValue },
}

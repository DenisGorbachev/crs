use std::hash::{DefaultHasher, Hash, Hasher};

pub fn content_version(value: &impl Hash) -> String {
    let mut hash = DefaultHasher::new();
    value.hash(&mut hash);
    format!("{:016x}", hash.finish())
}

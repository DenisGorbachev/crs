use serde::Serialize;
use serde_json::{Error, Value, to_value};

pub fn canonical_json(value: &impl Serialize) -> Result<Value, Error> {
    to_value(value).map(|mut json| {
        json.sort_all_objects();
        json
    })
}

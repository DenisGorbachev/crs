use std::env;
use std::ffi::OsString;

pub fn editor_var_os(input: Option<OsString>) -> OsString {
    input
        .or_else(|| env::var_os("VISUAL"))
        .or_else(|| env::var_os("EDITOR"))
        .unwrap_or_else(|| "vi".into())
}

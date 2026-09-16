use std::io::Write;

pub fn iter_write_as_markdown<'a>(_writer: &mut impl Write, _iter: impl IntoIterator<Item = &'a str>) {
    todo!()
}

#[cfg(test)]
mod tests {
    #[test]
    fn must_iter_write_as_markdown() {
        let _parts = ["foo", "bar"];
        let _output = String::new();
        // iter_write_as_markdown(&mut output, parts);
    }
}

pub trait StringExt {
    fn humanise(&self) -> String;
}

impl<T> StringExt for T
where
    T: AsRef<str>,
{
    fn humanise(&self) -> String {
        // When on newer compiler:
        // self.as_ref().replace(['_', '.'], " ")
        let pattern: &[char] = &['_', '.'];
        self.as_ref().replace(pattern, " ")
    }
}

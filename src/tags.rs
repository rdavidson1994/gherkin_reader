use std::{iter::Map, str::Split};

#[derive(Debug, Clone)]
pub(crate) struct GherkinTags<'a>(&'a str);

impl<'a> GherkinTags<'a> {
    pub(crate) fn new(after_first_at_sign: &'a str) -> Self {
        GherkinTags(after_first_at_sign)
    }
}

impl<'a> IntoIterator for GherkinTags<'a> {
    type Item = &'a str;

    type IntoIter = GherkinTagsIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        GherkinTagsIterator(self.0.split('@').map(str::trim))
    }
}
pub(crate) struct GherkinTagsIterator<'a>(Map<Split<'a, char>, for<'r> fn(&'r str) -> &'r str>);
impl<'a> Iterator for GherkinTagsIterator<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

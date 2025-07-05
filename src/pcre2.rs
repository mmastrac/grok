use crate::Error;
use pcre2::bytes::{Captures, Regex, RegexBuilder};
use std::collections::{btree_map, BTreeMap, HashMap};

pub(crate) const ENGINE: crate::Engine = crate::Engine::Pcre2;

/// The `Pattern` represents a compiled regex, ready to be matched against arbitrary text.
#[derive(Debug)]
pub(crate) struct Pcre2Pattern {
    pub regex: Regex,
    pub names: BTreeMap<String, usize>,
}

impl Pcre2Pattern {
    /// Creates a new pattern from a raw regex string and an alias map to identify the
    /// fields properly.
    pub(crate) fn new(regex: &str, alias: &HashMap<String, String>) -> Result<Self, Error> {
        let mut builder = RegexBuilder::new();
        builder.jit_if_available(true);
        builder.utf(true);
        match builder.build(regex) {
            Ok(r) => Ok({
                let mut names = BTreeMap::new();
                for (i, name) in r.capture_names().iter().enumerate() {
                    if let Some(name) = name {
                        let name = alias.get(name).map_or(name, |s| s).to_string();
                        names.insert(name, i);
                    }
                }
                Self { regex: r, names }
            }),
            Err(e) => Err(Error::RegexCompilationFailed(format!(
                "Regex compilation failed: {e:?}:\n{regex}"
            ))),
        }
    }

    /// Matches this compiled `Pattern` against the text and returns the matches.
    pub fn match_against<'a>(&'a self, text: &'a str) -> Option<Pcre2Matches<'a>> {
        self.regex
            .captures(text.as_bytes())
            .ok()
            .flatten()
            .map(|caps| Pcre2Matches {
                captures: caps,
                pattern: self,
            })
    }

    /// Returns all names this `Pattern` captures.
    pub fn capture_names(&self) -> impl Iterator<Item = &str> {
        self.names.keys().map(|s| s.as_str())
    }
}

/// The `Matches` represent matched results from a `Pattern` against a provided text.
#[derive(Debug)]
pub(crate) struct Pcre2Matches<'a> {
    captures: Captures<'a>,
    pub pattern: &'a Pcre2Pattern,
}

impl<'a> Pcre2Matches<'a> {
    /// Gets the value for the name (or) alias if found, `None` otherwise.
    pub fn get(&self, name_or_alias: &str) -> Option<&str> {
        self.pattern
            .names
            .get(name_or_alias)
            .and_then(|&idx| self.captures.get(idx))
            .map(|m| std::str::from_utf8(m.as_bytes()).unwrap())
    }

    /// Returns a tuple of key/value with all the matches found.
    ///
    /// Note that if no match is found, the value is empty.
    pub fn iter(&'a self) -> Pcre2MatchesIter<'a> {
        Pcre2MatchesIter {
            captures: &self.captures,
            names: self.pattern.names.iter(),
        }
    }
}

impl<'a> IntoIterator for &'a Pcre2Matches<'a> {
    type Item = (&'a str, &'a str);
    type IntoIter = Pcre2MatchesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An `Iterator` over all matches, accessible via `Matches`.
pub(crate) struct Pcre2MatchesIter<'a> {
    captures: &'a Captures<'a>,
    names: btree_map::Iter<'a, String, usize>,
}

impl<'a> Iterator for Pcre2MatchesIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        for (k, &v) in self.names.by_ref() {
            if let Some(m) = self.captures.get(v) {
                return Some((k.as_str(), std::str::from_utf8(m.as_bytes()).unwrap()));
            }
        }
        None
    }
}

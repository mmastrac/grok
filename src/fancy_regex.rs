use crate::Error;
use fancy_regex::{Captures, Regex};
use std::collections::{btree_map, BTreeMap, HashMap};

/// The `Pattern` represents a compiled regex, ready to be matched against arbitrary text.
#[derive(Debug)]
pub(crate) struct FancyRegexPattern {
    regex: Regex,
    pub names: BTreeMap<String, usize>,
}

impl FancyRegexPattern {
    /// Creates a new pattern from a raw regex string and an alias map to identify the
    /// fields properly.
    pub(crate) fn new(regex: &str, alias: &HashMap<String, String>) -> Result<Self, Error> {
        match Regex::new(regex) {
            Ok(r) => Ok({
                let mut names = BTreeMap::new();
                for (i, name) in r.capture_names().enumerate() {
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
    pub fn match_against<'a>(&'a self, text: &'a str) -> Option<FancyRegexMatches<'a>> {
        self.regex.captures(text).ok().flatten().and_then(|caps| {
            Some(FancyRegexMatches {
                captures: caps,
                pattern: self,
            })
        })
    }

    /// Returns all names this `Pattern` captures.
    pub fn capture_names(&self) -> impl Iterator<Item = &str> {
        self.names.keys().map(|s| s.as_str())
    }
}

/// The `Matches` represent matched results from a `Pattern` against a provided text.
#[derive(Debug)]
pub(crate) struct FancyRegexMatches<'a> {
    captures: Captures<'a>,
    pub pattern: &'a FancyRegexPattern,
}

impl<'a> FancyRegexMatches<'a> {
    /// Gets the value for the name (or) alias if found, `None` otherwise.
    pub fn get(&self, name_or_alias: &str) -> Option<&str> {
        self.pattern
            .names
            .get(name_or_alias)
            .and_then(|&idx| self.captures.get(idx))
            .map(|m| m.as_str())
    }

    /// Returns a tuple of key/value with all the matches found.
    pub fn iter(&'a self) -> FancyRegexMatchesIter<'a> {
        FancyRegexMatchesIter {
            captures: &self.captures,
            names: self.pattern.names.iter(),
        }
    }
}

impl<'a> IntoIterator for &'a FancyRegexMatches<'a> {
    type Item = (&'a str, &'a str);
    type IntoIter = FancyRegexMatchesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An `Iterator` over all matches, accessible via `Matches`.
pub(crate) struct FancyRegexMatchesIter<'a> {
    captures: &'a Captures<'a>,
    names: btree_map::Iter<'a, String, usize>,
}

impl<'a> Iterator for FancyRegexMatchesIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        for (k, &v) in self.names.by_ref() {
            if let Some(m) = self.captures.get(v) {
                return Some((k.as_str(), m.as_str()));
            }
        }
        None
    }
}

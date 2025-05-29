use crate::Error;
use regex::{Captures, Regex};
use std::collections::{btree_map, BTreeMap, HashMap};

/// The `Pattern` represents a compiled regex, ready to be matched against arbitrary text.
#[derive(Debug)]
pub struct RegexPattern {
    regex: Regex,
    pub(crate) names: BTreeMap<String, usize>,
}

impl RegexPattern {
    /// Creates a new pattern from a raw regex string and an alias map to identify the
    /// fields properly.
    pub(crate) fn new(regex: &str, alias: &HashMap<String, String>) -> Result<Self, Error> {
        match Regex::new(regex) {
            Ok(r) => Ok({
                let mut names = BTreeMap::new();
                for (i, name) in r.capture_names().enumerate() {
                    if let Some(name) = name {
                        let name = match alias.iter().find(|&(_k, v)| *v == name) {
                            Some(item) => item.0.clone(),
                            None => String::from(name),
                        };
                        names.insert(name, i);
                    }
                }
                Self { regex: r, names }
            }),
            Err(_) => Err(Error::RegexCompilationFailed(regex.into())),
        }
    }

    /// Matches this compiled `Pattern` against the text and returns the matches.
    pub fn match_against<'a>(&'a self, text: &'a str) -> Option<RegexMatches<'a>> {
        self.regex.captures(text).map(|caps| RegexMatches {
            text,
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
pub struct RegexMatches<'a> {
    text: &'a str,
    captures: Captures<'a>,
    pattern: &'a RegexPattern,
}

impl<'a> RegexMatches<'a> {
    /// Instantiates the matches for a pattern after the match.
    pub(crate) fn new(text: &'a str, captures: Captures<'a>, pattern: &'a RegexPattern) -> Self {
        RegexMatches {
            text,
            captures,
            pattern,
        }
    }

    /// Gets the value for the name (or) alias if found, `None` otherwise.
    pub fn get(&self, name_or_alias: &str) -> Option<&str> {
        self.pattern
            .names
            .get(name_or_alias)
            .and_then(|&idx| self.captures.get(idx))
            .map(|m| m.as_str())
    }

    /// Returns the number of matches.
    pub fn len(&self) -> usize {
        self.captures.len() - 1
    }

    /// Returns true if there are no matches, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a tuple of key/value with all the matches found.
    ///
    /// Note that if no match is found, the value is empty.
    pub fn iter(&'a self) -> RegexMatchesIter<'a> {
        RegexMatchesIter {
            text: &self.text,
            captures: &self.captures,
            names: self.pattern.names.iter(),
        }
    }
}

impl<'a> IntoIterator for &'a RegexMatches<'a> {
    type Item = (&'a str, &'a str);
    type IntoIter = RegexMatchesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An `Iterator` over all matches, accessible via `Matches`.
pub struct RegexMatchesIter<'a> {
    text: &'a str,
    captures: &'a Captures<'a>,
    names: btree_map::Iter<'a, String, usize>,
}

impl<'a> Iterator for RegexMatchesIter<'a> {
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

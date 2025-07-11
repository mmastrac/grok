use crate::Error;
use onig::{MatchParam, Regex, Region, SearchOptions};
use std::collections::{btree_map, BTreeMap, HashMap};

pub(crate) const ENGINE: crate::Engine = crate::Engine::Onig;

/// The `Pattern` represents a compiled regex, ready to be matched against arbitrary text.
#[derive(Debug)]
pub(crate) struct OnigPattern {
    pub regex: Regex,
    pub names: BTreeMap<String, u32>,
}

impl OnigPattern {
    /// Creates a new pattern from a raw regex string and an alias map to identify the
    /// fields properly.
    pub(crate) fn new(regex: &str, alias: &HashMap<String, String>) -> Result<Self, Error> {
        match Regex::new(regex) {
            Ok(r) => Ok({
                let mut names = BTreeMap::new();
                r.foreach_name(|cap_name, cap_idx| {
                    let name = alias.get(cap_name).map_or(cap_name, |s| s).to_string();
                    match names.entry(name) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(cap_idx[0]);
                        }
                        btree_map::Entry::Occupied(mut e) => {
                            if cap_idx[0] > *e.get() {
                                e.insert(cap_idx[0]);
                            }
                        }
                    }
                    true
                });
                Self { regex: r, names }
            }),
            Err(e) => Err(Error::RegexCompilationFailed(format!(
                "Regex compilation failed: {e:?}:\n{regex}"
            ))),
        }
    }

    /// Matches this compiled `Pattern` against the text and returns the matches.
    pub fn match_against<'a>(&'a self, text: &'a str) -> Option<OnigMatches<'a>> {
        // Inlined version of the onig methods that cause an internal panic
        let this = &self.regex;
        let mut region = Region::new();
        let to = text.len();
        let options = SearchOptions::SEARCH_OPTION_NONE;
        let match_param = MatchParam::default();
        let result = this.search_with_param(text, 0, to, options, Some(&mut region), match_param);

        result.unwrap_or_default().map(|_| OnigMatches {
            text,
            region,
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
pub(crate) struct OnigMatches<'a> {
    text: &'a str,
    region: Region,
    pub pattern: &'a crate::onig::OnigPattern,
}

impl<'a> OnigMatches<'a> {
    /// Gets the value for the name (or) alias if found, `None` otherwise.
    pub fn get(&self, name_or_alias: &str) -> Option<&str> {
        match self.pattern.names.get(name_or_alias) {
            Some(found) => self
                .region
                .pos(*found as usize)
                .map(|(start, end)| &self.text[start..end]),
            None => None,
        }
    }

    /// Returns a tuple of key/value with all the matches found.
    ///
    /// Note that if no match is found, the value is empty.
    pub fn iter(&'a self) -> OnigMatchesIter<'a> {
        OnigMatchesIter {
            text: self.text,
            region: &self.region,
            names: self.pattern.names.iter(),
        }
    }
}

impl<'a> IntoIterator for &'a OnigMatches<'a> {
    type Item = (&'a str, &'a str);
    type IntoIter = OnigMatchesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An `Iterator` over all matches, accessible via `Matches`.
pub(crate) struct OnigMatchesIter<'a> {
    text: &'a str,
    region: &'a Region,
    names: btree_map::Iter<'a, String, u32>,
}

impl<'a> Iterator for OnigMatchesIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        for (k, v) in self.names.by_ref() {
            match self.region.pos(*v as usize) {
                Some((start, end)) => return Some((k.as_str(), &self.text[start..end])),
                None => {
                    continue;
                }
            }
        }
        None
    }
}

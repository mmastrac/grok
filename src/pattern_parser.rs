use std::{iter::Peekable, ops::Range, str::CharIndices};

/// An error in the grok pattern.
#[derive(Debug)]
pub enum GrokPatternError {
    /// The pattern could not be parsed successfully.
    InvalidCharacter(char),
    /// The pattern is invalid.
    InvalidPattern,
    /// The pattern definition is invalid.
    InvalidPatternDefinition,
}

/// One of the components of a grok pattern: a regular expression, a pattern or
/// an error.
pub enum GrokComponent<'a> {
    /// This chunk is a regular expression.
    RegularExpression {
        /// The span of the original string.
        range: Range<usize>,
        /// The text chunk of the original string.
        string: &'a str,
    },
    /// This chunk is a grok pattern placeholder.
    GrokPattern {
        /// The span of the original string.
        range: Range<usize>,
        /// The text chunk of the original string.
        pattern: &'a str,
        /// The name part of the pattern.
        name: &'a str,
        /// The alias part of the pattern.
        alias: &'a str,
        /// The extract part of the pattern.
        extract: &'a str,
        /// The definition part of the pattern.
        definition: &'a str,
    },
    /// The pattern could not be parsed successfully.
    PatternError(GrokPatternError),
}

impl std::fmt::Debug for GrokComponent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrokComponent::RegularExpression{ string, .. } => write!(f, "{string:?}"),
            GrokComponent::GrokPattern{ name, alias, extract: capture, definition, .. } => write!(f, "%{{ name={name:?} alias={alias:?} capture={capture:?} definition={definition:?} }}"),
            GrokComponent::PatternError(e) => write!(f, "<error {e:?}>"),
        }
    }
}

impl std::fmt::Display for GrokComponent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrokComponent::RegularExpression { string, .. } => f.write_str(string),
            GrokComponent::GrokPattern { pattern, .. } => f.write_str(pattern),
            GrokComponent::PatternError(e) => write!(f, "<error {e:?}>"),
        }
    }
}

/// An iterator over the components of a grok pattern.
pub struct GrokSplit<'a> {
    string: &'a str,
    string_iter: Peekable<CharIndices<'a>>,
}

impl<'a> Iterator for GrokSplit<'a> {
    type Item = GrokComponent<'a>;

    /// Hand-rolled state machine
    fn next(&mut self) -> Option<Self::Item> {
        match self.try_next() {
            res @ Some(GrokComponent::PatternError(_)) => {
                // Fuse the iterator if we error out
                self.string_iter = "".char_indices().peekable();
                res
            }
            res => res,
        }
    }
}

impl<'a> GrokSplit<'a> {
    fn try_next(&mut self) -> Option<GrokComponent<'a>> {
        let (start, next) = self.string_iter.next()?;

        if next == '%' {
            // End of string, that's fine (skip updating index, we're done)
            let Some((_, next)) = self.string_iter.next() else {
                let range = start..self.string.len();
                return Some(GrokComponent::RegularExpression {
                    string: &self.string[range.clone()],
                    range,
                });
            };

            if next == '{' {
                let mut components: [&'a str; 3] = ["", "", ""];
                let mut comp_index = 0;

                // Load up to three components for PATTERN:field:type, erroring out if
                // more are present.
                loop {
                    match self.try_munch_word(comp_index > 0) {
                        Ok((terminator, word)) => {
                            if comp_index == 3 {
                                return Some(GrokComponent::PatternError(
                                    GrokPatternError::InvalidPattern,
                                ));
                            }

                            components[comp_index] = word;
                            let next = self.string_iter.next().unwrap();
                            comp_index += 1;

                            if comp_index == 3 && components[2].is_empty() {
                                return Some(GrokComponent::PatternError(
                                    GrokPatternError::InvalidPatternDefinition,
                                ));
                            }

                            if terminator == '}' {
                                let index = next.0 + 1;

                                if comp_index == 2 && components[1].is_empty() {
                                    return Some(GrokComponent::PatternError(
                                        GrokPatternError::InvalidPatternDefinition,
                                    ));
                                }

                                return Some(GrokComponent::GrokPattern {
                                    range: start..index,
                                    pattern: &self.string[start..index],
                                    name: components[0],
                                    alias: components[1],
                                    extract: components[2],
                                    definition: "",
                                });
                            } else if terminator == '=' {
                                let definition_start = next.0 + 1;
                                let index = loop {
                                    let Some((index, next)) = self.string_iter.next() else {
                                        return Some(GrokComponent::PatternError(
                                            GrokPatternError::InvalidPatternDefinition,
                                        ));
                                    };
                                    if next == '{' {
                                        return Some(GrokComponent::PatternError(
                                            GrokPatternError::InvalidPatternDefinition,
                                        ));
                                    }
                                    if next == '}' {
                                        break index;
                                    }
                                };

                                let definition = &self.string[definition_start..index];
                                if definition.is_empty() {
                                    return Some(GrokComponent::PatternError(
                                        GrokPatternError::InvalidPatternDefinition,
                                    ));
                                }
                                if comp_index == 2 && components[1].is_empty() {
                                    return Some(GrokComponent::PatternError(
                                        GrokPatternError::InvalidPatternDefinition,
                                    ));
                                }

                                return Some(GrokComponent::GrokPattern {
                                    range: start..index + 1,
                                    pattern: &self.string[start..index + 1],
                                    name: components[0],
                                    alias: components[1],
                                    extract: components[2],
                                    definition,
                                });
                            }
                        }
                        Err(e) => return Some(GrokComponent::PatternError(e)),
                    };
                }
            }
        }

        // Not a pattern, munch until end-of-string or a %
        while let Some((index, next)) = self.string_iter.peek() {
            if *next == '%' {
                let range = start..*index;
                return Some(GrokComponent::RegularExpression {
                    string: &self.string[range.clone()],
                    range,
                });
            }
            _ = self.string_iter.next();
        }

        let range = start..self.string.len();
        Some(GrokComponent::RegularExpression {
            string: &self.string[range.clone()],
            range,
        })
    }

    /// Attempt to munch a word at the current index, Returns the terminator
    /// character and the word.
    fn try_munch_word(
        &mut self,
        is_alias_or_capture: bool,
    ) -> Result<(char, &'a str), GrokPatternError> {
        let terminator;

        let Some((start, _)) = self.string_iter.peek() else {
            return Err(GrokPatternError::InvalidPattern);
        };
        let start = *start;
        let mut end;

        loop {
            if let Some((index, next)) = self.string_iter.peek() {
                end = *index;
                if *next == '}' || *next == '=' || *next == ':' {
                    terminator = *next;
                    break;
                }
                // is_alias or is_capture allows for extra chars: `-[].`
                if !next.is_ascii_alphanumeric()
                    && *next != '_'
                    && (!is_alias_or_capture || !"-[].".contains(*next))
                {
                    return Err(GrokPatternError::InvalidCharacter(*next));
                }
                _ = self.string_iter.next();
            } else {
                return Err(GrokPatternError::InvalidPattern);
            }
        }

        if end == start && !is_alias_or_capture {
            Err(GrokPatternError::InvalidPattern)
        } else {
            Ok((terminator, &self.string[start..end]))
        }
    }
}

/// Splits a grok pattern into its components.
///
/// A grok pattern is a regular expression string with grok pattern placeholders
/// embedded in it.
///
/// The grok pattern placeholders are of the form
/// `%{name:alias:extract=definition}`, where `name` is the name of the pattern,
/// `alias` is the alias of the pattern, `extract` is the extract of the
/// pattern, and `definition` is the definition of the pattern.
///
/// - `name` is the name of the pattern and is required. It may contain any
///   alphanumeric character, or `_`.
/// - `alias` is the alias of the pattern and is optional. It may contain any
///   alphanumeric character, or any of `_-[].`. If extract is provided,
///   `alias` may be empty.
/// - `extract` is the extract of the pattern and is optional. It may contain
///   any alphanumeric character, or any of `_-[].`.
/// - `definition` is the definition of the pattern and is optional. It may
///   contain any character other than `{` or `}`.
///
/// A literal `%` character may appear in a grok pattern as long as it is not
/// followed by `{`. You can surround the percent with grouped parentheses
/// `(%){..}`, a non-capturing group `(?:%){..}`, or use the `\x25` escape
/// sequence, ie: `\x25{..}`.
pub fn grok_split<'a, S: AsRef<str> + ?Sized>(string: &'a S) -> GrokSplit<'a> {
    let string = string.as_ref();
    GrokSplit {
        string,
        string_iter: string.char_indices().peekable(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grok_split() {
        let pattern = "Hello, %{name}!";
        let components = grok_split(pattern);

        assert_eq!(
            components.map(|c| format!("{c}")).collect::<Vec<_>>(),
            vec!["Hello, ", "%{name}", "!"]
        );
    }

    #[test]
    fn legal_grok_patterns() {
        for pattern in &[
            "%{name}",
            "%{name:name}",
            "%{name:name:name}",
            "%{name::name}",
            "%{name=defn}",
            "%{name:name=defn}",
            "%{name:name:name=defn}",
            "%{name:name[x]}",
            "%{name:name[x]:name[y]}",
        ] {
            eprintln!("{pattern} -> {:?}", grok_split(pattern).collect::<Vec<_>>());
            assert!(!grok_split(pattern).any(|c| matches!(c, GrokComponent::PatternError(_))));
            let result = grok_split(pattern).next().unwrap();
            eprintln!("{result:?}");

            let components = grok_split(pattern);
            for c in components {
                match c {
                    GrokComponent::RegularExpression { string, range, .. } => {
                        assert_eq!(&pattern[range], string);
                    }
                    GrokComponent::GrokPattern {
                        pattern: pattern_str,
                        range,
                        ..
                    } => {
                        assert_eq!(&pattern[range], pattern_str);
                    }
                    _ => unreachable!(),
                }
            }

            // TODO: test parse results
        }
    }

    #[test]
    fn real_grok_patterns() {
        for pattern in &[
            r"(?:\(Views: %{NUMBER:viewms}ms \| ActiveRecord: %{NUMBER:activerecordms}ms|\(ActiveRecord: %{NUMBER:activerecordms}ms)?",
            r"%{NUMBER:ts}\t%{NOTSPACE:uid}\t%{IP:orig_h}\t%{INT:orig_p}\t%{IP:resp_h}\t%{INT:resp_p}\t%{WORD:proto}\t%{INT:trans_id}\t%{GREEDYDATA:query}\t%{GREEDYDATA:qclass}\t%{GREEDYDATA:qclass_name}\t%{GREEDYDATA:qtype}\t%{GREEDYDATA:qtype_name}\t%{GREEDYDATA:rcode}\t%{GREEDYDATA:rcode_name}\t%{GREEDYDATA:AA}\t%{GREEDYDATA:TC}\t%{GREEDYDATA:RD}\t%{GREEDYDATA:RA}\t%{GREEDYDATA:Z}\t%{GREEDYDATA:answers}\t%{GREEDYDATA:TTLs}\t%{GREEDYDATA:rejected}",
        ] {
            eprintln!("{pattern} -> {:?}", grok_split(pattern).collect::<Vec<_>>());
            assert!(!grok_split(pattern).any(|c| matches!(c, GrokComponent::PatternError(_))));
            let result = grok_split(pattern).next().unwrap();
            eprintln!("{result:?}");

            // TODO: test parse results
        }
    }

    #[test]
    fn illegal_grok_patterns() {
        for pattern in &[
            "%{name",
            "%{name=",
            "%{name=}",
            "%{name=a",
            "%{name:",
            "%{name:}", // capture must be provided if alias is empty
            "%{name:a",
            "%{name:a:b",
            "%{name::",
            "%{name::b",
            "%{name:a:}",
            "%{name::}",
            "%{na.me:a:b}",
            "%{name:a:b:c}",
            "%{name:a:b:c:d}",
        ] {
            eprintln!("{pattern} -> {:?}", grok_split(pattern).collect::<Vec<_>>());

            assert!(
                grok_split(pattern).any(|c| matches!(c, GrokComponent::PatternError(_))),
                "{pattern} should have failed"
            );
        }
    }
}

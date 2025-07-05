use std::{iter::Peekable, ops::Range, str::CharIndices};

#[derive(Debug)]
pub enum GrokPatternError {
    /// The pattern could not be parsed successfully.
    InvalidCharacter(#[allow(unused)] char),
    /// The pattern is invalid.
    InvalidPattern,
    /// The pattern definition is invalid.
    InvalidPatternDefinition,
}

pub enum GrokComponent<'a> {
    /// This chunk is a regular expression.
    RegularExpression {
        range: Range<usize>,
        string: &'a str,
    },
    /// This chunk is a grok pattern placeholder.
    GrokPattern {
        range: Range<usize>,
        pattern: &'a str,
        name: &'a str,
        alias: &'a str,
        capture: &'a str,
        definition: &'a str,
    },
    /// The pattern could not be parsed successfully.
    PatternError(GrokPatternError),
}

impl std::fmt::Debug for GrokComponent<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrokComponent::RegularExpression{ string, .. } => write!(f, "{string:?}"),
            GrokComponent::GrokPattern{ name, alias, capture, definition, .. } => write!(f, "%{{ name={name:?} alias={alias:?} capture={capture:?} definition={definition:?} }}"),
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
                    match self.try_munch_word() {
                        Ok((terminator, word)) => {
                            if comp_index == 3 {
                                return Some(GrokComponent::PatternError(
                                    GrokPatternError::InvalidPattern,
                                ));
                            }

                            components[comp_index] = word;
                            let next = self.string_iter.next().unwrap();
                            comp_index += 1;

                            if terminator == '}' {
                                let index = next.0 + 1;
                                return Some(GrokComponent::GrokPattern {
                                    range: start..index,
                                    pattern: &self.string[start..index],
                                    name: components[0],
                                    alias: components[1],
                                    capture: components[2],
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

                                return Some(GrokComponent::GrokPattern {
                                    range: start..index,
                                    pattern: &self.string[start..index + 1],
                                    name: components[0],
                                    alias: components[1],
                                    capture: components[2],
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
    fn try_munch_word(&mut self) -> Result<(char, &'a str), GrokPatternError> {
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
                if !next.is_ascii_alphanumeric() && !"_-[]".contains(*next) {
                    return Err(GrokPatternError::InvalidCharacter(*next));
                }
                _ = self.string_iter.next();
            } else {
                return Err(GrokPatternError::InvalidPattern);
            }
        }

        if end == start {
            Err(GrokPatternError::InvalidPattern)
        } else {
            Ok((terminator, &self.string[start..end]))
        }
    }
}

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
            "%{name=defn}",
            "%{name:name=defn}",
            "%{name:name:name=defn}",
        ] {
            eprintln!("{pattern} -> {:?}", grok_split(pattern).collect::<Vec<_>>());
            assert!(!grok_split(pattern).any(|c| matches!(c, GrokComponent::PatternError(_))));
            let result = grok_split(pattern).next().unwrap();
            eprintln!("{result:?}");
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
            "%{name:}",
            "%{name:a",
            "%{name:a:b",
            "%{name::",
            "%{name::b",
            "%{name::b}",
            "%{name:a:}",
            "%{name::}",
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

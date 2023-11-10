use anyhow::{bail, Result};

pub enum RegexClass {
    Char(char),
    AlphaNum,
    Digit,
    CharGroup((Vec<char>, bool)),
    Sequence(Vec<RegexClass>),
}

impl RegexClass {
    fn min_size(&self) -> usize {
        match self {
            RegexClass::Sequence(seq) => {
                seq.iter().fold(0, |acc, item| { acc + item.min_size() } )
            }
            _ => 1,
        }
    }

    fn matches(&self, haystack: &str) -> (bool, usize) {
        let mut it = haystack.chars();
        match self {
            RegexClass::Char(pat) => {
                (it.next().is_some_and(|c| c == *pat), 1)
            }
            RegexClass::Digit => {
                let mt = it.next().is_some_and(|c| match c {
                    '0'..='9' => true,
                    _ => false
                });
                (mt, 1)
            }
            RegexClass::AlphaNum => {
                let mt = it.next().is_some_and(|c| match c {
                    '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => true,
                    _ => false
                });
                (mt, 1)
            }
            RegexClass::CharGroup((set, polarity)) => {
                let mt = it.next().is_some_and(|c| if set.contains(&c) {
                    *polarity
                } else {
                    !*polarity
                });
                (mt, 1)
            }
            RegexClass::Sequence(seq) => {
                let mut consumed = 0usize;

                for pat in seq {
                    let (matches, length) = pat.matches(&haystack[consumed..]);
                    if !matches {
                        return (false, 0)
                    } else {
                        consumed += length;
                    }
                }

                (true, consumed)
            }
        }
    }
}

pub struct RegexPattern {
    at_start: bool,
    sequence: RegexClass,
}

impl RegexPattern {
    pub fn parse(pattern: &str) -> Result<Self> {
        let mut at_start = false;
        let mut seq = vec![];
        let mut stream = pattern.chars();

        if pattern.starts_with('^') {
            stream.next();
            at_start = true;
        }

        while let Some(chr) = stream.next() {
            match chr {
                '\\' => {
                    match stream.next() {
                        Some('d') => seq.push(RegexClass::Digit),
                        Some('w') => seq.push(RegexClass::AlphaNum),
                        Some(chr) => seq.push(RegexClass::Char(chr)),
                        None => bail!("trailing backlash (\\)"),
                    }
                }
                '[' => {
                    let mut set = vec![];
                    let mut pos: usize = 0;
                    let mut polarity = true;
                    let mut closed = false;
                    while let Some(chr) = stream.next() {
                        match chr {
                            ']' => {
                                seq.push(RegexClass::CharGroup((set, polarity)));
                                closed = true;
                                break;
                            },
                            '^' => if pos == 0 { polarity = false } else { set.push('^') }
                            _ => if !set.contains(&chr) { set.push(chr) }
                        }
                        pos = pos + 1;
                    }
                    if !closed {
                        bail!("brackets ([ ]) not balanced")
                    }
                }
                _ => {
                    seq.push(RegexClass::Char(chr))
                }
            }
        };

        Ok(RegexPattern {
            at_start,
            sequence: RegexClass::Sequence(seq)
        })
    }

    pub fn is_contained_in(&self, haystack: &str) -> Result<bool> {
        let min_size = self.sequence.min_size();
        if min_size > haystack.len() {
            return Ok(false)
        }

        match &self.sequence {
            RegexClass::Sequence(..) => {
                if self.at_start {
                    return Ok(self.sequence.matches(haystack).0)
                }

                for offset in 0..=(haystack.len() - min_size) {
                    if self.sequence.matches(&haystack[(offset)..]).0 {
                        return Ok(true)
                    }
                }
                Ok(false)
            }
            _ => {
                bail!("Top pattern must be a sequence")
            }
        }
    }
}
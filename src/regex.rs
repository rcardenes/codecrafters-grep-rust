use anyhow::{bail, Result};

pub enum RegexPattern {
    Char(char),
    AlphaNum,
    Digit,
    CharGroup(Vec<char>),
    Empty,
}

impl RegexPattern {
    pub fn parse(pattern: &str) -> Result<Self> {
        let mut stream = pattern.chars();
        let res = match stream.next() {
            Some('\\') => {
                match stream.next() {
                    Some('d') => Ok(RegexPattern::Digit),
                    Some('w') => Ok(RegexPattern::AlphaNum),
                    Some(chr) => Ok(RegexPattern::Char(chr)),
                    None => bail!("trailing backlash (\\)"),
                }
            }
            Some('[') => {
                let mut set = vec![];
                while let Some(chr) = stream.next() {
                    match chr {
                        ']' => return Ok(RegexPattern::CharGroup(set)),
                        _ => if !set.contains(&chr) { set.push(chr) }
                    }
                }
                bail!("brackets ([ ]) not balanced")
            }
            Some(chr) => {
                Ok(RegexPattern::Char(chr))
            }
            None => {
                Ok(RegexPattern::Empty)
            }
        };

        if stream.next().is_none() {
            res
        } else {
            bail!("Unhandled pattern: {pattern}")
        }
    }

    pub fn is_contained_in(&self, haystack: &str) -> bool {
        match self {
            RegexPattern::Char(pat) => {
                haystack.contains(*pat)
            }
            RegexPattern::Digit => {
                let mut chars = haystack.chars();

                while let Some(chr) = chars.next() {
                    match chr {
                        '0'..='9' => return true,
                        _ => {}
                    }
                }
                false
            }
            RegexPattern::AlphaNum => {
                let mut chars = haystack.chars();

                while let Some(chr) = chars.next() {
                    match chr {
                        '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => return true,
                        _ => {}
                    }
                }
                false
            }
            RegexPattern::CharGroup(set) => {
                let mut chars = haystack.chars();

                while let Some(chr) = chars.next() {
                    if set.contains(&chr) {
                        return true
                    }
                }
                false
            }
            RegexPattern::Empty => true
        }
    }
}
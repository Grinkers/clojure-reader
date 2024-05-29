#![allow(clippy::inline_always)]

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::primitive::str;

use crate::edn::Edn;
use crate::error::{Code, Error};

const DELIMITERS: [char; 8] = [',', ']', '}', ')', ';', '(', '[', '{'];

#[derive(Debug)]
struct Walker {
    ptr: usize,
    column: usize,
    line: usize,
}

impl Walker {
    // Slurps until whitespace or delimiter, returning the slice.
    #[inline(always)]
    fn slurp_literal<'w>(&mut self, slice: &'w str) -> &'w str {
        let token = slice[self.ptr..]
            .split(|c: char| c.is_whitespace() || DELIMITERS.contains(&c))
            .next()
            .unwrap(); // At least an empty slice will always be on the first split, even on an empty str

        self.ptr += token.len();
        self.column += token.len();
        token
    }

    // Slurps a char. Special handling for chars that happen to be delimiters
    #[inline(always)]
    fn slurp_char<'a>(&mut self, slice: &'a str) -> &'a str {
        let starting_ptr = self.ptr;

        let mut ptr = 0;
        while let Some(c) = self.peek_next(slice) {
            // first is always \\, second is always a char we want.
            // Handles edge cases of having a valid "\\[" but also "\\c[lolthisisvalidedn"
            if ptr > 1 && (c.is_whitespace() || DELIMITERS.contains(&c)) {
                break;
            }

            let _ = self.nibble_next(slice);
            ptr += c.len_utf8();
        }
        &slice[starting_ptr..starting_ptr + ptr]
    }

    #[inline(always)]
    fn slurp_str<'w>(&mut self, slice: &'w str) -> Result<Edn<'w>, Error> {
        let _ = self.nibble_next(slice); // Consume the leading '"' char
        let starting_ptr = self.ptr;
        let mut escape = false;
        loop {
            if let Some(c) = self.nibble_next(slice) {
                if escape {
                    match c {
                        't' | 'r' | 'n' | '\\' | '\"' => (),
                        _ => {
                            return Err(Error {
                                code: Code::InvalidEscape,
                                column: Some(self.column),
                                line: Some(self.line),
                                ptr: Some(self.ptr),
                            })
                        }
                    }
                    escape = false;
                } else if c == '\"' {
                    return Ok(Edn::Str(&slice[starting_ptr..self.ptr - 1]));
                } else {
                    escape = c == '\\';
                }
            } else {
                return Err(Error {
                    code: Code::UnexpectedEOF,
                    column: Some(self.column),
                    line: Some(self.line),
                    ptr: Some(self.ptr),
                });
            }
        }
    }

    // Nibbles away until the next new line
    #[inline(always)]
    fn nibble_newline(&mut self, slice: &str) {
        let len = slice[self.ptr..].split('\n').next().unwrap(); // At least an empty slice will always be on the first split, even on an empty str
        self.ptr += len.len();
        self.nibble_whitespace(slice);
    }

    // Nibbles away until the start of the next form
    #[inline(always)]
    fn nibble_whitespace(&mut self, slice: &str) {
        while let Some(n) = self.peek_next(slice) {
            if n == ',' || n.is_whitespace() {
                let _ = self.nibble_next(slice);
                continue;
            }
            break;
        }
    }

    // Consumes next
    #[inline(always)]
    fn nibble_next<'w>(&'w mut self, slice: &'w str) -> Option<char> {
        let char = slice[self.ptr..].chars().next();
        if let Some(c) = char {
            self.ptr += c.len_utf8();
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        char
    }

    // Peek into the next char
    #[inline(always)]
    fn peek_next(&mut self, slice: &str) -> Option<char> {
        slice[self.ptr..].chars().next()
    }
}

pub fn parse(edn: &str) -> Result<Edn<'_>, Error> {
    let mut walker = Walker {
        ptr: 0,
        column: 1,
        line: 1,
    };

    let internal_parse = parse_internal(&mut walker, edn)?;
    internal_parse.map_or_else(|| Ok(Edn::Nil), Ok)
}

#[inline]
fn parse_internal<'e>(walker: &mut Walker, slice: &'e str) -> Result<Option<Edn<'e>>, Error> {
    walker.nibble_whitespace(slice);
    while let Some(next) = walker.peek_next(slice) {
        let column_start = walker.column;
        let ptr_start = walker.ptr;
        let line_start = walker.line;
        if let Some(ret) = match next {
            '\\' => match parse_char(walker.slurp_char(slice)) {
                Ok(edn) => Some(Ok(edn)),
                Err(code) => {
                    return Err(Error {
                        code,
                        line: Some(walker.line),
                        column: Some(column_start),
                        ptr: Some(walker.ptr),
                    })
                }
            },
            '\"' => Some(walker.slurp_str(slice)),
            // comment. consume until a new line.
            ';' => {
                walker.nibble_newline(slice);
                None
            }
            '[' => return Ok(Some(parse_vector(walker, slice, ']')?)),
            '(' => return Ok(Some(parse_vector(walker, slice, ')')?)),
            '{' => return Ok(Some(parse_map(walker, slice)?)),
            '#' => parse_tag_set_discard(walker, slice)?.map(Ok),
            // non-string literal case
            _ => match edn_literal(walker.slurp_literal(slice)) {
                Ok(edn) => match edn {
                    Some(e) => Some(Ok(e)),
                    None => {
                        return Ok(None);
                    }
                },
                Err(code) => {
                    return Err(Error {
                        code,
                        line: Some(line_start),
                        column: Some(column_start),
                        ptr: Some(ptr_start),
                    })
                }
            },
        } {
            return Ok(Some(ret?));
        }
    }
    Ok(None)
}

#[inline]
fn parse_tag_set_discard<'e>(
    walker: &mut Walker,
    slice: &'e str,
) -> Result<Option<Edn<'e>>, Error> {
    let _ = walker.nibble_next(slice); // Consume the leading '#' char

    match walker.peek_next(slice) {
        Some('{') => parse_set(walker, slice).map(Some),
        Some('_') => parse_discard(walker, slice),
        _ => parse_tag(walker).map(Some),
    }
}

#[inline]
fn parse_discard<'e>(walker: &mut Walker, slice: &'e str) -> Result<Option<Edn<'e>>, Error> {
    let _ = walker.nibble_next(slice); // Consume the leading '_' char
    Ok(match parse_internal(walker, slice)? {
        None => {
            return Err(Error {
                code: Code::UnexpectedEOF,
                line: Some(walker.line),
                column: Some(walker.column),
                ptr: Some(walker.ptr),
            })
        }
        _ => match walker.peek_next(slice) {
            Some(_) => parse_internal(walker, slice)?,
            None => return Ok(Some(Edn::Nil)),
        },
    })
}

#[inline]
fn parse_set<'e>(walker: &mut Walker, slice: &'e str) -> Result<Edn<'e>, Error> {
    let _ = walker.nibble_next(slice); // Consume the leading '{' char
    let mut set: BTreeSet<Edn<'_>> = BTreeSet::new();

    loop {
        match walker.peek_next(slice) {
            Some('}') => {
                let _ = walker.nibble_next(slice);
                return Ok(Edn::Set(set));
            }
            Some(n) => {
                if n == ']' || n == ')' {
                    return Err(Error {
                        code: Code::UnmatchedDelimiter(n),
                        line: Some(walker.line),
                        column: Some(walker.column),
                        ptr: Some(walker.ptr),
                    });
                }

                if let Some(n) = parse_internal(walker, slice)? {
                    if !set.insert(n) {
                        return Err(Error {
                            code: Code::SetDuplicateKey,
                            line: Some(walker.line),
                            column: Some(walker.column),
                            ptr: Some(walker.ptr),
                        });
                    };
                }
            }
            _ => {
                return Err(Error {
                    code: Code::UnexpectedEOF,
                    line: Some(walker.line),
                    column: Some(walker.column),
                    ptr: Some(walker.ptr),
                })
            }
        }
    }
}

#[inline]
#[allow(clippy::needless_pass_by_ref_mut)]
fn parse_tag<'e>(walker: &mut Walker) -> Result<Edn<'e>, Error> {
    Err(Error {
        code: Code::Unimplemented("Tagged Element"),
        line: Some(walker.line),
        column: Some(walker.column),
        ptr: Some(walker.ptr),
    })
}

#[inline]
fn parse_map<'e>(walker: &mut Walker, slice: &'e str) -> Result<Edn<'e>, Error> {
    let _ = walker.nibble_next(slice); // Consume the leading '{' char
    let mut map: BTreeMap<Edn<'_>, Edn<'_>> = BTreeMap::new();
    loop {
        match walker.peek_next(slice) {
            Some('}') => {
                let _ = walker.nibble_next(slice);
                return Ok(Edn::Map(map));
            }
            Some(n) => {
                if n == ']' || n == ')' {
                    return Err(Error {
                        code: Code::UnmatchedDelimiter(n),
                        line: Some(walker.line),
                        column: Some(walker.column),
                        ptr: Some(walker.ptr),
                    });
                }

                let key = parse_internal(walker, slice)?;
                let val = parse_internal(walker, slice)?;

                // When this is not true, errors are caught on the next loop
                if let (Some(k), Some(v)) = (key, val) {
                    // Existing keys are considered an error
                    if map.insert(k, v).is_some() {
                        return Err(Error {
                            code: Code::HashMapDuplicateKey,
                            line: Some(walker.line),
                            column: Some(walker.column),
                            ptr: Some(walker.ptr),
                        });
                    }
                }
            }
            _ => {
                return Err(Error {
                    code: Code::UnexpectedEOF,
                    line: Some(walker.line),
                    column: Some(walker.column),
                    ptr: Some(walker.ptr),
                })
            }
        }
    }
}

#[inline]
fn parse_vector<'e>(walker: &mut Walker, slice: &'e str, delim: char) -> Result<Edn<'e>, Error> {
    let _ = walker.nibble_next(slice); // Consume the leading '[' char
    let mut vec = Vec::new();

    loop {
        match walker.peek_next(slice) {
            Some(p) => {
                if p == delim {
                    let _ = walker.nibble_next(slice);
                    if delim == ']' {
                        return Ok(Edn::Vector(vec));
                    }

                    return Ok(Edn::List(vec));
                }

                if let Some(next) = parse_internal(walker, slice)? {
                    vec.push(next);
                } else {
                    let _ = walker.nibble_next(slice);
                }
            }
            _ => {
                return Err(Error {
                    code: Code::UnexpectedEOF,
                    line: Some(walker.line),
                    column: Some(walker.column),
                    ptr: Some(walker.ptr),
                })
            }
        }
    }
}

#[inline]
fn edn_literal(literal: &str) -> Result<Option<Edn<'_>>, Code> {
    fn numeric(s: &str) -> bool {
        let (first, second) = {
            let mut s = s.chars();
            (s.next(), s.next())
        };

        let first = first.expect("Empty str is previously caught as nil");
        if first.is_numeric() {
            return true;
        }

        if first == '-' || first == '+' {
            if let Some(s) = second {
                if s.is_numeric() {
                    return true;
                }
            }
        }

        false
    }

    Ok(match literal {
        "nil" => Some(Edn::Nil),
        "true" => Some(Edn::Bool(true)),
        "false" => Some(Edn::Bool(false)),
        "" => None,
        k if k.starts_with(':') => {
            if k.len() <= 1 {
                return Err(Code::InvalidKeyword);
            }
            Some(Edn::Key(k))
        }
        n if numeric(n) => return Ok(Some(parse_number(n)?)),
        _ => Some(Edn::Symbol(literal)),
    })
}

#[inline]
fn parse_char(lit: &str) -> Result<Edn<'_>, Code> {
    let lit = &lit[1..]; // ignore the leading '\\'
    match lit {
        "newline" => Ok(Edn::Char('\n')),
        "return" => Ok(Edn::Char('\r')),
        "tab" => Ok(Edn::Char('\t')),
        "space" => Ok(Edn::Char(' ')),
        c if c.len() == 1 => Ok(Edn::Char(c.chars().next().unwrap())),
        _ => Err(Code::InvalidChar),
    }
}

#[inline]
fn parse_number(lit: &str) -> Result<Edn<'_>, Code> {
    let mut chars = lit.chars().peekable();
    let (number, radix, polarity) = {
        let mut num_ptr_start = 0;
        let polarity = chars.peek().map_or(1i8, |c| {
            if *c == '-' {
                num_ptr_start += 1;
                -1i8
            } else if *c == '+' {
                // The EDN spec allows for a redundant '+' symbol, we just ignore it.
                num_ptr_start += 1;
                1i8
            } else {
                1i8
            }
        });

        let mut number = &lit[num_ptr_start..];

        if number.to_lowercase().starts_with("0x") {
            number = &number[2..];
            (number, 16, polarity)
        } else if let Some(index) = number.to_lowercase().find('r') {
            let radix = (number[0..index]).parse::<u8>();

            match radix {
                Ok(r) => {
                    // from_str_radix panics if radix is not in the range from 2 to 36
                    if !(2..=36).contains(&r) {
                        return Err(Code::InvalidRadix(Some(r)));
                    }

                    number = &number[(index + 1)..];
                    (number, r, polarity)
                }
                Err(_) => {
                    return Err(Code::InvalidRadix(None));
                }
            }
        } else {
            (number, 10, polarity)
        }
    };

    if let Ok(n) = i64::from_str_radix(number, radix.into()) {
        return Ok(Edn::Int(n * i64::from(polarity)));
    }
    #[cfg(feature = "floats")]
    if let Ok(n) = number.parse::<f64>() {
        return Ok(Edn::Double((n * f64::from(polarity)).into()));
    }
    if let Some((n, d)) = num_den_from_slice(number) {
        return Ok(Edn::Rational((n, d)));
    }

    Err(Code::InvalidNumber)
}

#[inline]
fn num_den_from_slice(slice: &str) -> Option<(i64, i64)> {
    let index = slice.find('/');

    if let Some(i) = index {
        let (num, den) = slice.split_at(i); // This can't panic because the index is valid
        let num = num.parse::<i64>();
        let den = den[1..].parse::<i64>();

        if let (Ok(n), Ok(d)) = (num, den) {
            return Some((n, d));
        }
    }
    None
}

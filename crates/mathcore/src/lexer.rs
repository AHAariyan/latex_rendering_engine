//! On-demand tokenizer for TeX math mode.
//!
//! The lexer is pull-based because the parser sometimes needs raw source
//! (for `\text{...}`) instead of tokens.

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok<'a> {
    /// A control sequence without the leading backslash: `frac`, `,` or `\`.
    Cmd(&'a str),
    Char(char),
    LBrace,
    RBrace,
    Sup,
    Sub,
    Amp,
    Eof,
}

pub struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    peeked: Option<(usize, Tok<'a>)>,
    /// Byte offset just past the last token actually consumed, which is where
    /// a source span ends. `pos` would include the whitespace before the next.
    last_end: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer {
            src,
            pos: 0,
            peeked: None,
            last_end: 0,
        }
    }

    /// Byte offset of the next unread token.
    pub fn pos(&self) -> usize {
        match &self.peeked {
            Some((p, _)) => *p,
            None => self.pos,
        }
    }

    pub fn peek(&mut self) -> Result<&Tok<'a>> {
        if self.peeked.is_none() {
            let start = self.skip_ws();
            let tok = self.lex()?;
            self.peeked = Some((start, tok));
        }
        Ok(&self.peeked.as_ref().unwrap().1)
    }

    pub fn advance(&mut self) -> Result<Tok<'a>> {
        if let Some((_, t)) = self.peeked.take() {
            self.last_end = self.pos;
            return Ok(t);
        }
        self.skip_ws();
        let t = self.lex();
        self.last_end = self.pos;
        t
    }

    /// Byte offset just past the last consumed token.
    pub fn end(&self) -> usize {
        self.last_end
    }

    /// Reads a balanced `{...}` group as raw text, or a single character.
    /// Used for `\text{}`, `\operatorname{}` and environment names.
    pub fn raw_group(&mut self) -> Result<&'a str> {
        if let Some((p, tok)) = self.peeked.take() {
            // Re-lex from the peeked position so the raw scan sees the braces.
            self.pos = p;
            let _ = tok;
        }
        self.skip_ws();
        let start = self.pos;
        let mut chars = self.src[start..].char_indices();
        match chars.next() {
            Some((_, '{')) => {}
            Some((i, c)) => {
                self.pos = start + i + c.len_utf8();
                self.last_end = self.pos;
                return Ok(&self.src[start..self.pos]);
            }
            None => return Err(Error::parse(start, "expected a group")),
        }
        let mut depth = 1usize;
        let mut prev_backslash = false;
        for (i, c) in chars {
            if prev_backslash {
                prev_backslash = false;
                continue;
            }
            match c {
                '\\' => prev_backslash = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        self.pos = start + i + 1;
                        self.last_end = self.pos;
                        return Ok(&self.src[start + 1..start + i]);
                    }
                }
                _ => {}
            }
        }
        Err(Error::parse(start, "unbalanced group"))
    }

    /// Reads `\verb<delim>text<delim>`, returning the text between the
    /// delimiters. Any character may be the delimiter, as TeX allows.
    pub fn verbatim(&mut self) -> Option<&'a str> {
        self.peeked = None;
        let rest = &self.src[self.pos..];
        let mut chars = rest.char_indices();
        let (_, delim) = chars.next()?;
        let start = self.pos + delim.len_utf8();
        let end = self.src[start..].find(delim)? + start;
        self.pos = end + delim.len_utf8();
        self.last_end = self.pos;
        Some(&self.src[start..end])
    }

    fn skip_ws(&mut self) -> usize {
        let bytes = self.src.as_bytes();
        loop {
            while self.pos < bytes.len() && bytes[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.pos < bytes.len() && bytes[self.pos] == b'%' {
                while self.pos < bytes.len() && bytes[self.pos] != b'\n' {
                    self.pos += 1;
                }
                continue;
            }
            break;
        }
        self.pos
    }

    fn lex(&mut self) -> Result<Tok<'a>> {
        let rest = &self.src[self.pos..];
        let mut it = rest.char_indices();
        let Some((_, c)) = it.next() else { return Ok(Tok::Eof) };
        let start = self.pos;
        self.pos += c.len_utf8();
        Ok(match c {
            '{' => Tok::LBrace,
            '}' => Tok::RBrace,
            '^' => Tok::Sup,
            '_' => Tok::Sub,
            '&' => Tok::Amp,
            '\\' => {
                let name_start = self.pos;
                let mut end = name_start;
                for (i, ch) in self.src[name_start..].char_indices() {
                    if ch.is_ascii_alphabetic() {
                        end = name_start + i + ch.len_utf8();
                    } else {
                        break;
                    }
                }
                if end == name_start {
                    // Single non-letter control symbol such as `\,` or `\\`.
                    let Some(ch) = self.src[name_start..].chars().next() else {
                        return Err(Error::parse(start, "dangling backslash"));
                    };
                    end = name_start + ch.len_utf8();
                }
                self.pos = end;
                Tok::Cmd(&self.src[name_start..end])
            }
            '#' => return Err(Error::parse(start, "macro parameter outside of a macro")),
            _ => Tok::Char(c),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(src: &str) -> Vec<Tok<'_>> {
        let mut l = Lexer::new(src);
        let mut v = vec![];
        loop {
            let t = l.advance().unwrap();
            if t == Tok::Eof {
                break;
            }
            v.push(t);
        }
        v
    }

    #[test]
    fn commands_and_chars() {
        assert_eq!(
            all(r"\frac{a}^2 \, % comment
            b"),
            vec![
                Tok::Cmd("frac"),
                Tok::LBrace,
                Tok::Char('a'),
                Tok::RBrace,
                Tok::Sup,
                Tok::Char('2'),
                Tok::Cmd(","),
                Tok::Char('b')
            ]
        );
    }

    #[test]
    fn raw_group_reads_nested_braces() {
        let mut l = Lexer::new(r"\text{a {b} c}x");
        assert_eq!(l.advance().unwrap(), Tok::Cmd("text"));
        assert_eq!(l.raw_group().unwrap(), "a {b} c");
        assert_eq!(l.advance().unwrap(), Tok::Char('x'));
    }

    #[test]
    fn raw_group_after_peek() {
        let mut l = Lexer::new(r"{abc}");
        let _ = l.peek().unwrap();
        assert_eq!(l.raw_group().unwrap(), "abc");
    }
}

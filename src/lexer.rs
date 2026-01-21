use std::{cell::UnsafeCell, char, panic, str};

use crate::arena::{Arena, ArenaVec};

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Token<'a> {
    At,
    Hashtag,
    Colon,
    OParen,
    CParen,
    OBracket,
    CBracket,
    Plus,
    Minus,
    Slash,
    Asterisk,
    Percent,
    #[allow(unused)] Dot,
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanOrEquals,
    LessThan,
    LessThanOrEquals,
    Not,
    And,
    Bar,
    Comma,
    Concat,
    If,
    Else,
    For,
    In,
    Ident(&'a str),
    Boolean(bool),
    Number(f32),
    String(&'a str),
    Keys,
    Base,
}

pub type Template<'a> = &'a [Token<'a>];

#[derive(Debug, PartialEq)]
pub(crate) enum DocumentKind<'a, 's> {
    Markup(&'s str),
    Template(Template<'a>),
}

pub(crate) struct Lexer<'a, 's> {
    contents: UnsafeCell<&'s str>, // I'm sorry
    arena: &'a Arena<'a>,
}

enum Status {
    Continue,
    Eof,
}

impl<'a, 's> Lexer<'a, 's> {
    pub fn new(contents: &'s str, arena: &'a Arena<'a>) -> Self {
        Lexer {
            contents: UnsafeCell::new(contents),
            arena,
        }
    }

    fn next_char(&self) -> Option<char> {
        let str = unsafe { *self.contents.get() };
        let next = str.chars().next();
        self.advance();
        next
    }

    fn advance(&self) {
        let str = unsafe { &mut *self.contents.get() };
        *str = &str[1..];
    }

    fn advance_n(&self, n: usize) {
        let str = unsafe { &mut *self.contents.get() };
        *str = &str[n..];
    }

    fn peek_char(&self) -> Option<char> {
        let str = unsafe { *self.contents.get() };
        str.chars().next()
    }

    fn nth(&self, n: usize) -> Option<char> {
        unsafe { *self.contents.get() }
            .as_bytes()
            .iter()
            .nth(n)
            .map(|&b| b as char) 
    }

    fn skip_whitespace(&self) {
        while let Some(char) = self.peek_char() {
            if !char.is_whitespace() {
                return;
            }
            self.advance();
        }
    }

    fn read_until(&self, target: char) -> (&'s str, Status) {
        let str = unsafe { &mut *self.contents.get() };
        let mut n = 0;
        while let Some(char) = self.nth(n) {
            if char == target {
                let res = (&str[0..n], Status::Continue);
                self.advance_n(n + 1); // +1 to skip the target character
                return res;
            }
            n += 1;
        }

        let res = (&str[0..n], Status::Eof);
        self.advance_n(n);
        res
    }

    fn read_while(&self, predicate: impl Fn(char) -> bool) -> &str {
        let str = unsafe { &mut *self.contents.get() };
        let mut n = 0;
        while let Some(char) = self.nth(n) {
            if !predicate(char) {
                break;
            }
            n += 1;
        }

        let res = &str[0..n];
        self.advance_n(n);
        res
    }

    fn next_ident(&self) -> Token<'a> {
        let string = self.read_while(|char| char.is_alphanumeric() || char == '_');

        let token = match string {
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "in" => Token::In,
            "keys" => Token::Keys,
            "base" => Token::Base,
            "true" => Token::Boolean(true),
            "false" => Token::Boolean(false),
            _ => Token::Ident(self.arena.alloc_str(string)),
        };
        token
    }

    fn next_number(&self) -> Token<'a> {
        if let Ok(number) = self.read_while(|char| char.is_numeric() || char == '.').parse() {
            return Token::Number(number);
        } else {
            panic!("Error reading number");
        }
    }

    fn unescape(character: char) -> char {
        match character {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            anything_else => anything_else,
        }
    }

    fn next_string(&self) -> Token<'a> {
        let open_quote = self.next_char();
        debug_assert_eq!(open_quote, Some('"'));
        
        let mut string = ArenaVec::new(self.arena);
        let mut backslash_found = false;
        let mut char_buf = [0; 4];
        while let Some(char) = self.next_char() {
            if backslash_found {
                Self::unescape(char).encode_utf8(&mut char_buf).as_bytes().iter()
                    .for_each(|b| string.push(*b));
                backslash_found = false;
                continue;
            }
            if char == '"' { break };
            if char == '\\' {
                backslash_found = true;
                continue;
            }

            char.encode_utf8(&mut char_buf).as_bytes().iter()
                .for_each(|b| string.push(*b));
        }

        Token::String(unsafe {
            str::from_utf8_unchecked(string.into_slice())
        })
    }

    fn next_literal(&self) -> Token<'a> {
        if let Some(peek) = self.peek_char() {
            if peek == '"' {
                return self.next_string();
            } else if peek.is_numeric() {
                return self.next_number();
            } else {
                return self.next_ident();
            }
        }
        unreachable!()
    }

    fn next_token(&self) -> Option<Token<'a>> {
        self.skip_whitespace();

        let first = self.peek_char()?;
        if matches!(first, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '"') {
            return Some(self.next_literal());
        }
        self.advance();

        let second = self.peek_char();
        let result = match (first, second) {
            ('!', Some('=')) => {
                self.advance();
                Some(Token::NotEquals)
            }
            ('<', Some('=')) => {
                self.advance();
                Some(Token::LessThanOrEquals)
            }
            ('>', Some('=')) => {
                self.advance();
                Some(Token::GreaterThanOrEquals)
            }
            ('+', Some('+')) => {
                self.advance();
                Some(Token::Concat)
            }
        
            ('@', _) => Some(Token::At),
            ('#', _) => Some(Token::Hashtag),
            (':', _) => Some(Token::Colon),
            ('/', _) => Some(Token::Slash),
            (',', _) => Some(Token::Comma),
            ('+', _) => Some(Token::Plus),
            ('-', _) => Some(Token::Minus),
            ('*', _) => Some(Token::Asterisk),
            ('%', _) => Some(Token::Percent),
            ('!', _) => Some(Token::Not),
            ('&', _) => Some(Token::And),
            ('|', _) => Some(Token::Bar),
            ('=', _) => Some(Token::Equals),
            ('<', _) => Some(Token::LessThan),
            ('>', _) => Some(Token::GreaterThan),
            ('(', _) => Some(Token::OParen),
            (')', _) => Some(Token::CParen),
            ('[', _) => Some(Token::OBracket),
            (']', _) => Some(Token::CBracket),
            ('}', _) => None,

            // having return here skips `self.current += 1` below the match stmt
            (first, _) => panic!("Unexpected character in template: {}", first),
        };

        result
    }

    fn next_template(&self) -> Template<'a> {
        let mut template = Vec::new();
        while let Some(token) = self.next_token() {
            template.push(token);
        }
        self.arena.alloc_slice(template.as_ref())
    }

    // pub fn execute(self: &'self mut Self<'a>) -> Vec<DocumentKind<'s>> {
    // 1. 'self |> return lives as long as &self lives
    // 2. 'a |> data in self lives as long as self lives 
    // 3. 'a: 'self
    pub fn execute(self, buf: &mut Vec<DocumentKind<'a, 's>>) {
        loop {
            match self.read_until('{') {
                (before, Status::Continue) => buf.push(DocumentKind::Markup(before)),
                (before, Status::Eof) => {
                    buf.push(DocumentKind::Markup(before));
                    break;
                }
            }

            let template = self.next_template();
            buf.push(DocumentKind::Template(template));
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::arena;
//     use super::*;

//     const ARENA_SIZE: usize = 8 * 1024;

//     #[test]
//     fn categorizes_markup_and_templates() {
//         let contents = "markup{}end";
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         assert_eq!(lexer.execute(), vec![
//             DocumentKind::Markup("markup"),
//             DocumentKind::Template(vec![]),
//             DocumentKind::Markup("end"),
//         ]);
//     }

//     #[test]
//     fn lexes_multiple_templates() {
//         let contents = "markup 1: {}markup 2: {}markup 3: {}";
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         assert_eq!(lexer.execute(), vec![
//             DocumentKind::Markup("markup 1: "),
//             DocumentKind::Template(vec![]),
//             DocumentKind::Markup("markup 2: "),
//             DocumentKind::Template(vec![]),
//             DocumentKind::Markup("markup 3: "),
//             DocumentKind::Template(vec![]),
//             DocumentKind::Markup(""),
//         ]);
//     }

//     #[test]
//     fn skips_whitespace_and_recongnizes_idents() {
//         let contents = "{      variable_1       }";
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         assert_eq!(lexer.execute(), vec![
//             DocumentKind::Markup(""),
//             DocumentKind::Template(vec![Token::Ident("variable_1")]),
//             DocumentKind::Markup(""),
//         ]);
//     }

//     #[test]
//     fn recognizes_string() {
//         let contents = r#"{"lorem ipsum"}"#;
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         assert_eq!(lexer.execute(), vec![
//             DocumentKind::Markup(""),
//             DocumentKind::Template(vec![Token::String("lorem ipsum")]),
//             DocumentKind::Markup(""),
//         ]);
//     }

//     #[test]
//     fn recognizes_escaped_string() {
//         let contents = r#"{"\"lorem\\ipsum\"\n"}"#;
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         assert_eq!(lexer.execute(), vec![
//             DocumentKind::Markup(""),
//             DocumentKind::Template(vec![Token::String("\"lorem\\ipsum\"\n")]),
//             DocumentKind::Markup(""),
//         ]);
//     }

//     #[test]
//     #[should_panic]
//     fn panics_on_deformed_escape_char() {
//         let contents = r#"{\q}"#;
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         lexer.execute();
//     }

//     #[test]
//     fn recognizes_number() {
//         let contents = "{23491.23}";
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         assert_eq!(lexer.execute(), &[
//             DocumentKind::Markup(""),
//             DocumentKind::Template(&[Token::Number(23491.23)]),
//             DocumentKind::Markup(""),
//         ]);
//     }

//     #[test]
//     #[should_panic]
//     fn panics_on_deformed_number() {
//         let contents = "{2s3491.23}";
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         lexer.execute();
//     }

//     #[test]
//     fn recognizes_boolean() {
//         let contents = "{true} {false}";
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         assert_eq!(lexer.execute(), &[
//             DocumentKind::Markup(""),
//             DocumentKind::Template(&[Token::Boolean(true)]),
//             DocumentKind::Markup(" "),
//             DocumentKind::Template(&[Token::Boolean(false)]),
//             DocumentKind::Markup(""),
//         ]);
//     }

//     #[test]
//     fn recognizes_keywords() {
//         let contents = "{if else for in keys}";
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         assert_eq!(lexer.execute(), &[
//             DocumentKind::Markup(""),
//             DocumentKind::Template(&[
//                 Token::If,
//                 Token::Else,
//                 Token::For,
//                 Token::In,
//                 Token::Keys,
//             ]),
//             DocumentKind::Markup(""),
//         ]);
//     }

//     #[test]
//     fn recognizes_tokens() {
//         let contents = "{#:/@}";
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         assert_eq!(lexer.execute(), &[
//             DocumentKind::Markup(""),
//             DocumentKind::Template(&[
//                 Token::Hashtag,
//                 Token::Colon,
//                 Token::Slash,
//                 Token::At,
//             ]),
//             DocumentKind::Markup(""),
//         ]);
//     }

//     #[test]
//     fn recognizes_two_length_tokens() {
//         let contents = "{<= >= != ++}";
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         assert_eq!(lexer.execute(), &[
//             DocumentKind::Markup(""),
//             DocumentKind::Template(&[
//                 Token::LessThanOrEquals,
//                 Token::GreaterThanOrEquals,
//                 Token::NotEquals,
//                 Token::Concat,
//             ]),
//             DocumentKind::Markup(""),
//         ]);
//     }

//     #[test]
//     fn bunch_of_stuff() {
//         let contents = "{#if len(list) > 4 & true}and {\"yes \" ++ \"it works\"}.{:else}no{/}";
//         let arena = arena::Arena::new(ARENA_SIZE);
//         let mut lexer = Lexer::new(&contents, &arena);
//         assert_eq!(lexer.execute(), &[
//             DocumentKind::Markup(""),
//             DocumentKind::Template(&[
//                 Token::Hashtag,
//                 Token::If,
//                 Token::Ident("len"),
//                 Token::OParen,
//                 Token::Ident("list"),
//                 Token::CParen,
//                 Token::GreaterThan,
//                 Token::Number(4.0),
//                 Token::And,
//                 Token::Boolean(true),
//             ]),
//             DocumentKind::Markup("and "),
//             DocumentKind::Template(&[
//                 Token::String("yes "),
//                 Token::Concat,
//                 Token::String("it works"),
//             ]),
//             DocumentKind::Markup("."),
//             DocumentKind::Template(&[
//                 Token::Colon,
//                 Token::Else
//             ]),
//             DocumentKind::Markup("no"),
//             DocumentKind::Template(&[
//                 Token::Slash,
//             ]),
//             DocumentKind::Markup(""),
//         ]);
//     }
// }
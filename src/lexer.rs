use std::{cell::UnsafeCell, char, panic};

#[derive(Debug, PartialEq)]
pub(crate) enum Token {
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
    Ident(String),
    Boolean(bool),
    Number(f32),
    String(String),
    Keys,
    Base,
}

type Template = Vec<Token>;

#[derive(Debug, PartialEq)]
pub(crate) enum DocumentKind<'a> {
    Markup(&'a str),
    Template(Template),
}

pub(crate) struct Lexer<'a> {
    contents: UnsafeCell<&'a str>, // I'm sorry
}

impl<'a> Lexer<'a> {
    pub fn new(contents: &'a str) -> Self {
        Lexer {
            contents: UnsafeCell::new(contents),
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

    fn read_until(&self, target: char) -> Result<&str, &str> {
        let str = unsafe { &mut *self.contents.get() };
        let mut n = 0;
        while let Some(char) = self.nth(n) {
            if char == target {
                let res = Ok(&str[0..n]);
                self.advance_n(n + 1); // +1 to skip the target character
                return res;
            }
            n += 1;
        }

        let res = Err(&str[0..n]);
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

    fn next_ident(&self) -> Token {
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
            _ => Token::Ident(string.to_owned()),
        };
        token
    }

    fn next_number(&self) -> Token {
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

    fn next_string(&self) -> Token {
        let open_quote = self.next_char();
        debug_assert_eq!(open_quote, Some('"'));
        
        let mut string = String::new();
        let mut backslash_found = false;
        while let Some(char) = self.next_char() {
            if backslash_found {
                string.push(Self::unescape(char));
                backslash_found = false;
                continue;
            }
            if char == '"' {
                return Token::String(string);
            }
            if char == '\\' {
                backslash_found = true;
                continue;
            }

            string.push(char);
        }

        Token::String(string)
    }

    fn next_literal(&self) -> Token {
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

    fn next_token(&self) -> Option<Token> {
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

    fn next_template(&self) -> Template {
        let mut template = Vec::new();
        while let Some(token) = self.next_token() {
            template.push(token);
        }
        template
    }

    // pub fn execute(self: &'s mut Self<'a>) -> Vec<DocumentKind<'s>> {
    // 1. 's |> return lives as long as &self lives
    // 2. 'a |> data in self lives as long as self lives 
    // 3. 'a: 's
    pub fn execute(&mut self) -> Vec<DocumentKind> {
        let mut tokens = Vec::new();
        loop {
            match self.read_until('{') {
                Ok(before) => tokens.push(DocumentKind::Markup(before)),
                Err(before) => {
                    tokens.push(DocumentKind::Markup(before));
                    break;
                }
            }

            let template = self.next_template();
            tokens.push(DocumentKind::Template(template));
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorizes_markup_and_templates() {
        let contents = "markup{}end".to_owned();
        let mut lexer = Lexer::new(&contents);
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("markup"),
            DocumentKind::Template(vec![]),
            DocumentKind::Markup("end"),
        ]);
    }

    #[test]
    fn lexes_multiple_templates() {
        let contents = "markup 1: {}markup 2: {}markup 3: {}".to_owned();
        let mut lexer = Lexer::new(&contents);
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("markup 1: "),
            DocumentKind::Template(vec![]),
            DocumentKind::Markup("markup 2: "),
            DocumentKind::Template(vec![]),
            DocumentKind::Markup("markup 3: "),
            DocumentKind::Template(vec![]),
            DocumentKind::Markup(""),
        ]);
    }

    #[test]
    fn skips_whitespace_and_recongnizes_idents() {
        let contents = "{      variable_1       }".to_owned();
        let mut lexer = Lexer::new(&contents);
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup(""),
            DocumentKind::Template(vec![Token::Ident("variable_1".to_owned())]),
            DocumentKind::Markup(""),
        ]);
    }

    #[test]
    fn recognizes_string() {
        let contents = r#"{"lorem ipsum"}"#.to_owned();
        let mut lexer = Lexer::new(&contents);
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup(""),
            DocumentKind::Template(vec![Token::String("lorem ipsum".to_owned())]),
            DocumentKind::Markup(""),
        ]);
    }

    #[test]
    fn recognizes_escaped_string() {
        let contents = r#"{"\"lorem\\ipsum\"\n"}"#.to_owned();
        let mut lexer = Lexer::new(&contents);
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup(""),
            DocumentKind::Template(vec![Token::String("\"lorem\\ipsum\"\n".to_owned())]),
            DocumentKind::Markup(""),
        ]);
    }

    #[test]
    #[should_panic]
    fn panics_on_deformed_escape_char() {
        let contents = r#"{\q}"#.to_owned();
        let mut lexer = Lexer::new(&contents);
        lexer.execute();
    }

    #[test]
    fn recognizes_number() {
        let contents = "{23491.23}".to_owned();
        let mut lexer = Lexer::new(&contents);
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup(""),
            DocumentKind::Template(vec![Token::Number(23491.23)]),
            DocumentKind::Markup(""),
        ]);
    }

    #[test]
    #[should_panic]
    fn panics_on_deformed_number() {
        let contents = "{2s3491.23}".to_owned();
        let mut lexer = Lexer::new(&contents);
        lexer.execute();
    }

    #[test]
    fn recognizes_boolean() {
        let contents = "{true} {false}".to_owned();
        let mut lexer = Lexer::new(&contents);
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup(""),
            DocumentKind::Template(vec![Token::Boolean(true)]),
            DocumentKind::Markup(" "),
            DocumentKind::Template(vec![Token::Boolean(false)]),
            DocumentKind::Markup(""),
        ]);
    }

    #[test]
    fn recognizes_keywords() {
        let contents = "{if else for in keys}".to_owned();
        let mut lexer = Lexer::new(&contents);
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup(""),
            DocumentKind::Template(vec![
                Token::If,
                Token::Else,
                Token::For,
                Token::In,
                Token::Keys,
            ]),
            DocumentKind::Markup(""),
        ]);
    }

    #[test]
    fn recognizes_tokens() {
        let contents = "{#:/@}".to_owned();
        let mut lexer = Lexer::new(&contents);
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup(""),
            DocumentKind::Template(vec![
                Token::Hashtag,
                Token::Colon,
                Token::Slash,
                Token::At,
            ]),
            DocumentKind::Markup(""),
        ]);
    }

    #[test]
    fn recognizes_two_length_tokens() {
        let contents = "{<= >= != ++}".to_owned();
        let mut lexer = Lexer::new(&contents);
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup(""),
            DocumentKind::Template(vec![
                Token::LessThanOrEquals,
                Token::GreaterThanOrEquals,
                Token::NotEquals,
                Token::Concat,
            ]),
            DocumentKind::Markup(""),
        ]);
    }

    #[test]
    fn bunch_of_stuff() {
        let contents = "{#if len(list) > 4 & true}and {\"yes \" ++ \"it works\"}.{:else}no{/}".to_owned();
        let mut lexer = Lexer::new(&contents);
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup(""),
            DocumentKind::Template(vec![
                Token::Hashtag,
                Token::If,
                Token::Ident("len".to_owned()),
                Token::OParen,
                Token::Ident("list".to_owned()),
                Token::CParen,
                Token::GreaterThan,
                Token::Number(4.0),
                Token::And,
                Token::Boolean(true),
            ]),
            DocumentKind::Markup("and "),
            DocumentKind::Template(vec![
                Token::String("yes ".to_owned()),
                Token::Concat,
                Token::String("it works".to_owned()),
            ]),
            DocumentKind::Markup("."),
            DocumentKind::Template(vec![
                Token::Colon,
                Token::Else
            ]),
            DocumentKind::Markup("no"),
            DocumentKind::Template(vec![
                Token::Slash,
            ]),
            DocumentKind::Markup(""),
        ]);
    }
}
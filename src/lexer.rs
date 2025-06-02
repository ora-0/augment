use std::{char, iter::Peekable, panic};

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
pub(crate) enum DocumentKind {
    Markup(String),
    Template(Template),
}

pub(crate) struct Lexer<T: Iterator<Item = char>> {
    contents: Peekable<T>,
}

impl<T: Iterator<Item = char>> Lexer<T> {
    pub fn new(contents: impl IntoIterator<IntoIter = T>) -> Self {
        Lexer {
            // Converting to a u8 slice, so that array access is O(1).
            // Previously, to index into `contents`, the code used
            // `content.chars().nth(i)`, which has to account for UTF-8
            // strings, thus making array access O(n).
            contents: contents.into_iter().peekable(),
        }
    }

    fn next_ident(&mut self) -> Token {
        let string = self.read_while(|char| char.is_alphanumeric() || char == '_');

        let token = match string.as_str() {
            "if" => Token::If,
            "else" => Token::Else,
            "for" => Token::For,
            "in" => Token::In,
            "keys" => Token::Keys,
            "base" => Token::Base,
            "true" => Token::Boolean(true),
            "false" => Token::Boolean(false),
            _ => Token::Ident(string),
        };
        token
    }

    fn next_number(&mut self) -> Token {
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

    fn next_string(&mut self) -> Token {
        self.contents.next();
        let mut string = String::new();
        let mut backslash_found = false;
        while let Some(&char) = self.contents.peek() {
            self.contents.next();
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

    fn next_literal(&mut self) -> Token {
        if let Some(&peek) = self.contents.peek() {
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

    fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();
        // let (first, second) = self.peek_pair();
        let &first = self.contents.peek()?;
        if matches!(first, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '"')  {
            return Some(self.next_literal());
        }

        self.contents.next();
        let second = self.contents.peek();
        let result = match (first, second) {
            ('!', Some('=')) => {
                self.contents.next();
                Some(Token::NotEquals)
            }
            ('<', Some('=')) => {
                self.contents.next();
                Some(Token::LessThanOrEquals)
            }
            ('>', Some('=')) => {
                self.contents.next();
                Some(Token::GreaterThanOrEquals)
            }
            ('+', Some('+')) => {
                self.contents.next();
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

            (first, _) => panic!("Unexpected character in template: {}", first),
        };

        result
    }

    fn next_template(&mut self) -> Template {
        let mut template = Vec::new();
        while let Some(token) = self.next_token() {
            template.push(token);
        }
        template
    }

    pub fn execute(&mut self) -> Vec<DocumentKind> {
        let mut tokens = Vec::new();
        loop {
            let before = self.read_until('{');
            tokens.push(DocumentKind::Markup(before));
            if self.contents.peek().is_none() {
                break;
            }

            let template = self.next_template();
            tokens.push(DocumentKind::Template(template));
        }

        tokens
    }

    fn skip_whitespace(&mut self) {
        while let Some(char) = self.contents.peek() {
            if !char.is_whitespace() {
                return;
            }
            self.contents.next();
        }
    }

    fn read_until(&mut self, target: char) -> String {
        self.contents.by_ref().take_while(|c| *c != target).collect()
    }

    fn read_while(&mut self, predicate: impl Fn(char) -> bool) -> String {
        let mut acc = String::new();
        while let Some(char) = self.contents.next_if(|c| predicate(*c)) {
            acc.push(char)
        }

        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorizes_markup_and_templates() {
        let contents = "markup{}end".to_owned();
        let mut lexer = Lexer::new(contents.chars());
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("markup".to_owned()),
            DocumentKind::Template(vec![]),
            DocumentKind::Markup("end".to_owned()),
        ]);
    }

    #[test]
    fn lexes_multiple_templates() {
        let contents = "markup 1: {}markup 2: {}markup 3: {}".to_owned();
        let mut lexer = Lexer::new(contents.chars());
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("markup 1: ".to_owned()),
            DocumentKind::Template(vec![]),
            DocumentKind::Markup("markup 2: ".to_owned()),
            DocumentKind::Template(vec![]),
            DocumentKind::Markup("markup 3: ".to_owned()),
            DocumentKind::Template(vec![]),
            DocumentKind::Markup("".to_owned()),
        ]);
    }

    #[test]
    fn skips_whitespace_and_recongnizes_idents() {
        let contents = "{      variable_1       }".to_owned();
        let mut lexer = Lexer::new(contents.chars());
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("".to_owned()),
            DocumentKind::Template(vec![Token::Ident("variable_1".to_owned())]),
            DocumentKind::Markup("".to_owned()),
        ]);
    }

    #[test]
    fn recognizes_string() {
        let contents = r#"{"lorem ipsum"}"#.to_owned();
        let mut lexer = Lexer::new(contents.chars());
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("".to_owned()),
            DocumentKind::Template(vec![Token::String("lorem ipsum".to_owned())]),
            DocumentKind::Markup("".to_owned()),
        ]);
    }

    #[test]
    fn recognizes_escaped_string() {
        let contents = r#"{"\"lorem\\ipsum\"\n"}"#.to_owned();
        let mut lexer = Lexer::new(contents.chars());
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("".to_owned()),
            DocumentKind::Template(vec![Token::String("\"lorem\\ipsum\"\n".to_owned())]),
            DocumentKind::Markup("".to_owned()),
        ]);
    }

    #[test]
    #[should_panic]
    fn panics_on_deformed_escape_char() {
        let contents = r#"{\q}"#.to_owned();
        let mut lexer = Lexer::new(contents.chars());
        lexer.execute();
    }

    #[test]
    fn recognizes_number() {
        let contents = "{23491.23}".to_owned();
        let mut lexer = Lexer::new(contents.chars());
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("".to_owned()),
            DocumentKind::Template(vec![Token::Number(23491.23)]),
            DocumentKind::Markup("".to_owned()),
        ]);
    }

    #[test]
    #[should_panic]
    fn panics_on_deformed_number() {
        let contents = "{2s3491.23}".to_owned();
        let mut lexer = Lexer::new(contents.chars());
        lexer.execute();
    }

    #[test]
    fn recognizes_boolean() {
        let contents = "{true} {false}".to_owned();
        let mut lexer = Lexer::new(contents.chars());
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("".to_owned()),
            DocumentKind::Template(vec![Token::Boolean(true)]),
            DocumentKind::Markup(" ".to_owned()),
            DocumentKind::Template(vec![Token::Boolean(false)]),
            DocumentKind::Markup("".to_owned()),
        ]);
    }

    #[test]
    fn recognizes_keywords() {
        let contents = "{if else for in keys}".to_owned();
        let mut lexer = Lexer::new(contents.chars());
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("".to_owned()),
            DocumentKind::Template(vec![
                Token::If,
                Token::Else,
                Token::For,
                Token::In,
                Token::Keys,
            ]),
            DocumentKind::Markup("".to_owned()),
        ]);
    }

    #[test]
    fn recognizes_tokens() {
        let contents = "{#:/@}".to_owned();
        let mut lexer = Lexer::new(contents.chars());
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("".to_owned()),
            DocumentKind::Template(vec![
                Token::Hashtag,
                Token::Colon,
                Token::Slash,
                Token::At,
            ]),
            DocumentKind::Markup("".to_owned()),
        ]);
    }

    #[test]
    fn recognizes_two_length_tokens() {
        let contents = "{<= >= != ++}".to_owned();
        let mut lexer = Lexer::new(contents.chars());
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("".to_owned()),
            DocumentKind::Template(vec![
                Token::LessThanOrEquals,
                Token::GreaterThanOrEquals,
                Token::NotEquals,
                Token::Concat,
            ]),
            DocumentKind::Markup("".to_owned()),
        ]);
    }

    #[test]
    fn bunch_of_stuff() {
        let contents = "{#if len(list) > 4 & true}and {\"yes \" ++ \"it works\"}.{:else}no{/}".to_owned();
        let mut lexer = Lexer::new(contents.chars());
        assert_eq!(lexer.execute(), vec![
            DocumentKind::Markup("".to_owned()),
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
            DocumentKind::Markup("and ".to_owned()),
            DocumentKind::Template(vec![
                Token::String("yes ".to_owned()),
                Token::Concat,
                Token::String("it works".to_owned()),
            ]),
            DocumentKind::Markup(".".to_owned()),
            DocumentKind::Template(vec![
                Token::Colon,
                Token::Else
            ]),
            DocumentKind::Markup("no".to_owned()),
            DocumentKind::Template(vec![
                Token::Slash,
            ]),
            DocumentKind::Markup("".to_owned()),
        ]);
    }
}
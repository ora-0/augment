use std::{char, fs::read_to_string, io, panic};

struct Block {
    head: String,
    content: String,
    tail: String,
    render: (),
}

impl Block {
    fn new(head: &str, content: &str, tail: &str) -> Self {
        Block {
            head: head.to_string(),
            content: content.to_string(),
            tail: tail.to_string(),
            render: (),
        }
    }
}

#[derive(Debug)]
enum Keyword {
    If,
    For,
    In,
}

#[derive(Debug)]
enum Token {
    Hashtag,
    Colon,
    Slash,
    OParen,
    CParen,
    Add,
    Sub,
    Mul,
    Div,
    Dot,
    Not,
    Keyword(Keyword),
    Ident(String),
}

type Template = Vec<Token>;
#[derive(Debug)]
enum DocumentKind {
    Markup(String),
    Template(Template),
}

struct Lexer {
    contents: String,
    current: usize,
}

impl Lexer {
    fn new<'a>(contents: String) -> Self {
        Lexer {
            contents,
            current: 0,
        }
    }

    fn next_char(&mut self) -> Option<char> {
        let result = self.contents.chars().nth(self.current);
        self.current += 1;
        result
    }

    fn peek_char(&self) -> Option<char> {
        self.contents.chars().nth(self.current)
    }

    fn next_ident(&mut self) -> Token {
        let mut string = String::new();
        while let Some(peek) = self.peek_char() {
            if !(peek.is_alphanumeric() || peek == '_') {
                // dbg!(peek);
                break;
            }
            string.push(peek);
            self.current += 1;
        }

        use Keyword::*;
        let token = match string.as_str() {
            "if" => Token::Keyword(If),
            "for" => Token::Keyword(If),
            "in" => Token::Keyword(If),
            _ => Token::Ident(string),
        };
        token
    }

    fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();
        let mut token = String::new();
        let char = self.peek_char()?;
        // dbg!(char);
        let result = match char {
            '#' => Some(Token::Hashtag),
            ':' => Some(Token::Colon),
            '/' => Some(Token::Slash),
            '}' => None,

            // having return here skips `self.current += 1` below the match stmt
            'a'..='z' | 'A'..='Z' | '_' => return Some(self.next_ident()),
            _ => panic!("Unexpected character in template"),
        };

        self.current += 1;
        result
    }

    fn next_template(&mut self) -> Template {
        let mut template = Vec::new();
        while let Some(token) = self.next_token() {
            template.push(token);
        }
        template
    }

    fn execute(&mut self) -> Vec<DocumentKind> {
        let mut tokens = Vec::new();
        loop {
            let Some(before) = self.read_until('{') else {
                break;
            };
            tokens.push(DocumentKind::Markup(before));

            let template = self.next_template();
            tokens.push(DocumentKind::Template(template));
        }

        tokens
    }

    fn skip_whitespace(&mut self) {
        while let Some(char) = self.peek_char() {
            // println!("{char} {skipped}, {curr}", skipped = !char.is_whitespace(), curr = self.current );
            if !char.is_whitespace() {
                return;
            }
            self.current += 1;
            // println!("{curr}", curr = self.current );
        }
    }

    fn read_until(&mut self, target: char) -> Option<String> {
        let mut string = String::new();
        while let Some(char) = self.next_char() {
            if char == target {
                return Some(string);
            }
            string.push(char);
        }

        // if self.next_char() is none, also return none
        None
    }
}

fn main() -> io::Result<()> {
    let contents = read_to_string("./test.html")?;

    let mut lexer = Lexer::new(contents);
    let result = lexer.execute();
    println!("{result:#?}");

    Ok(())
}

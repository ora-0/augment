mod lexer;
mod parser;
mod arena;
mod template;

use lexer::Lexer;
use parser::{Parser, Value};
use std::{collections::HashMap, env, fs::read_to_string, io::{self, Read, stdin}, path::PathBuf, str::Chars};
use template::{Environment, Augment};

use crate::arena::Arena;

struct ArgumentParser<'a> {
    arena: &'a Arena<'a>,
    // scratch: RefCell<String>,
}

impl<'a> ArgumentParser<'a> {
    fn new(arena: &'a Arena<'a>) -> Self {
        Self {
            arena,
            // scratch: String::with_capacity(512).into(),
        }
    }

    fn parse_value(&mut self, value: &str) -> Value<'a> {
        if value.starts_with('"') && value.ends_with('"') {
            let string = &value[1..value.len()-1];
            Value::String(self.arena.alloc_str(string))
        } else if value.starts_with('[') && value.ends_with(']') {
            let mut inner = value.chars();
            inner.next();
            inner.next_back();
            self.parse_array(&mut inner)
        } else if value.is_empty() {
            Value::Null
        } else if value.starts_with(char::is_numeric) {
            let number = value.parse().expect("Failed to parse number");
            Value::Number(number)
        } else {
            match value {
                "true" => Value::Boolean(true),
                "false" => Value::Boolean(false),
                string => Value::String(self.arena.alloc_str(string)),
            }
        }
    }

    fn parse_array(&mut self, inner: &mut Chars) -> Value<'a> {
        let mut vec = Vec::new(); // recursive call, better not allocate it in arena
        let mut scratch = String::new();
        while let Some(char) = inner.next() {
            if char == '[' {
                scratch.clear();
                vec.push(self.parse_array(inner));
                continue;
            }
            if char == ']' {
                if !scratch.is_empty() {
                    vec.push(self.parse_value(&scratch));
                }
                scratch.clear();
                break;
            }
            if char == ',' {
                if !scratch.is_empty() {
                    vec.push(self.parse_value(&scratch));
                }
                scratch.clear();
                continue;
            }
            if char.is_whitespace() && scratch.is_empty() {
                continue;
            }
            scratch.push(char);
        }

        let vec = self.arena.alloc_slice(&vec);
        Value::Array(vec)
    }

    fn parse_argument(&mut self, param: String) -> (&'a str, Value<'a>) {
        let Some((ident, value)) = param.split_once('=') else {
            panic!("Expected equals sign in parameter specification. Example: username=\"John\"")
        };
        let value = value.trim();
        let value = self.parse_value(value);
        (self.arena.alloc_str(ident), value)
    }
}

fn read_from_stdin() -> String {
    let mut handle = stdin().lock();
    let mut buf = Vec::new();
    handle.read_to_end(&mut buf).expect("Failed to read from stdin");
    match String::from_utf8(buf) {
        Ok(string) => string,
        Err(err) => panic!("Failed convert stdin to string: {err}"),
    }
}

const ARENA_SIZE: usize = 16 * 1024;

/// returns (the file templated, the base template that this one extends from)
fn template_a_file<'a, 'env>(contents: String, arena: &'a Arena<'a>, env: &'env mut Environment<'a>) -> (String, Option<PathBuf>) {
    // use std::time::Instant;
    // let before = Instant::now();
    // println!("{:?}", Instant::now() - before);
 
    let mut result = Vec::new();
    let lexer = Lexer::new(&contents, &arena);
    lexer.execute(&mut result);

    let parser = Parser::new(&arena);
    let (result, base_template) = parser.execute(result);

    let iter = result.iter();
    let templater = Augment::new(iter, env);
    let result = templater.execute();

    (result, base_template)
}

fn main() -> io::Result<()> {
    let mut arguments = env::args().peekable();
    arguments.next();

    let mut env = HashMap::new();
    env.insert("slot", Value::String("".into()));

    let arena = arena::Arena::new(ARENA_SIZE);

    // parse cmd line arguments
    let mut advance = false;
    let contents = arguments.peek().map(|argument| {
        if argument.starts_with('-') {
            return read_from_stdin();
        }

        advance = true;
        match read_to_string(argument) {
            Ok(contents) => return contents,
            Err(err) => panic!("Failed to read file: {err}"),
        }
    }).unwrap_or_else(read_from_stdin);

    if advance { arguments.next(); }
    if let Some(argument) = arguments.next() {
        let mut parser = ArgumentParser::new(&arena);
        if argument == "-i" {
            arguments.for_each(|arg| {
                let (k, v) = parser.parse_argument(arg);
                env.insert(k, v);
            });
        }
    }

    let mut to_be_templated = contents;
    loop {
        let (result, base_template) = template_a_file(to_be_templated, &arena, &mut env);
        if let Some(path) = base_template {
            to_be_templated = read_to_string(path).unwrap();
            env.insert("slot", Value::String(result.leak()));
        } else {
            println!("{result}");
            break;
        }
    }

    Ok(())
}
mod lexer;
mod parser;
use lexer::Lexer;
use parser::{Parser, Value};
mod template;
use std::{collections::HashMap, env, fs::read_to_string, io::{self, stdin, Read}, path::PathBuf};
use template::{augment, Environment};

fn parse_value(value: &str) -> Value {
    // eprintln!("{value}");
    if value.starts_with('"') && value.ends_with('"') {
        let mut string = value.chars();
        string.next();
        string.next_back();
        return Value::String(string.as_str().into());
    } else if value.starts_with('[') && value.ends_with(']') {
        let mut inner = value.chars().peekable();
        inner.next();
        inner.next_back();
        return parse_array(&mut inner);
    } else if value.is_empty() {
        return Value::Null;
    } else if value.starts_with(char::is_numeric) {
        let number = value.parse().expect("Failed to parse number");
        return Value::Number(number);
    } else {
        match value {
            "true" => return Value::Boolean(true),
            "false" => return Value::Boolean(false),
            string => return Value::String(string.into()),
        }
    }
}

fn parse_array(inner: &mut impl Iterator<Item = char>) -> Value {
    let mut vec = Vec::new();
    let mut value = String::new(); // can't use take_while() because it will consume the last character
    while let Some(char) = inner.next() {
        if char == '[' {
            value.clear();
            vec.push(parse_array(inner));
            continue;
        }
        if char == ']' {
            if !value.is_empty() {
                vec.push(parse_value(&value));
                value.clear();
            }
            break;
        }
        if char == ',' {
            if !value.is_empty() {
                vec.push(parse_value(&value));
                value.clear();
            }
            continue;
        }
        if char.is_whitespace() && value.is_empty() {
            continue;
        }
        value.push(char);
    }
    return Value::Array(vec.into());
}

fn parse_argument(param: String) -> (String, Value) {
    let Some((ident, value)) = param.split_once('=') else {
        panic!("Expected equals sign in parameter specification. Example: username=\"John\"")
    };
    let value = value.trim();
    let value = parse_value(value);
    (ident.to_owned(), value)
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

/// returns (the file templated, the base template that this one extends from)
fn template_a_file(contents: String, environment: &mut Environment) -> (String, Option<PathBuf>) {
    // use std::time::Instant;
    // let before = Instant::now();
    let mut lexer = Lexer::new(contents.chars());
    let result = lexer.execute();
    // println!("{:?}", Instant::now() - before);

    let parser = Parser::new();
    let (result, base_template) = parser.execute(result);

    let mut it = result.into_iter();
    let result = augment(&mut it, environment);

    (result, base_template)
}

fn main() -> io::Result<()> {
    let mut arguments = env::args().peekable();
    arguments.next();

    let mut environment = HashMap::new();
    environment.insert("slot".to_owned(), Value::String("".into()));

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
        if argument == "-i" {
            environment = arguments.map(parse_argument).collect();
        }
    }

    let mut to_be_templated = contents;
    loop {
        let (result, base_template) = template_a_file(to_be_templated, &mut environment);
        if let Some(path) = base_template {
            to_be_templated = read_to_string(path).unwrap();
            environment.insert("slot".to_owned(), Value::String(result.into()));
        } else {
            println!("{result}");
            break;
        }
    }

    Ok(())
}
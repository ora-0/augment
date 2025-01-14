use crate::lexer::{DocumentKind, Token};
use std::{mem, rc::Rc};

pub(crate) type Contents = Vec<Content>;

#[derive(Debug, Clone)]
pub(crate) enum Content {
    Markup(String),
    Expression(Expr),
    Keys(Vec<String>),
    Block { kind: Block, body: Contents },
}

#[derive(Debug, Clone)]
pub(crate) enum Block {
    If {
        condition: Expr,
    },
    ElseIf {
        condition: Expr,
    },
    Else,
    For {
        // has to be an identifier
        element: Value,
        iterable: Value,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum Expr {
    BinaryOp {
        kind: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    UnaryOp {
        kind: UnaryOp,
        value: Box<Expr>,
    },
    Function {
        ident: String,
        arguments: Vec<Expr>,
    },
    Value(Value),
}

#[derive(Debug)]
pub(crate) enum Value {
    Boolean(bool),
    Number(f32),
    String(Rc<str>),
    Variable(String),
    Array(Rc<[Value]>), // this is only possible via the environment
    Null,
}

impl Clone for Value {
    // I'm not sure if I can just derive this and it will automatically
    // use the rc implementation for clone. (pretty sure it will)
    // but this makes it more explicit that the clones aren't heavy
    /// light clone
    fn clone(&self) -> Self {
        match self {
            Self::Boolean(bool) => Self::Boolean(*bool),
            Self::Number(num) => Self::Number(*num),
            Self::String(contents) => Self::String(Rc::clone(contents)),
            Self::Variable(ident) => Self::Variable(ident.clone()),
            Self::Array(vec) => Self::Array(Rc::clone(vec)),
            Self::Null => Self::Null,
        }
    }
}

impl Value {
    pub(crate) fn unwrap_boolean(self) -> bool {
        if let Self::Boolean(content) = self {
            return content;
        }
        panic!("Expected boolean, got {:?}", self);
    }

    #[allow(unused)]
    pub(crate) fn unwrap_string(self) -> Rc<str> {
        if let Self::String(content) = self {
            return content;
        }
        panic!("Expected string, got {:?}", self);
    }

    pub(crate) fn unwrap_number(self) -> f32 {
        if let Self::Number(content) = self {
            return content;
        }
        panic!("Expected number, got {:?}", self);
    }

    pub(crate) fn unwrap_array(self) -> Rc<[Value]> {
        if let Self::Array(content) = self {
            return content;
        }
        panic!("Expected array, got {:?}", self);
    }

    pub(crate) fn clone_to_string(self) -> String {
        match self {
            Value::Boolean(bool) => bool.to_string(),
            Value::Number(num) => num.to_string(),
            Value::String(content) => content.to_string(),
            Value::Null => "null".to_owned(),
            Value::Variable(_) => panic!(),
            Value::Array(_) => panic!("Cannot convert array to string"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanOrEquals,
    LessThan,
    LessThanOrEquals,
    And,
    Or,
    Concat,
    Index,
}

#[derive(Debug, Clone)]
pub(crate) enum UnaryOp {
    Dummy, // Brackets
    Not,
    Negate,
}

pub(crate) trait Operation {
    fn takes_in_booleans(&self) -> bool;
    fn takes_in_strings(&self) -> bool;
    fn takes_in_numbers(&self) -> bool;
}

impl Operation for BinaryOp {
    fn takes_in_booleans(&self) -> bool {
        use BinaryOp::*;
        matches!(self, And | Or)
    }

    fn takes_in_strings(&self) -> bool {
        use BinaryOp::*;
        matches!(self, Concat)
    }

    fn takes_in_numbers(&self) -> bool {
        use BinaryOp::*;
        matches!(
            self,
            Add | Subtract
                | Multiply
                | Divide
                | Modulo
                | Equals
                | NotEquals
                | GreaterThan
                | GreaterThanOrEquals
                | LessThan
                | LessThanOrEquals
        )
    }
}

impl Operation for UnaryOp {
    fn takes_in_booleans(&self) -> bool {
        use UnaryOp::*;
        matches!(self, Not | Dummy)
    }

    fn takes_in_strings(&self) -> bool {
        use UnaryOp::*;
        matches!(self, Dummy)
    }

    fn takes_in_numbers(&self) -> bool {
        use UnaryOp::*;
        matches!(self, Negate | Dummy)
    }
}

pub(crate) struct Parser {
    template: Vec<Token>,
    ast: Contents,
    current: usize,
    nesting_path: Vec<usize>,
}

impl Parser {
    pub(crate) fn new() -> Self {
        Parser {
            template: Vec::new(),
            ast: Vec::new(),
            current: 0,
            nesting_path: Vec::new(),
        }
    }

    fn next_if(&mut self, token: Token) -> bool {
        let Some(current) = self.template.get(self.current) else {
            return false;
        };

        // compares enums without comparing the insides.
        let equals = mem::discriminant(&token) == mem::discriminant(current);
        if equals {
            self.current += 1;
        }
        return equals;
    }

    fn expect(&mut self, token: Token) -> Result<(), ()> {
        if self.next_if(token) {
            return Ok(());
        }
        Err(())
    }

    fn peek(&self) -> Option<&Token> {
        self.template.get(self.current)
    }

    fn next_and_take(&mut self) -> Option<Token> {
        let current = self.template.get_mut(self.current)?;
        let token = mem::replace(current, Token::CParen);
        self.current += 1;
        Some(token)
    }

    fn get_last_block_added(&mut self) -> &mut Contents {
        let mut last = &mut self.ast;
        for &index in &self.nesting_path {
            let Content::Block { ref mut body, .. } = last[index] else {
                unreachable!()
            };
            last = body;
        }
        last
    }

    // modifies the nesting_path to point to the new block
    fn increase_nesting(&mut self) {
        let length = self.get_last_block_added().len() - 1;
        self.nesting_path.push(length);
    }

    fn decrease_nesting(&mut self) {
        self.nesting_path.pop();
    }

    fn parse_identifier(&mut self, ident: String) -> Expr {
        // function call
        if self.next_if(Token::OParen) {
            let mut arguments = Vec::new();
            loop {
                let argument = self.parse_expression();
                arguments.push(argument);
                if self.next_if(Token::Comma) {
                    continue;
                } else {
                    break;
                }
            }
            self.expect(Token::CParen).expect("Missing closing paren");
            return Expr::Function { ident, arguments };
        }

        if self.next_if(Token::OBracket) {
            let index = self.parse_expression();
            self.expect(Token::CBracket)
                .expect("Missing closing bracket");
            let mut indexing_onion = Expr::BinaryOp {
                kind: BinaryOp::Index,
                lhs: Box::new(Expr::Value(Value::Variable(ident))),
                rhs: Box::new(index),
            };
            while self.next_if(Token::OBracket) {
                let index = self.parse_expression();
                self.expect(Token::CBracket)
                    .expect("Missing closing bracket");
                indexing_onion = Expr::BinaryOp {
                    kind: BinaryOp::Index,
                    lhs: Box::new(indexing_onion),
                    rhs: Box::new(index),
                };
            }
            return indexing_onion;
        }

        return Expr::Value(Value::Variable(ident));
    }

    fn parse_factor(&mut self) -> Expr {
        if self.next_if(Token::Minus) {
            return Expr::UnaryOp {
                kind: UnaryOp::Negate,
                value: Box::new(self.parse_factor()),
            };
        }
        if self.next_if(Token::Not) {
            return Expr::UnaryOp {
                kind: UnaryOp::Not,
                value: Box::new(self.parse_factor()),
            };
        }
        if self.next_if(Token::OParen) {
            let inside = self.parse_logical();
            self.expect(Token::CParen).expect("Expected '('");
            return Expr::UnaryOp {
                kind: UnaryOp::Dummy,
                value: Box::new(inside),
            };
        }

        match self.next_and_take() {
            Some(Token::Ident(ident)) => return self.parse_identifier(ident),
            Some(Token::String(content)) => return Expr::Value(Value::String(content.into())),
            Some(Token::Boolean(bool)) => return Expr::Value(Value::Boolean(bool)),
            Some(Token::Number(num)) => return Expr::Value(Value::Number(num)),
            Some(_) => unreachable!(),
            None => panic!("Expected a value"),
        }
    }

    fn parse_term(&mut self) -> Expr {
        let lhs = self.parse_factor();

        let kind = if self.next_if(Token::Asterisk) {
            BinaryOp::Multiply
        } else if self.next_if(Token::Slash) {
            BinaryOp::Divide
        } else if self.next_if(Token::Percent) {
            BinaryOp::Modulo
        } else {
            return lhs;
        };

        let rhs = self.parse_term();
        return Expr::BinaryOp {
            kind,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
    }

    fn parse_expression(&mut self) -> Expr {
        let lhs = self.parse_term();

        let kind = if self.next_if(Token::Plus) {
            BinaryOp::Add
        } else if self.next_if(Token::Minus) {
            BinaryOp::Subtract
        } else if self.next_if(Token::Concat) {
            // :P
            BinaryOp::Concat
        } else {
            return lhs;
        };

        let rhs = self.parse_expression();
        return Expr::BinaryOp {
            kind,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
    }

    fn parse_condition(&mut self) -> Expr {
        let lhs = self.parse_expression();
        let kind = match self.peek() {
            Some(Token::Equals) => BinaryOp::Equals,
            Some(Token::NotEquals) => BinaryOp::NotEquals,
            Some(Token::GreaterThan) => BinaryOp::GreaterThan,
            Some(Token::GreaterThanOrEquals) => BinaryOp::GreaterThanOrEquals,
            Some(Token::LessThan) => BinaryOp::LessThan,
            Some(Token::LessThanOrEquals) => BinaryOp::LessThanOrEquals,
            _ => return lhs,
        };
        self.current += 1;

        let rhs = self.parse_expression();
        Expr::BinaryOp {
            kind,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    fn parse_logical(&mut self) -> Expr {
        let lhs = self.parse_condition();

        let kind = if self.next_if(Token::And) {
            BinaryOp::And
        } else if self.next_if(Token::Bar) {
            BinaryOp::Or
        } else {
            return lhs;
        };

        let rhs = self.parse_logical();
        Expr::BinaryOp {
            kind,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    fn parse_block_declaration(&mut self) {
        let declaration = if self.next_if(Token::If) {
            Content::Block {
                kind: Block::If {
                    condition: self.parse_logical(),
                },
                body: Vec::new(),
            }
        } else if self.next_if(Token::For) {
            // NOTE: the self.expect function only compares the enum variant, and not the insides.
            let Some(Token::Ident(element_ident)) = self.next_and_take() else {
                panic!("Expected Identifier");
            };
            self.expect(Token::In).expect("Expected in keyword");
            let Some(Token::Ident(iterable_ident)) = self.next_and_take() else {
                panic!("Expected Identifier");
            };

            Content::Block {
                kind: Block::For {
                    element: Value::Variable(element_ident),
                    iterable: Value::Variable(iterable_ident),
                },
                body: Vec::new(),
            }
        } else {
            panic!("Expected if or for");
        };
        self.get_last_block_added().push(declaration);
        self.increase_nesting();
    }

    fn parse_else_declaration(&mut self) {
        self.decrease_nesting();
        self.expect(Token::Else).expect("Expected else statement");

        let declaration = if self.next_if(Token::If) {
            Content::Block {
                kind: Block::ElseIf {
                    condition: self.parse_logical(),
                },
                body: Vec::new(),
            }
        } else {
            Content::Block {
                kind: Block::Else,
                body: Vec::new(),
            }
        };

        self.get_last_block_added().push(declaration);
        self.increase_nesting();
    }

    fn parse_statement(&mut self) {
        self.expect(Token::Keys).expect("Expected keyword keys");
        let mut idents = Vec::new();
        while let Some(ident) = self.next_and_take() {
            let Token::Ident(ident) = ident else {
                panic!("Expected identifier, found {:?}", ident);
            };
            idents.push(ident)
        }
        self.get_last_block_added().push(Content::Keys(idents));
    }

    fn parse_template(&mut self) {
        if self.next_if(Token::Hashtag) {
            self.parse_block_declaration();
        } else if self.next_if(Token::Colon) {
            self.parse_else_declaration();
        } else if self.next_if(Token::Slash) {
            self.decrease_nesting();
        } else if self.next_if(Token::At) {
            self.parse_statement();
        } else {
            let expr = Content::Expression(self.parse_logical());
            self.get_last_block_added().push(expr);
        }
    }

    pub(crate) fn execute(mut self, content: Vec<DocumentKind>) -> Contents {
        content.into_iter().for_each(|thing| {
            if let DocumentKind::Markup(text) = thing {
                self.get_last_block_added().push(Content::Markup(text));
                return;
            }

            if let DocumentKind::Template(template) = thing {
                self.template = template;
                self.current = 0;
                self.parse_template();
                return;
            }
        });

        self.ast
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_binary_op() {

    }
}
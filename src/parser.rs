use crate::lexer::{DocumentKind, Template, Token};
use crate::arena::{Arena, ArenaBox, ArenaVec};
use std::{mem, path::PathBuf, fmt::Write};

#[derive(Debug)]
pub(crate) enum Content<'a> {
    Markup(&'a str),
    Expression(ExprRef<'a>),
    Keys(ArenaVec<'a, &'a str>),
    Block { kind: Block<'a> },
    EndBlock,
}

#[derive(Debug)]
pub(crate) enum Block<'a> {
    If {
        condition: ExprRef<'a>,
    },
    ElseIf {
        condition: ExprRef<'a>,
    },
    Else,
    For {
        //  to be an identifier
        element: Value<'a>,
        iterable: Value<'a>,
    },
}

pub type ExprRef<'a> = ArenaBox<'a, Expr<'a>>;

#[derive(Debug)]
pub(crate) enum Expr<'a> {
    BinaryOp {
        kind: BinaryOp,
        lhs: ExprRef<'a>,
        rhs: ExprRef<'a>,
    },
    UnaryOp {
        kind: UnaryOp,
        value: ExprRef<'a>,
    },
    Function {
        ident: &'a str,
        arguments: ArenaVec<'a, ExprRef<'a>>,
    },
    Value(Value<'a>),
}

#[derive(Debug, Clone)]
pub(crate) enum Value<'a> {
    Boolean(bool),
    Number(f32),
    String(&'a str),
    VarRef(&'a str),
    Array(&'a [Value<'a>]), // this is only possible via the environment
    Null,
}

impl<'a> Value<'a> {
    pub(crate) fn unwrap_boolean(self) -> bool {
        if let Self::Boolean(content) = self {
            return content;
        }
        panic!("Expected boolean, got {:?}", self);
    }

    #[allow(unused)]
    pub(crate) fn unwrap_string(self) -> &'a str {
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

    pub(crate) fn unwrap_array(self) -> &'a [Value<'a>] {
        if let Self::Array(content) = self {
            return content;
        }
        panic!("Expected array, got {:?}", self);
    }

    #[allow(unused)]
    pub(crate) fn clone_to_string(self) -> String {
        match self {
            Value::Boolean(bool) => bool.to_string(),
            Value::Number(num) => num.to_string(),
            Value::String(content) => content.to_string(),
            Value::Null => "null".to_owned(),
            Value::VarRef(_) => panic!(),
            Value::Array(_) => panic!("Cannot convert array to string"),
        }
    }

    pub(crate) fn write_to(self, buf: &mut String) {
        match self {
            Value::Boolean(bool) => write!(buf, "{bool}").unwrap(),
            Value::Number(num) => write!(buf, "{num}").unwrap(),
            Value::String(content) => buf.push_str(&content),
            Value::Null => buf.push_str("null"),
            Value::VarRef(_) => panic!(),
            Value::Array(_) => panic!("Cannot convert array to string"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
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

pub(crate) struct Parser<'a> {
    template: Template<'a>,
    ast: Vec<Content<'a>>,
    current: usize,
    base_template: Option<PathBuf>,
    arena: &'a Arena<'a>,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(arena: &'a Arena) -> Self {
        Parser {
            template: &[],
            ast: Vec::new(),
            current: 0,
            base_template: None,
            arena: arena,
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

    fn peek(&self) -> Option<&Token<'_>> {
        self.template.get(self.current)
    }

    fn next(&mut self) -> Option<Token<'a>> {
        let current = self.template.get(self.current).map(|x| x.clone())?;
        self.current += 1;
        Some(current)
    }

    fn parse_identifier(&mut self, ident: &'a str) -> ExprRef<'a> {
        // function call
        if self.next_if(Token::OParen) {
            let mut arguments = ArenaVec::new(self.arena);
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

            return ArenaBox::new(self.arena, Expr::Function { ident, arguments });
        }

        if self.next_if(Token::OBracket) {
            let index = self.parse_expression();
            self.expect(Token::CBracket)
                .expect("Missing closing bracket");
            let mut indexing_onion = Expr::BinaryOp {
                kind: BinaryOp::Index,
                lhs: ArenaBox::new(self.arena, Expr::Value(Value::VarRef(ident))),
                rhs: index,
            };
            while self.next_if(Token::OBracket) {
                let index = self.parse_expression();
                self.expect(Token::CBracket)
                    .expect("Missing closing bracket");
                indexing_onion = Expr::BinaryOp {
                    kind: BinaryOp::Index,
                    lhs: ArenaBox::new(self.arena, indexing_onion),
                    rhs: index,
                };
            }
            return ArenaBox::new(self.arena, indexing_onion);
        }

        return ArenaBox::new(self.arena, Expr::Value(Value::VarRef(ident)));
    }

    fn parse_factor(&mut self) -> ExprRef<'a> {
        if self.next_if(Token::Minus) {
            return ArenaBox::new(self.arena, Expr::UnaryOp {
                kind: UnaryOp::Negate,
                value: self.parse_factor(),
            });
        }
        if self.next_if(Token::Not) {
            return ArenaBox::new(self.arena, Expr::UnaryOp {
                kind: UnaryOp::Not,
                value: self.parse_factor(),
            });
        }
        if self.next_if(Token::OParen) {
            let inside = self.parse_logical();
            self.expect(Token::CParen).expect("Expected '('");
            return ArenaBox::new(self.arena, Expr::UnaryOp {
                kind: UnaryOp::Dummy,
                value: inside,
            });
        }

        let val = match self.next() {
            Some(Token::Ident(ident)) => return self.parse_identifier(ident),
            Some(Token::String(content)) => Value::String(content),
            Some(Token::Boolean(bool)) => Value::Boolean(bool),
            Some(Token::Number(num)) => Value::Number(num),
            Some(_) => unreachable!(),
            None => panic!("Expected a value"),
        };

        ArenaBox::new(self.arena, Expr::Value(val))
    }

    fn parse_term(&mut self) -> ExprRef<'a> {
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
        ArenaBox::new(self.arena, Expr::BinaryOp {
            kind,
            lhs,
            rhs,
        })
    }

    fn parse_expression(&mut self) -> ExprRef<'a> {
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
        ArenaBox::new(self.arena, Expr::BinaryOp {
            kind,
            lhs: lhs,
            rhs: rhs,
        })
    }

    fn parse_condition(&mut self) -> ExprRef<'a> {
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
        ArenaBox::new(self.arena, Expr::BinaryOp {
            kind,
            lhs,
            rhs,
        })
    }

    fn parse_logical(&mut self) -> ExprRef<'a> {
        let lhs = self.parse_condition();

        let kind = if self.next_if(Token::And) {
            BinaryOp::And
        } else if self.next_if(Token::Bar) {
            BinaryOp::Or
        } else {
            return lhs;
        };

        let rhs = self.parse_logical();
        ArenaBox::new(self.arena, Expr::BinaryOp {
            kind,
            lhs,
            rhs,
        })
    }

    fn parse_block_declaration(&mut self) {
        let declaration = if self.next_if(Token::If) {
            Content::Block {
                kind: Block::If {
                    condition: self.parse_logical(),
                },
            }
        } else if self.next_if(Token::For) {
            // NOTE: the self.expect function only compares the enum variant, and not the insides.
            let Some(Token::Ident(element_ident)) = self.next() else {
                panic!("Expected Identifier");
            };
            self.expect(Token::In).expect("Expected in keyword");
            let Some(Token::Ident(iterable_ident)) = self.next() else {
                panic!("Expected Identifier");
            };

            Content::Block {
                kind: Block::For {
                    element: Value::VarRef(element_ident),
                    iterable: Value::VarRef(iterable_ident),
                },
            }
        } else {
            panic!("Expected if or for");
        };
        self.ast.push(declaration);
    }

    fn parse_else_declaration(&mut self) {
        self.ast.push(Content::EndBlock);
        self.expect(Token::Else).expect("Expected else statement");

        let declaration = if self.next_if(Token::If) {
            Content::Block {
                kind: Block::ElseIf {
                    condition: self.parse_logical(),
                },
            }
        } else {
            Content::Block { kind: Block::Else }
        };

        self.ast.push(declaration);
    }

    fn parse_statement(&mut self) {
        if self.next_if(Token::Keys) {
            let mut idents = ArenaVec::new(self.arena);
            while let Some(ident) = self.next() {
                let Token::Ident(ident) = ident else {
                    panic!("Expected identifier, found {:?}", ident);
                };
                idents.push(ident)
            }
            self.ast.push(Content::Keys(idents));
        } else if self.next_if(Token::Base) {
            if self.base_template.is_some() {
                panic!("There may only be one @base statement per file")
            }

            let Expr::Value(Value::String(ref path)) = *self.parse_expression() else {
                panic!("@base statement needs to take in a string as argument. For example `@base \"./file.html\"");
            };

            self.base_template = Some(PathBuf::from(path));
        }
    }

    fn parse_template(&mut self) {
        if self.next_if(Token::Hashtag) {
            self.parse_block_declaration();
        } else if self.next_if(Token::Colon) {
            self.parse_else_declaration();
        } else if self.next_if(Token::Slash) {
            self.ast.push(Content::EndBlock);
        } else if self.next_if(Token::At) {
            self.parse_statement();
        } else {
            let expr = Content::Expression(self.parse_logical());
            self.ast.push(expr);
        }
    }

    pub(crate) fn execute(mut self, content: Vec<DocumentKind<'a>>) -> (Vec<Content<'a>>, Option<PathBuf>) {
        content.into_iter().for_each(|thing| {
            if let DocumentKind::Markup(text) = thing {
                self.ast.push(Content::Markup(text));
                return;
            }

            if let DocumentKind::Template(template) = thing {
                self.template = template;
                self.current = 0;
                self.parse_template();
                return;
            }
        });

        (self.ast, self.base_template)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_binary_op() {

    }
}
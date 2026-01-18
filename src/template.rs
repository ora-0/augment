use crate::parser::*;
use std::{collections::HashMap, slice};

pub(crate) type Environment<'a> = HashMap<&'a str, Value<'a>>;

fn evaluate_arithmetic<'a>(kind: BinaryOp, lhs: &Expr<'a>, rhs: &Expr<'a>, env: &Environment<'a>) -> Value<'a> {
    use BinaryOp as Op;
    use Value::*;
    let a = evaluate_expression(lhs, env).unwrap_number();
    let b = evaluate_expression(rhs, env).unwrap_number();
    match kind {
        Op::Add => Number(a + b),
        Op::Subtract => Number(a - b),
        Op::Multiply => Number(a * b),
        Op::Divide => Number(a / b),
        Op::Modulo => Number(a % b),
        Op::Equals => Boolean(a == b),
        Op::NotEquals => Boolean(a != b),
        Op::GreaterThan => Boolean(a > b),
        Op::GreaterThanOrEquals => Boolean(a >= b),
        Op::LessThan => Boolean(a < b),
        Op::LessThanOrEquals => Boolean(a <= b),
        _ => unreachable!(),
    }
}

fn evaluate_logic<'a>(kind: BinaryOp, lhs: &Expr<'a>, rhs: &Expr<'a>, env: &Environment<'a>) -> Value<'a> {
    use BinaryOp as Op;
    use Value::*;
    let a = evaluate_expression(lhs, env).unwrap_boolean();
    let b = evaluate_expression(rhs, env).unwrap_boolean();
    match kind {
        Op::And => Boolean(a && b),
        Op::Or => Boolean(a || b),
        _ => unreachable!(),
    }
}

#[allow(unused)]
fn evaluate_concat<'a>(kind: BinaryOp, lhs: &Expr<'a>, rhs: &Expr<'a>, env: &Environment<'a>) -> Value<'a> {
    use BinaryOp as Op;
    use Value::*;
    // let a = evaluate_expression(lhs, env).clone_to_string();
    // let b = evaluate_expression(rhs, env).clone_to_string();
    // match kind {
    //     Op::Concat => String(&(a + &b)),
    //     _ => unreachable!(),
    // }
    unimplemented!()
}

fn evaluate_index<'a>(lhs: &Expr<'a>, rhs: &Expr<'a>, env: &Environment<'a>) -> Value<'a> {
    let list = evaluate_expression(lhs, env).unwrap_array();
    let index = evaluate_expression(rhs, env).unwrap_number();
    if index.is_sign_negative() {
        panic!("Cannot have negative index");
    }
    list[index.trunc() as usize].clone()
}

fn evaluate_binary_op<'a>(kind: BinaryOp, lhs: &Expr<'a>, rhs: &Expr<'a>, env: &Environment<'a>) -> Value<'a> {
    if kind.takes_in_numbers() {
        return evaluate_arithmetic(kind, lhs, rhs, env);
    }
    if kind.takes_in_booleans() {
        return evaluate_logic(kind, lhs, rhs, env);
    }
    if kind.takes_in_strings() {
        return evaluate_concat(kind, lhs, rhs, env);
    }
    if let BinaryOp::Index = kind {
        return evaluate_index(lhs, rhs, env);
    }
    unreachable!()
}

fn evaluate_unary_op<'a>(kind: UnaryOp, value: &Expr<'a>, env: &Environment<'a>) -> Value<'a> {
    use UnaryOp::*;
    match kind {
        Dummy => return evaluate_expression(value, env),
        Not => {
            let Value::Number(num) = evaluate_expression(value, env) else {
                panic!("Cannot not non booleans");
            };
            return Value::Number(-num);
        }
        Negate => {
            let Value::Number(num) = evaluate_expression(value, env) else {
                panic!("Cannot negate non numbers");
            };
            return Value::Number(-num);
        }
    }
}

fn evaluate_function_call<'a>(ident: &str, args: &[ExprRef<'a>], env: &Environment<'a>) -> Value<'a> {
    // currently not very scalable
    // I'm planning to make a function struct and store them thereree
    match ident {
        "len" => {
            // make this better later
            assert_eq!(args.len(), 1);
            let mut args = args.iter().map(|arg| evaluate_expression(arg, env));
            if let Some(Value::Array(array)) = args.next() {
                return Value::Number(array.len() as f32);
            } else {
                panic!();
            }
        }
        _ => panic!("Unrecognized function: {ident}"),
    }
}

fn evaluate_expression<'a>(expr: &Expr<'a>, env: &Environment<'a>) -> Value<'a> {
    match expr {
        Expr::BinaryOp { kind, lhs, rhs } => evaluate_binary_op(*kind, lhs, rhs, env),
        Expr::UnaryOp { kind, value } => evaluate_unary_op(*kind, value, env),
        Expr::Value(Value::VarRef(ident)) => env.get(*ident).unwrap_or(&Value::Null).to_owned(),
        Expr::Value(value) => value.to_owned(),
        Expr::Function { ident, arguments } => evaluate_function_call(ident, arguments.as_ref(), env),
    }
}

pub struct Augment<'a, 'b> {
    iter: slice::Iter<'b, Content<'a>>,
    result_buf: String,
    env: &'b mut Environment<'a>,
    last_condition_is_true: bool,
}

impl<'a, 'b> Augment<'a, 'b> {
    pub fn new(iter: slice::Iter<'b, Content<'a>>, env: &'b mut Environment<'a>) -> Self {
        Self {
            iter,
            result_buf: String::with_capacity(2048),
            env,
            last_condition_is_true: false
        }
    }

    pub fn execute(mut self) -> String {
        self.augment();
        return self.result_buf
    }

    fn augment(&mut self) {
        use crate::parser::Block::*;
        use crate::parser::Content::*;
        while let Some(next) = self.iter.next() {
            match next {
                Markup(content) => self.result_buf.push_str(content),

                Block { kind: block @ (Else | If {..} | ElseIf {..}) } => self.augment_if(block),

                Block { kind: For { element, iterable } } => self.augment_for(element, iterable),
                EndBlock => return,

                Expression(expr) => evaluate_expression(expr, self.env).write_to(&mut self.result_buf),

                Keys(idents) => {
                    idents.iter().for_each(|a| println!("ident: {a}"));
                    idents.iter().enumerate().for_each(|(i, ident)| {
                        self.env.insert(ident, Value::Number(i as f32));
                    });
                }
            }
        }
    }

    fn augment_if(&mut self, block: &Block<'a>) {
        // if i was bothered i would clean this up
        match block {
            Block::If { condition } => {
                let condition = evaluate_expression(condition, self.env).unwrap_boolean();
                self.last_condition_is_true = false;
                if condition {
                    self.augment();
                    self.last_condition_is_true = true;
                }
            }
            Block::ElseIf { .. } if self.last_condition_is_true => (),
            Block::ElseIf { condition } => {
                let condition = evaluate_expression(condition, self.env).unwrap_boolean();
                if condition {
                    self.augment();
                    self.last_condition_is_true = true;
                }
            }

            Block::Else if self.last_condition_is_true => (),
            Block::Else => {
                self.augment();
            }

            Block::For { .. } => unreachable!(),
        }
    }

    fn augment_for(&mut self, element: &Value<'a>, iterable: &Value<'a>) {
        let body = self.iter.clone();

        let Value::VarRef(iteration_var) = element else { unreachable!() };
        if self.env.contains_key(*iteration_var) {
            panic!("Cannot iterate with variable {iteration_var} because it has already been defined");
        }

        let Value::VarRef(iter_ident) = iterable else { unreachable!() };
        let iterable = self.env.get(*iter_ident).unwrap_or_else(|| {
            panic!("Cannot iterate with variable {iter_ident} because it has not been defined");
        });
        let Value::Array(ref array) = iterable.clone() else {
            panic!("Cannot iterate with variable {iter_ident} because it is not an array",);
        };

        self.env.insert(iteration_var, Value::Null);
        array
            .iter()
            .map(|value| {
                *self.env.get_mut(*iteration_var).unwrap() = value.clone();
                self.iter = body.clone();
                self.augment()
            })
            .collect()
    }
}

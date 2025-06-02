use crate::parser::*;
use std::{borrow::Cow, collections::HashMap, vec::IntoIter};

pub(crate) type Environment = HashMap<String, Value>;

fn evaluate_arithmetic(kind: BinaryOp, lhs: Expr, rhs: Expr, env: &Environment) -> Value {
    use BinaryOp as Op;
    use Value::*;
    let a = evaluate_expression(lhs, env).unwrap_number();
    let b = evaluate_expression(rhs, env).unwrap_number();
    return match kind {
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
    };
}

fn evaluate_logic(kind: BinaryOp, lhs: Expr, rhs: Expr, env: &Environment) -> Value {
    use BinaryOp as Op;
    use Value::*;
    let a = evaluate_expression(lhs, env).unwrap_boolean();
    let b = evaluate_expression(rhs, env).unwrap_boolean();
    return match kind {
        Op::And => Boolean(a && b),
        Op::Or => Boolean(a || b),
        _ => unreachable!(),
    };
}

fn evaluate_concat(kind: BinaryOp, lhs: Expr, rhs: Expr, env: &Environment) -> Value {
    use BinaryOp as Op;
    use Value::*;
    let a = evaluate_expression(lhs, env).clone_to_string();
    let b = evaluate_expression(rhs, env).clone_to_string();
    return match kind {
        Op::Concat => String((a + &b).into()),
        _ => unreachable!(),
    };
}

fn evaluate_index(lhs: Expr, rhs: Expr, env: &Environment) -> Value {
    let list = evaluate_expression(lhs, env).unwrap_array();
    let index = evaluate_expression(rhs, env).unwrap_number();
    if index.is_sign_negative() {
        panic!("Cannot have negative index");
    }
    return list[index.trunc() as usize].clone();
}

fn evaluate_binary_op(kind: BinaryOp, lhs: Expr, rhs: Expr, env: &Environment) -> Value {
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

fn evaluate_unary_op(kind: UnaryOp, value: Expr, env: &Environment) -> Value {
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

fn evaluate_function_call(ident: String, args: Vec<Expr>, env: &Environment) -> Value {
    // currently not very scalable
    // I'm planning to make a function struct and store them thereree
    match ident.as_ref() {
        "len" => {
            // make this better later
            let mut args = args.into_iter().map(|arg| evaluate_expression(arg, env));
            assert_eq!(args.len(), 1);
            if let Some(Value::Array(array)) = args.next() {
                return Value::Number(array.len() as f32);
            } else {
                panic!();
            }
        }
        _ => panic!("Unrecognized function: {ident}"),
    }
}

fn evaluate_expression(expr: Expr, env: &Environment) -> Value {
    match expr {
        Expr::BinaryOp { kind, lhs, rhs } => evaluate_binary_op(kind, *lhs, *rhs, env),
        Expr::UnaryOp { kind, value } => evaluate_unary_op(kind, *value, env),
        Expr::Value(Value::Variable(ident)) => env.get(&ident).unwrap_or(&Value::Null).clone(),
        Expr::Value(value) => value,
        Expr::Function { ident, arguments } => evaluate_function_call(ident, arguments, env),
    }
}

// this
type ContentIter<'a> = IntoIter<Content<'a>>;
pub(crate) fn augment(contents: &mut ContentIter, env: &mut Environment) -> String {
    let mut last_if_state = false;
    let mut templated = String::new();
    while let Some((string, state)) = augment_one(contents, env, last_if_state) {
        templated += string.as_ref();
        last_if_state = state;
    }
    templated
}

fn augment_one<'a>(contents: &mut ContentIter<'a>, env: &mut Environment, last_condition_is_true: bool) -> Option<(Cow<'a, str>, bool)> {
    use crate::parser::Block::*;
    use crate::parser::Content::*;
    let next = contents.next()?;
    match next {
        Markup(content) => Some((content.into(), false)),

        Block { kind: block @ (Else | If { .. } | ElseIf { .. }), } => {
            let (str, last_condition_is_true) = augment_if(contents, block, env, last_condition_is_true);
            Some((str.into(), last_condition_is_true))
        }

        Block { kind: For { element, iterable }, } 
            => Some((augment_for(contents, element, iterable, env).into(), false)),
        EndBlock => None,

        Expression(expr) => Some((evaluate_expression(expr, env).clone_to_string().into(), false)),

        Keys(idents) => {
            idents.into_iter().enumerate().for_each(|(i, ident)| {
                env.insert(ident, Value::Number(i as f32));
            });
            Some(("".into(), false))
        }
    }
}

fn augment_if<'a>(
    body: &mut ContentIter,
    block: Block,
    env: &mut Environment,
    last_condition_is_true: bool,
) -> (String, bool) {
    // if i was bothered i would clean this up
    match block {
        Block::If { condition } => {
            let condition = evaluate_expression(condition, env).unwrap_boolean();
            if condition {
                return (augment(body, env), true);
            }
            return ("".to_owned(), false);
        }
        Block::ElseIf { .. } if last_condition_is_true => return ("".to_owned(), true),
        Block::ElseIf { condition } => {
            let condition = evaluate_expression(condition, env).unwrap_boolean();
            if condition {
                return (augment(body, env).to_owned(), true);
            }
            return ("".to_owned(), false);
        }
        Block::Else if last_condition_is_true => return ("".to_owned(), false),
        Block::Else => return (augment(body, env), false),
        Block::For { .. } => unreachable!(),
    }
}

fn augment_for(body: &mut ContentIter, element: Value, iterable: Value, env: &mut Environment) -> String {
    let Value::Variable(element_ident) = element else {
        unreachable!()
    };
    if env.contains_key(&element_ident) {
        panic!("Cannot iterate with variable {element_ident} because it has already been defined");
    }

    let Value::Variable(iter_ident) = iterable else {
        unreachable!()
    };
    let iterable = env.get(&iter_ident).unwrap_or_else(|| {
        panic!("Cannot iterate with variable {iter_ident} because it has not been defined");
    });
    let Value::Array(ref vector) = iterable.clone() else {
        panic!("Cannot iterate with variable {iter_ident} because it is not an array",);
    };

    env.insert(element_ident.clone(), Value::Null);
    vector
        .iter()
        .map(|value| {
            *env.get_mut(&element_ident).unwrap() = value.clone();
            augment(body, env)
        })
        .collect()
}

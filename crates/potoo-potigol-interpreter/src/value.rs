use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{Expr, FunctionDecl, NodeId, Parameter, TypeDecl};
use crate::environment::EnvironmentRef;

#[derive(Clone)]
pub(crate) enum Value {
    Integer(i64),
    Real(f64),
    BigInteger(String),
    Text(String),
    Character(char),
    Logical(bool),
    Nothing,
    Tuple(Vec<Value>),
    List(Vec<Value>),
    MutableList(Rc<RefCell<Vec<Value>>>),
    Function(Rc<Callable>),
    TypeName(String),
    Object(Rc<RefCell<ObjectValue>>),
    FieldReference(Rc<RefCell<ObjectValue>>, String),
}

#[derive(Clone)]
pub(crate) enum Callable {
    Function {
        declaration: FunctionDecl,
        environment: EnvironmentRef,
        owner: Option<String>,
        receiver: Option<Value>,
    },
    Lambda {
        id: NodeId,
        parameters: Vec<Parameter>,
        body: Expr,
        environment: EnvironmentRef,
    },
    Constructor {
        declaration: TypeDecl,
        environment: EnvironmentRef,
    },
    Member {
        receiver: Value,
        name: String,
    },
    Placeholder {
        expression: Expr,
        environment: EnvironmentRef,
        arity: usize,
    },
    Partial {
        callable: Rc<Callable>,
        arguments: Vec<Value>,
    },
}

pub(crate) struct ObjectValue {
    pub class_name: String,
    pub fields: HashMap<String, Value>,
}

impl Value {
    pub(crate) fn display(&self) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Real(value) => value.to_string(),
            Self::BigInteger(value) | Self::Text(value) => value.clone(),
            Self::Character(value) => value.to_string(),
            Self::Logical(true) => "verdadeiro".to_owned(),
            Self::Logical(false) => "falso".to_owned(),
            Self::Nothing => "nada".to_owned(),
            Self::Tuple(values) => format!(
                "({})",
                values
                    .iter()
                    .map(Self::display)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::List(values) => format_list(values),
            Self::MutableList(values) => format_list(&values.borrow()),
            Self::Function(_) => "<função>".to_owned(),
            Self::TypeName(name) => format!("<tipo {name}>"),
            Self::Object(object) => format!("<{}>", object.borrow().class_name),
            Self::FieldReference(_, _) => "<atributo>".to_owned(),
        }
    }
}

fn format_list(values: &[Value]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(Value::display)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Integer(left), Self::Integer(right)) => left == right,
            (Self::Real(left), Self::Real(right)) => left == right,
            (Self::BigInteger(left), Self::BigInteger(right)) => left == right,
            (Self::Text(left), Self::Text(right)) => left == right,
            (Self::Character(left), Self::Character(right)) => left == right,
            (Self::Logical(left), Self::Logical(right)) => left == right,
            (Self::Nothing, Self::Nothing) => true,
            (Self::Tuple(left), Self::Tuple(right)) | (Self::List(left), Self::List(right)) => {
                left == right
            }
            (Self::MutableList(left), Self::MutableList(right)) => {
                *left.borrow() == *right.borrow()
            }
            (Self::List(left), Self::MutableList(right)) => *left == *right.borrow(),
            (Self::MutableList(left), Self::List(right)) => *left.borrow() == *right,
            (Self::Function(left), Self::Function(right)) => Rc::ptr_eq(left, right),
            (Self::TypeName(left), Self::TypeName(right)) => left == right,
            (Self::Object(left), Self::Object(right)) => Rc::ptr_eq(left, right),
            (Self::FieldReference(left, left_name), Self::FieldReference(right, right_name)) => {
                Rc::ptr_eq(left, right) && left_name == right_name
            }
            _ => false,
        }
    }
}

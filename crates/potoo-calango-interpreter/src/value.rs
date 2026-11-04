use std::cell::RefCell;
use std::rc::Rc;

use crate::ast::Type;

#[derive(Clone, Debug)]
pub enum Value {
    Integer(i64),
    Real(f64),
    Logical(bool),
    Character(u16),
    Text(String),
    Array {
        ty: Type,
        values: Rc<RefCell<Vec<Rc<RefCell<Value>>>>>,
    },
    Unit,
}

impl Value {
    pub(crate) fn display(&self) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Real(value) => value.to_string(),
            Self::Logical(value) => {
                if *value {
                    "verdadeiro".to_owned()
                } else {
                    "falso".to_owned()
                }
            }
            Self::Character(value) => char::from_u32(u32::from(*value))
                .unwrap_or('\u{fffd}')
                .to_string(),
            Self::Text(value) => value.clone(),
            Self::Array { values, .. } => values
                .borrow()
                .iter()
                .map(|value| value.borrow().display())
                .collect::<Vec<_>>()
                .join(", "),
            Self::Unit => String::new(),
        }
    }

    pub(crate) fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    pub(crate) fn as_real(&self) -> Option<f64> {
        match self {
            Self::Integer(value) => Some(*value as f64),
            Self::Real(value) => Some(*value),
            _ => None,
        }
    }

    pub(crate) fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Integer(left), Self::Integer(right)) => left == right,
            (Self::Integer(left), Self::Real(right)) => *left as f64 == *right,
            (Self::Real(left), Self::Integer(right)) => *left == *right as f64,
            (Self::Real(left), Self::Real(right)) => left == right,
            (Self::Logical(left), Self::Logical(right)) => left == right,
            (Self::Character(left), Self::Character(right)) => left == right,
            (Self::Text(left), Self::Text(right)) => left == right,
            (Self::Unit, Self::Unit) => true,
            (Self::Array { values: left, .. }, Self::Array { values: right, .. }) => {
                Rc::ptr_eq(left, right)
            }
            _ => false,
        }
    }
}

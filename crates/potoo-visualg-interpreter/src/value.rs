use std::cell::RefCell;
use std::rc::Rc;

use crate::ast::{ArrayDimension, Type};

#[derive(Clone, Debug)]
pub(crate) enum Value {
    Integer(i64),
    Real(f64),
    Logical(bool),
    Text(String),
    Array(ArrayValue),
    Void,
}

#[derive(Clone, Debug)]
pub(crate) struct ArrayValue {
    pub(crate) dimensions: Vec<ArrayDimension>,
    pub(crate) elements: Vec<Rc<RefCell<Value>>>,
}

impl Value {
    pub(crate) fn default_for(data_type: Type) -> Self {
        match data_type {
            Type::Integer => Self::Integer(0),
            Type::Real => Self::Real(0.0),
            Type::Character => Self::Text(String::new()),
            Type::Logical => Self::Logical(false),
        }
    }

    pub(crate) fn type_name(&self) -> &'static str {
        match self {
            Self::Integer(_) => "inteiro",
            Self::Real(_) => "real",
            Self::Logical(_) => "logico",
            Self::Text(_) => "caractere",
            Self::Array(_) => "vetor",
            Self::Void => "vazio",
        }
    }

    pub(crate) fn to_text(&self) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Real(value) => value.to_string(),
            Self::Logical(true) => "VERDADEIRO".to_owned(),
            Self::Logical(false) => "FALSO".to_owned(),
            Self::Text(value) => value.clone(),
            Self::Array(_) => "[vetor]".to_owned(),
            Self::Void => String::new(),
        }
    }

    pub(crate) fn coerce_to(self, data_type: Type) -> Result<Self, String> {
        match (self, data_type) {
            (value @ Self::Integer(_), Type::Integer)
            | (value @ Self::Real(_), Type::Real)
            | (value @ Self::Logical(_), Type::Logical)
            | (value @ Self::Text(_), Type::Character) => Ok(value),
            (value, expected) => Err(format!(
                "não é possível atribuir {} a uma variável do tipo {}",
                value.type_name(),
                type_name(expected)
            )),
        }
    }
}

pub(crate) fn type_name(data_type: Type) -> &'static str {
    match data_type {
        Type::Integer => "inteiro",
        Type::Real => "real",
        Type::Character => "caractere",
        Type::Logical => "logico",
    }
}

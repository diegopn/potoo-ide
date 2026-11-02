use std::collections::HashMap;

use crate::ast::NodeId;

/// A resolved or inferred Potigol type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ValueType {
    Integer,
    Real,
    BigInteger,
    Text,
    Character,
    Logical,
    Nothing,
    List(Box<ValueType>),
    MutableList(Box<ValueType>),
    Tuple(Vec<ValueType>),
    Function(Vec<ValueType>, Box<ValueType>),
    Named(String),
    Variable(u32),
}

impl ValueType {
    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Integer | Self::Real | Self::BigInteger)
    }
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer => formatter.write_str("Inteiro"),
            Self::Real => formatter.write_str("Real"),
            Self::BigInteger => formatter.write_str("BigInt"),
            Self::Text => formatter.write_str("Texto"),
            Self::Character => formatter.write_str("Caractere"),
            Self::Logical => formatter.write_str("Lógico"),
            Self::Nothing => formatter.write_str("Nada"),
            Self::List(element) => write!(formatter, "Lista[{element}]"),
            Self::MutableList(element) => write!(formatter, "Lista.mutável[{element}]"),
            Self::Tuple(elements) => {
                formatter.write_str("(")?;
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "{element}")?;
                }
                formatter.write_str(")")
            }
            Self::Function(parameters, result) => {
                formatter.write_str("(")?;
                for (index, parameter) in parameters.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "{parameter}")?;
                }
                write!(formatter, ") => {result}")
            }
            Self::Named(name) => formatter.write_str(name),
            Self::Variable(id) => write!(formatter, "'t{id}"),
        }
    }
}

/// Resolved parameter and return types for a function or method.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionSignature {
    pub parameters: Vec<ValueType>,
    pub return_type: ValueType,
}

/// Metadata for a user-defined field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldInfo {
    pub name: String,
    pub value_type: ValueType,
    pub mutable: bool,
    pub constructor_parameter: bool,
}

/// Metadata for a user-defined Potigol type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassInfo {
    pub name: String,
    pub parent: Option<String>,
    pub abstract_type: bool,
    pub fields: Vec<FieldInfo>,
    pub methods: HashMap<String, FunctionSignature>,
}

/// Static analysis output consumed by the evaluator and IDE host.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TypeInfo {
    pub node_types: HashMap<NodeId, ValueType>,
    pub functions: HashMap<String, FunctionSignature>,
    pub classes: HashMap<String, ClassInfo>,
}

impl TypeInfo {
    pub fn type_of(&self, node: NodeId) -> Option<&ValueType> {
        self.node_types.get(&node)
    }
}

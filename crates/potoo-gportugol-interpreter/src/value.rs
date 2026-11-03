use crate::{DataType, Diagnostic, DiagnosticKind, SourceSpan};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Integer(i64),
    Real(f64),
    Character(char),
    CharacterByte(i8),
    Literal(String),
    NullLiteral,
    Logical(bool),
    LogicalNumber(i32),
    DeclaredType {
        data_type: DataType,
        value: Box<Value>,
    },
    Array {
        element: DataType,
        dimensions: Vec<usize>,
        values: Vec<Value>,
    },
}

impl Value {
    pub(crate) fn zero(data_type: DataType) -> Self {
        match data_type {
            DataType::Integer => Self::Integer(0),
            DataType::Real => Self::Real(0.0),
            DataType::Character => Self::Character('\0'),
            DataType::Literal => Self::NullLiteral,
            DataType::Logical => Self::Logical(false),
        }
    }

    pub(crate) fn data_type(&self) -> Option<DataType> {
        match self {
            Self::Integer(_) => Some(DataType::Integer),
            Self::Real(_) => Some(DataType::Real),
            Self::Character(_) => Some(DataType::Character),
            Self::CharacterByte(_) => Some(DataType::Character),
            Self::Literal(_) => Some(DataType::Literal),
            Self::NullLiteral => Some(DataType::Literal),
            Self::Logical(_) | Self::LogicalNumber(_) => Some(DataType::Logical),
            Self::DeclaredType { data_type, .. } => Some(*data_type),
            Self::Array { .. } => None,
        }
    }

    pub(crate) fn truthy(&self) -> bool {
        match self {
            Self::Integer(value) => *value != 0,
            Self::Real(value) => *value != 0.0,
            Self::Character(value) => *value != '\0',
            Self::CharacterByte(value) => *value != 0,
            Self::Literal(value) => !value.is_empty(),
            Self::NullLiteral => false,
            Self::Logical(value) => *value,
            Self::LogicalNumber(value) => *value != 0,
            Self::DeclaredType { value, .. } => value.truthy(),
            Self::Array { .. } => false,
        }
    }

    pub(crate) fn integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(i64::from(*value as i32)),
            Self::Character(value) => Some(*value as i64),
            Self::CharacterByte(value) => Some(i64::from(*value)),
            Self::Logical(value) => Some(i64::from(*value)),
            Self::LogicalNumber(value) => Some(i64::from(*value)),
            Self::DeclaredType { value, .. } => value.integer(),
            Self::Real(value) => Some(*value as i64),
            _ => None,
        }
    }

    pub(crate) fn real(&self) -> Option<f64> {
        match self {
            Self::Integer(value) => Some(*value as f64),
            Self::Real(value) => Some(*value),
            Self::Character(value) => Some(*value as u32 as f64),
            Self::CharacterByte(value) => Some(f64::from(*value)),
            Self::Logical(value) => Some(f64::from(u8::from(*value))),
            Self::LogicalNumber(value) => Some(f64::from(*value)),
            Self::DeclaredType { value, .. } => value.real(),
            _ => None,
        }
    }

    pub(crate) fn retag(self, data_type: DataType) -> Self {
        if self.data_type() == Some(data_type) {
            return self;
        }
        let value = match self {
            Self::DeclaredType {
                data_type: DataType::Real,
                value,
            } if data_type != DataType::Real => {
                let integer = value
                    .real()
                    .filter(|value| value.is_finite())
                    .map(|value| i64::from(value as i32))
                    .unwrap_or(0);
                Box::new(Self::Integer(integer))
            }
            Self::Real(value) if data_type != DataType::Real => {
                let integer = if value.is_finite() {
                    i64::from(value as i32)
                } else {
                    0
                };
                Box::new(Self::Integer(integer))
            }
            Self::DeclaredType { value, .. } => value,
            value => Box::new(value),
        };
        Self::DeclaredType { data_type, value }
    }

    fn raw_text(&self) -> Option<String> {
        Some(match self {
            Self::Integer(value) => (*value as i32).to_string(),
            Self::Real(value) => value.to_string(),
            Self::Character(value) => (*value as u32).to_string(),
            Self::CharacterByte(value) => value.to_string(),
            Self::Literal(value) => value.clone(),
            Self::NullLiteral => String::new(),
            Self::Logical(value) => u8::from(*value).to_string(),
            Self::LogicalNumber(value) => value.to_string(),
            Self::DeclaredType { value, .. } => return value.raw_text(),
            Self::Array { .. } => return None,
        })
    }

    pub(crate) fn convert(self, data_type: DataType, span: SourceSpan) -> Result<Self, Diagnostic> {
        let converted = match data_type {
            DataType::Literal => match self {
                Self::Literal(_)
                | Self::NullLiteral
                | Self::DeclaredType {
                    data_type: DataType::Literal,
                    ..
                } => self,
                _ => return Err(conversion_error(span)),
            },
            DataType::Integer => Self::Integer(i64::from(
                self.integer().ok_or_else(|| conversion_error(span))? as i32,
            )),
            DataType::Real => Self::Real(self.real().ok_or_else(|| conversion_error(span))?),
            DataType::Character => match self {
                Self::Character(_)
                | Self::CharacterByte(_)
                | Self::DeclaredType {
                    data_type: DataType::Character,
                    ..
                } => self,
                value => {
                    let value = value.integer().ok_or_else(|| conversion_error(span))?;
                    let character = u32::try_from(value)
                        .ok()
                        .and_then(char::from_u32)
                        .ok_or_else(|| conversion_error(span))?;
                    Self::Character(character)
                }
            },
            DataType::Logical => Self::Logical(self.truthy()),
        };
        Ok(converted)
    }

    pub(crate) fn output_text(&self) -> Option<String> {
        Some(match self {
            Self::Integer(value) => (*value as i32).to_string(),
            Self::Real(value) => format!("{:.2}", *value as f32),
            Self::Character(value) => value.to_string(),
            Self::CharacterByte(value) if *value < 0 => format!("\\x{:02x}", *value as u8),
            Self::CharacterByte(value) => char::from(*value as u8).to_string(),
            Self::Literal(value) if value.is_empty() => "(nulo)".to_owned(),
            Self::Literal(value) => value.clone(),
            Self::NullLiteral => "(nulo)".to_owned(),
            Self::Logical(true) => "verdadeiro".to_owned(),
            Self::Logical(false) => "falso".to_owned(),
            Self::LogicalNumber(value) if *value != 0 => "verdadeiro".to_owned(),
            Self::LogicalNumber(_) => "falso".to_owned(),
            Self::DeclaredType { data_type, value } => match data_type {
                DataType::Integer => (value.integer()? as i32).to_string(),
                DataType::Real => format!("{:.2}", value.real()? as f32),
                DataType::Character => {
                    let value = value.integer()? as i8;
                    if value < 0 {
                        format!("\\x{:02x}", value as u8)
                    } else {
                        char::from(value as u8).to_string()
                    }
                }
                DataType::Literal => {
                    let value = value.raw_text()?;
                    if value.is_empty() {
                        "(nulo)".to_owned()
                    } else {
                        value
                    }
                }
                DataType::Logical => {
                    if value.integer()? != 0 {
                        "verdadeiro".to_owned()
                    } else {
                        "falso".to_owned()
                    }
                }
            },
            Self::Array { .. } => return None,
        })
    }
}

fn conversion_error(span: SourceSpan) -> Diagnostic {
    Diagnostic::new(
        DiagnosticKind::Runtime,
        "value cannot be converted to the expected type",
        span,
    )
}

use crate::DataType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Primitive(DataType),
    Array {
        element: DataType,
        dimensions: Vec<usize>,
    },
}

impl Type {
    pub fn from_declaration(data_type: DataType, dimensions: &[usize]) -> Self {
        if dimensions.is_empty() {
            Self::Primitive(data_type)
        } else {
            Self::Array {
                element: data_type,
                dimensions: dimensions.to_vec(),
            }
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Primitive(data_type) if is_numeric(*data_type))
    }

    pub fn is_primitive(&self) -> bool {
        matches!(self, Self::Primitive(_))
    }

    pub fn compatible_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Primitive(left), Self::Primitive(right)) => {
                left == right || (is_numeric(*left) && is_numeric(*right))
            }
            (
                Self::Array {
                    element: left_element,
                    dimensions: left_dimensions,
                },
                Self::Array {
                    element: right_element,
                    dimensions: right_dimensions,
                },
            ) => {
                (left_element == right_element
                    || (is_numeric(*left_element) && is_numeric(*right_element)))
                    && left_dimensions == right_dimensions
            }
            _ => false,
        }
    }
}

pub(crate) fn is_numeric(data_type: DataType) -> bool {
    matches!(
        data_type,
        DataType::Integer | DataType::Real | DataType::Character | DataType::Logical
    )
}

pub(crate) fn promoted_numeric_type(left: DataType, right: DataType) -> DataType {
    if left == DataType::Real || right == DataType::Real {
        DataType::Real
    } else {
        DataType::Integer
    }
}

pub(crate) fn promoted_semantic_numeric_type(left: DataType, right: DataType) -> DataType {
    if left == DataType::Real || right == DataType::Real {
        DataType::Real
    } else if left == DataType::Integer || right == DataType::Integer {
        DataType::Integer
    } else if left == DataType::Character || right == DataType::Character {
        DataType::Character
    } else {
        DataType::Logical
    }
}

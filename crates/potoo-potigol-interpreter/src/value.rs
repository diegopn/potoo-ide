use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use crate::ast::{Expr, FunctionBody, FunctionDecl, NodeId, Parameter, TypeDecl};
use crate::environment::EnvironmentRef;

#[derive(Clone)]
pub(crate) enum Value {
    Integer(i32),
    Real(f64),
    BigInteger(String),
    Text(String),
    Character(u16),
    Logical(bool),
    Nothing,
    Option(Option<Box<Value>>),
    Tuple(Vec<Value>),
    List(Vec<Value>),
    MutableList(Rc<RefCell<Vec<Value>>>),
    Function(Rc<Callable>),
    TypeName(String),
    Object(Rc<RefCell<ObjectValue>>),
    JerimumObject(Rc<RefCell<JerimumObject>>),
    FieldReference(Rc<RefCell<ObjectValue>>, String),
}

#[derive(Clone, PartialEq)]
pub(crate) enum JerimumObject {
    Color {
        red: i32,
        green: i32,
        blue: i32,
    },
    AwtColor {
        red: i32,
        green: i32,
        blue: i32,
        alpha: i32,
    },
    Image {
        path: String,
        handle: i64,
        width: i64,
        height: i64,
    },
    ImageBuffer {
        handle: i64,
        width: i64,
        height: i64,
    },
    Animation {
        speed: i32,
        images: Vec<Value>,
        started: Instant,
    },
    Font {
        size: i32,
    },
    Sound {
        path: String,
        handle: i64,
        music: bool,
    },
    Screen {
        id: u64,
        title: String,
        width: i32,
        height: i32,
    },
    Frame {
        screen_id: u64,
        title: String,
        width: i32,
        height: i32,
    },
    Canvas {
        screen_id: u64,
        width: i32,
        height: i32,
    },
    GraphicsContext {
        red: i32,
        green: i32,
        blue: i32,
        active: bool,
    },
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
        body: FunctionBody,
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
    Builtin(String),
    Partial {
        callable: Rc<Callable>,
        arguments: Vec<Value>,
    },
}

pub(crate) struct ObjectValue {
    pub class_name: String,
    pub constructor_fields: Vec<String>,
    pub fields: HashMap<String, Value>,
}

impl Value {
    pub(crate) fn display(&self) -> String {
        self.display_with_context(false)
    }

    fn display_with_context(&self, nested: bool) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Real(value) => format_real(*value),
            Self::BigInteger(value) | Self::Text(value) => value.clone(),
            Self::Character(value) => String::from_utf16_lossy(&[*value]),
            Self::Logical(value) if nested => value.to_string(),
            Self::Logical(true) => "verdadeiro".to_owned(),
            Self::Logical(false) => "falso".to_owned(),
            Self::Nothing => "()".to_owned(),
            Self::Option(Some(value)) => format!("Some({})", value.display_with_context(true)),
            Self::Option(None) => "None".to_owned(),
            Self::Tuple(values) => format!("({})", format_values(values, ",")),
            Self::List(values) => format_list(values),
            Self::MutableList(values) => format_list(&values.borrow()),
            Self::Function(_) => "<função>".to_owned(),
            Self::TypeName(name) => format!("<tipo {name}>"),
            Self::Object(object) => {
                let object = object.borrow();
                let fields = object
                    .constructor_fields
                    .iter()
                    .map(|name| {
                        object
                            .fields
                            .get(name)
                            .map(|value| value.display_with_context(true))
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{}({fields})", object.class_name)
            }
            Self::JerimumObject(object) => match &*object.borrow() {
                JerimumObject::Color { red, green, blue } => format!("Cor({red},{green},{blue})"),
                JerimumObject::AwtColor {
                    red, green, blue, ..
                } => format!("java.awt.Color[r={red},g={green},b={blue}]"),
                JerimumObject::Image { .. } => "<Imagem>".to_owned(),
                JerimumObject::ImageBuffer { .. } => "<BufferedImage>".to_owned(),
                JerimumObject::Animation { speed, images, .. } => {
                    format!("Animacao({speed},{})", format_list(images))
                }
                JerimumObject::Font { size } => format!("Fonte({size})"),
                JerimumObject::Sound { path, music, .. } => {
                    format!("{}({path})", if *music { "Musica" } else { "Som" })
                }
                JerimumObject::Screen {
                    title,
                    width,
                    height,
                    ..
                } => {
                    format!("Tela({title},{width},{height})")
                }
                JerimumObject::Frame { title, .. } => format!("<JFrame:{title}>"),
                JerimumObject::Canvas { .. } => "<Canvas>".to_owned(),
                JerimumObject::GraphicsContext { .. } => "<Graphics2D>".to_owned(),
            },
            Self::FieldReference(_, _) => "<atributo>".to_owned(),
        }
    }
}

fn format_list(values: &[Value]) -> String {
    format!("[{}]", format_values(values, ", "))
}

fn format_values(values: &[Value], separator: &str) -> String {
    values
        .iter()
        .map(|value| value.display_with_context(true))
        .collect::<Vec<_>>()
        .join(separator)
}

fn format_real(value: f64) -> String {
    let text = value.to_string();
    if value.is_finite() && !text.contains('.') && !text.contains('e') && !text.contains('E') {
        format!("{text}.0")
    } else {
        text
    }
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
            (Self::Option(left), Self::Option(right)) => left == right,
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
            (Self::Object(left), Self::Object(right)) => {
                let left = left.borrow();
                let right = right.borrow();
                left.class_name == right.class_name
                    && left
                        .constructor_fields
                        .iter()
                        .all(|name| left.fields.get(name) == right.fields.get(name))
            }
            (Self::JerimumObject(left), Self::JerimumObject(right)) => {
                *left.borrow() == *right.borrow()
            }
            (Self::FieldReference(left, left_name), Self::FieldReference(right, right_name)) => {
                Rc::ptr_eq(left, right) && left_name == right_name
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Value;

    #[test]
    fn nothing_displays_as_scala_unit() {
        assert_eq!(Value::Nothing.display(), "()");
    }
}

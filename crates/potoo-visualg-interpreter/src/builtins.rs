use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::Diagnostic;
use crate::ast::Expression;
use crate::interpreter::runtime_error;
use crate::value::Value;

const BUILTIN_NAMES: &[&str] = &[
    "abs",
    "arccos",
    "arcsen",
    "arctan",
    "asc",
    "carac",
    "caracpnum",
    "compr",
    "copia",
    "cos",
    "cotan",
    "exp",
    "grauprad",
    "int",
    "log",
    "logn",
    "maiusc",
    "minusc",
    "numpcarac",
    "pi",
    "pos",
    "quad",
    "raizq",
    "rand",
    "randi",
    "radpgrau",
    "sen",
    "tan",
];

pub(crate) fn contains(name: &str) -> bool {
    BUILTIN_NAMES.contains(&name)
}

pub(crate) fn call(
    name: &str,
    arguments: Vec<Value>,
    expression: &Expression,
    max_string_bytes: usize,
) -> Result<Value, Diagnostic> {
    let (minimum, maximum) = match name {
        "exp" => (2, 2),
        "copia" => (3, 3),
        "pos" => (2, 2),
        "rand" | "pi" => (0, 0),
        "randi" | "abs" | "arccos" | "arcsen" | "arctan" | "asc" | "carac" | "caracpnum"
        | "compr" | "cos" | "grauprad" | "int" | "log" | "logn" | "maiusc" | "minusc"
        | "numpcarac" | "quad" | "raizq" | "radpgrau" | "sen" | "tan" | "cotan" => (1, 1),
        _ => {
            return Err(runtime_error(
                expression,
                "função pré-definida desconhecida",
            ));
        }
    };
    if arguments.len() < minimum || arguments.len() > maximum {
        let expected = if minimum == maximum {
            format!("{minimum}")
        } else {
            format!("de {minimum} a {maximum}")
        };
        return Err(runtime_error(
            expression,
            &format!("'{name}' espera {expected} argumento(s)"),
        ));
    }
    let argument = |index: usize| &arguments[index];
    let real = |index: usize| -> Result<f64, Diagnostic> {
        match argument(index) {
            Value::Integer(value) => Ok(*value as f64),
            Value::Real(value) => Ok(*value),
            value => Err(runtime_error(
                expression,
                &format!(
                    "'{name}' exige valor numérico, recebeu {}",
                    value.type_name()
                ),
            )),
        }
    };
    let real_only = |index: usize| -> Result<f64, Diagnostic> {
        match argument(index) {
            Value::Real(value) => Ok(*value),
            value => Err(runtime_error(
                expression,
                &format!("'{name}' exige valor real, recebeu {}", value.type_name()),
            )),
        }
    };
    let integer = |index: usize| -> Result<i64, Diagnostic> {
        match argument(index) {
            Value::Integer(value) => Ok(*value),
            value => Err(runtime_error(
                expression,
                &format!(
                    "'{name}' exige valor inteiro, recebeu {}",
                    value.type_name()
                ),
            )),
        }
    };
    let text = |index: usize| -> Result<&str, Diagnostic> {
        match argument(index) {
            Value::Text(value) => Ok(value),
            value => Err(runtime_error(
                expression,
                &format!("'{name}' exige caractere, recebeu {}", value.type_name()),
            )),
        }
    };

    let result = match name {
        "pi" => Value::Real(3.141592),
        "rand" => Value::Real(random_unit()),
        "randi" => {
            let upper = integer(0)?;
            if upper <= 0 {
                return Err(runtime_error(
                    expression,
                    "'RandI' exige um limite maior que zero",
                ));
            }
            Value::Integer((random_unit() * upper as f64) as i64)
        }
        "abs" => match argument(0) {
            Value::Integer(value) => Value::Integer(
                value
                    .checked_abs()
                    .ok_or_else(|| runtime_error(expression, "estouro em 'Abs'"))?,
            ),
            Value::Real(value) => Value::Real(value.abs()),
            value => {
                return Err(runtime_error(
                    expression,
                    &format!("'Abs' exige valor numérico, recebeu {}", value.type_name()),
                ));
            }
        },
        "arccos" => inverse_trig(real(0)?.acos(), expression)?,
        "arcsen" => inverse_trig(real(0)?.asin(), expression)?,
        "arctan" => Value::Real(real(0)?.atan()),
        "cos" => Value::Real(real(0)?.cos()),
        "cotan" => {
            let tangent = real(0)?.tan();
            if tangent == 0.0 {
                return Err(runtime_error(
                    expression,
                    "'CoTan' não está definida para esse valor",
                ));
            }
            finite(1.0 / tangent, expression, "resultado inválido em 'CoTan'")?
        }
        "exp" => {
            let value = real_only(0)?.powf(real_only(1)?);
            finite(value, expression, "resultado inválido em 'Exp'")?
        }
        "grauprad" => Value::Real(real(0)?.to_radians()),
        "int" => {
            let value = match argument(0) {
                Value::Integer(value) => return Ok(Value::Integer(*value)),
                Value::Real(value) => value.trunc(),
                value => {
                    return Err(runtime_error(
                        expression,
                        &format!("'Int' exige valor numérico, recebeu {}", value.type_name()),
                    ));
                }
            };
            if !value.is_finite() || value < i64::MIN as f64 || value >= -(i64::MIN as f64) {
                return Err(runtime_error(
                    expression,
                    "'Int' recebeu um número fora do limite inteiro",
                ));
            }
            Value::Integer(value.trunc() as i64)
        }
        "log" => positive_log(real(0)?, 10.0, expression, "Log")?,
        "logn" => positive_log(real(0)?, std::f64::consts::E, expression, "LogN")?,
        "quad" => {
            let value = real(0)?;
            finite(value * value, expression, "resultado inválido em 'Quad'")?
        }
        "radpgrau" => finite(
            real(0)?.to_degrees(),
            expression,
            "resultado inválido em 'RadpGrau'",
        )?,
        "raizq" => {
            let value = real(0)?;
            if value < 0.0 {
                return Err(runtime_error(
                    expression,
                    "'RaizQ' exige um valor não negativo",
                ));
            }
            finite(value.sqrt(), expression, "resultado inválido em 'RaizQ'")?
        }
        "sen" => Value::Real(real(0)?.sin()),
        "tan" => Value::Real(real(0)?.tan()),
        "asc" => {
            let character = text(0)?
                .chars()
                .next()
                .ok_or_else(|| runtime_error(expression, "'Asc' exige um caractere não vazio"))?;
            if character as u32 > 255 {
                return Err(runtime_error(
                    expression,
                    "'Asc' só aceita caracteres do conjunto de códigos de 0 a 255",
                ));
            }
            Value::Integer(character as i64)
        }
        "carac" => {
            let code = integer(0)?;
            if !(0..=255).contains(&code) {
                return Err(runtime_error(
                    expression,
                    "'Carac' só aceita códigos de 0 a 255",
                ));
            }
            let code = u32::try_from(code)
                .map_err(|_| runtime_error(expression, "código inválido para 'Carac'"))?;
            let character = char::from_u32(code)
                .ok_or_else(|| runtime_error(expression, "código inválido para 'Carac'"))?;
            Value::Text(character.to_string())
        }
        "caracpnum" => {
            let source = text(0)?.trim();
            if let Ok(value) = source.parse::<i64>() {
                Value::Integer(value)
            } else {
                let value = source.parse::<f64>().map_err(|_| {
                    runtime_error(
                        expression,
                        "'Caracpnum' recebeu texto que não representa número",
                    )
                })?;
                if !value.is_finite() {
                    return Err(runtime_error(
                        expression,
                        "'Caracpnum' recebeu número fora do limite",
                    ));
                }
                Value::Real(value)
            }
        }
        "compr" => Value::Integer(i64::try_from(text(0)?.chars().count()).unwrap_or(i64::MAX)),
        "copia" => {
            let source = text(0)?;
            let start = usize::try_from(integer(1)?)
                .map_err(|_| runtime_error(expression, "posição inválida em 'Copia'"))?;
            let length = usize::try_from(integer(2)?)
                .map_err(|_| runtime_error(expression, "tamanho inválido em 'Copia'"))?;
            if start == 0 {
                return Err(runtime_error(
                    expression,
                    "as posições de 'Copia' começam em 1",
                ));
            }
            let copied = source
                .chars()
                .skip(start - 1)
                .take(length)
                .collect::<String>();
            if copied.len() > max_string_bytes {
                return Err(runtime_error(
                    expression,
                    "texto retornado por 'Copia' excede o limite",
                ));
            }
            Value::Text(copied)
        }
        "maiusc" => bounded_text(text(0)?.to_uppercase(), expression, max_string_bytes)?,
        "minusc" => bounded_text(text(0)?.to_lowercase(), expression, max_string_bytes)?,
        "numpcarac" => match argument(0) {
            Value::Integer(value) => Value::Text(value.to_string()),
            Value::Real(value) if value.is_finite() => Value::Text(value.to_string()),
            value => {
                return Err(runtime_error(
                    expression,
                    &format!(
                        "'Numpcarac' exige valor numérico, recebeu {}",
                        value.type_name()
                    ),
                ));
            }
        },
        "pos" => {
            let needle = text(0)?;
            let haystack = text(1)?;
            let position = if needle.is_empty() {
                1
            } else {
                haystack
                    .find(needle)
                    .map(|byte| haystack[..byte].chars().count() + 1)
                    .unwrap_or(0)
            };
            Value::Integer(position as i64)
        }
        _ => unreachable!("all builtin cases are listed above"),
    };
    Ok(result)
}

fn inverse_trig(value: f64, expression: &Expression) -> Result<Value, Diagnostic> {
    if value.is_nan() {
        Err(runtime_error(
            expression,
            "argumento fora do domínio da função trigonométrica",
        ))
    } else {
        Ok(Value::Real(value))
    }
}

fn positive_log(
    value: f64,
    base: f64,
    expression: &Expression,
    name: &str,
) -> Result<Value, Diagnostic> {
    if value <= 0.0 {
        return Err(runtime_error(
            expression,
            &format!("'{name}' exige um valor maior que zero"),
        ));
    }
    finite(
        value.log(base),
        expression,
        "resultado inválido de logaritmo",
    )
}

fn finite(value: f64, expression: &Expression, message: &str) -> Result<Value, Diagnostic> {
    if value.is_finite() {
        Ok(Value::Real(value))
    } else {
        Err(runtime_error(expression, message))
    }
}

fn bounded_text(
    text: String,
    expression: &Expression,
    max_bytes: usize,
) -> Result<Value, Diagnostic> {
    if text.len() > max_bytes {
        Err(runtime_error(
            expression,
            "texto retornado pela função excede o limite",
        ))
    } else {
        Ok(Value::Text(text))
    }
}

pub(crate) fn random_u64() -> u64 {
    static STATE: AtomicU64 = AtomicU64::new(0);
    loop {
        let mut current = STATE.load(Ordering::Relaxed);
        if current == 0 {
            let seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(1, |duration| duration.as_nanos() as u64)
                .max(1);
            if STATE
                .compare_exchange(0, seed, Ordering::Relaxed, Ordering::Relaxed)
                .is_err()
            {
                continue;
            }
            current = seed;
        }
        let mut next = current;
        next ^= next << 13;
        next ^= next >> 7;
        next ^= next << 17;
        if STATE
            .compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return next;
        }
    }
}

pub(crate) fn random_unit() -> f64 {
    (random_u64() >> 11) as f64 / ((1_u64 << 53) as f64)
}

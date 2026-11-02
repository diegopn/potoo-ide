use crate::ast::Expression;
use crate::value::Value;
use crate::{Diagnostic, DiagnosticKind};

pub(crate) fn format_value(
    value: &Value,
    width: Option<i64>,
    precision: Option<i64>,
    maximum_size: usize,
    expression: &Expression,
) -> Result<String, Diagnostic> {
    let width = width
        .map(|width| {
            usize::try_from(width)
                .map_err(|_| format_error(expression, "largura de saída inválida"))
        })
        .transpose()?;
    if width.is_some_and(|width| width > maximum_size) {
        return Err(format_error(expression, "largura de saída excede o limite"));
    }
    let content = match value {
        Value::Real(number) => {
            let formatted = if let Some(precision) = precision {
                let precision = usize::try_from(precision)
                    .map_err(|_| format_error(expression, "precisão de saída inválida"))?;
                if precision > maximum_size {
                    return Err(format_error(
                        expression,
                        "precisão de saída excede o limite",
                    ));
                }
                format!("{number:.precision$}")
            } else {
                number.to_string()
            };
            apply_width(formatted, width)
        }
        Value::Integer(_) | Value::Logical(_) => apply_width(value.to_text(), width),
        Value::Text(text) => apply_width(text.clone(), width),
        Value::Array(_) | Value::Void => value.to_text(),
    };
    let prefix = if matches!(
        value,
        Value::Integer(_) | Value::Real(_) | Value::Logical(_)
    ) {
        " "
    } else {
        ""
    };
    let output = format!("{prefix}{content}");
    if output.len() > maximum_size {
        return Err(format_error(expression, "texto formatado excede o limite"));
    }
    Ok(output)
}

fn apply_width(content: String, width: Option<usize>) -> String {
    match width {
        Some(width) if content.chars().count() < width => format!("{:>width$}", content),
        _ => content,
    }
}

fn format_error(expression: &Expression, message: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticKind::Runtime,
        message,
        expression.line,
        expression.column,
    )
}

use super::*;

pub(super) fn integer_argument(arguments: &[HostValue], index: usize) -> Result<i64, String> {
    match arguments.get(index) {
        Some(HostValue::Integer(value)) => Ok(*value),
        _ => Err(format!("argumento {} deve ser inteiro", index + 1)),
    }
}

pub(super) fn i32_argument(
    arguments: &[HostValue],
    index: usize,
    message: &str,
) -> Result<i32, String> {
    i32::try_from(integer_argument(arguments, index)?).map_err(|_| message.to_owned())
}

pub(super) fn logical_argument(arguments: &[HostValue], index: usize) -> Result<bool, String> {
    match arguments.get(index) {
        Some(HostValue::Logical(value)) => Ok(*value),
        _ => Err(format!("argumento {} deve ser lógico", index + 1)),
    }
}

pub(super) fn string_argument(arguments: &[HostValue], index: usize) -> Result<String, String> {
    match arguments.get(index) {
        Some(HostValue::String(value)) => Ok(value.clone()),
        Some(HostValue::Character(value)) => Ok(value.to_string()),
        _ => Err(format!(
            "argumento {} deve ser cadeia de caracteres",
            index + 1
        )),
    }
}

pub(super) fn real_argument(arguments: &[HostValue], index: usize) -> Result<f64, String> {
    match arguments.get(index) {
        Some(HostValue::Real(value)) => Ok(*value),
        Some(HostValue::Integer(value)) => Ok(*value as f64),
        _ => Err(format!("argumento {} deve ser numérico", index + 1)),
    }
}

pub(super) fn checked_dimension(value: i64) -> Result<i32, String> {
    if !(1..=8192).contains(&value) {
        return Err("a dimensão deve estar entre 1 e 8192 pixels".to_owned());
    }
    i32::try_from(value).map_err(|_| "dimensão inválida".to_owned())
}

pub(super) fn check_pixel_count(width: i32, height: i32) -> Result<(), String> {
    if i64::from(width) * i64::from(height) > 32 * 1024 * 1024 {
        return Err("a imagem excede o limite de 32 milhões de pixels".to_owned());
    }
    Ok(())
}

pub(super) fn point_array_argument(
    arguments: &[HostValue],
    index: usize,
) -> Result<Vec<(f64, f64)>, String> {
    let Some(HostValue::Array(points)) = arguments.get(index) else {
        return Err("os pontos do polígono devem formar uma matriz de inteiros".to_owned());
    };
    points
        .iter()
        .map(|point| {
            let HostValue::Array(coordinates) = point else {
                return Err("cada ponto do polígono deve conter X e Y".to_owned());
            };
            if coordinates.len() != 2 {
                return Err("cada ponto do polígono deve conter X e Y".to_owned());
            }
            match (&coordinates[0], &coordinates[1]) {
                (HostValue::Integer(x), HostValue::Integer(y)) => Ok((*x as f64, *y as f64)),
                _ => Err("as coordenadas do polígono devem ser inteiras".to_owned()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_arguments_report_missing_values() {
        assert_eq!(
            integer_argument(&[], 0),
            Err("argumento 1 deve ser inteiro".to_owned())
        );
    }

    #[test]
    fn graphics_dimensions_keep_the_supported_range() {
        assert_eq!(checked_dimension(1), Ok(1));
        assert_eq!(checked_dimension(8192), Ok(8192));
        assert_eq!(
            checked_dimension(0),
            Err("a dimensão deve estar entre 1 e 8192 pixels".to_owned())
        );
        assert_eq!(
            checked_dimension(8193),
            Err("a dimensão deve estar entre 1 e 8192 pixels".to_owned())
        );
    }
}

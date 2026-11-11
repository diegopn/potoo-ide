use crate::languages::model::SourceDimension;
use std::collections::{HashMap, HashSet};

pub(crate) fn insert_unique_bounds(
    bounds: &mut HashMap<String, Vec<SourceDimension>>,
    ambiguous: &mut HashSet<String>,
    name: &str,
    dimensions: Vec<SourceDimension>,
    case_sensitive: bool,
) {
    let name = if case_sensitive {
        name.to_owned()
    } else {
        name.to_lowercase()
    };
    if ambiguous.contains(&name) {
        return;
    }
    if bounds.remove(&name).is_some() {
        ambiguous.insert(name);
    } else {
        bounds.insert(name, dimensions);
    }
}

#[cfg(test)]
mod tests {
    use super::insert_unique_bounds;
    use crate::languages::model::SourceDimension;
    use std::collections::{HashMap, HashSet};

    fn dimension(lower: i64, upper: i64) -> Vec<SourceDimension> {
        vec![SourceDimension {
            lower_bound: Some(lower),
            upper_bound: Some(upper),
        }]
    }

    #[test]
    fn inserts_a_unique_bound() {
        let mut bounds = HashMap::new();
        let mut ambiguous = HashSet::new();

        insert_unique_bounds(&mut bounds, &mut ambiguous, "dados", dimension(0, 2), true);

        assert_eq!(bounds.get("dados"), Some(&dimension(0, 2)));
        assert!(ambiguous.is_empty());
    }

    #[test]
    fn removes_duplicate_bounds_and_marks_the_name_ambiguous() {
        let mut bounds = HashMap::new();
        let mut ambiguous = HashSet::new();

        insert_unique_bounds(&mut bounds, &mut ambiguous, "dados", dimension(0, 2), true);
        insert_unique_bounds(&mut bounds, &mut ambiguous, "dados", dimension(1, 3), true);
        insert_unique_bounds(&mut bounds, &mut ambiguous, "dados", dimension(0, 4), true);

        assert!(!bounds.contains_key("dados"));
        assert!(ambiguous.contains("dados"));
    }

    #[test]
    fn identifier_case_follows_the_language_profile() {
        let mut sensitive_bounds = HashMap::new();
        let mut sensitive_ambiguous = HashSet::new();
        insert_unique_bounds(
            &mut sensitive_bounds,
            &mut sensitive_ambiguous,
            "Dados",
            dimension(0, 2),
            true,
        );
        insert_unique_bounds(
            &mut sensitive_bounds,
            &mut sensitive_ambiguous,
            "dados",
            dimension(1, 3),
            true,
        );
        assert_eq!(sensitive_bounds.len(), 2);
        assert!(sensitive_ambiguous.is_empty());

        let mut insensitive_bounds = HashMap::new();
        let mut insensitive_ambiguous = HashSet::new();
        insert_unique_bounds(
            &mut insensitive_bounds,
            &mut insensitive_ambiguous,
            "Dados",
            dimension(0, 2),
            false,
        );
        insert_unique_bounds(
            &mut insensitive_bounds,
            &mut insensitive_ambiguous,
            "dados",
            dimension(1, 3),
            false,
        );
        assert!(insensitive_bounds.is_empty());
        assert!(insensitive_ambiguous.contains("dados"));
    }
}

mod format;

pub use self::format::Format;

use std::collections::HashMap;

pub fn feature_names_eq(features: &[(i32, String)], counts: &HashMap<String, u64>) -> bool {
    if features.len() != counts.len() {
        return false;
    }

    for (_, name) in features {
        if !counts.contains_key(name) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_names_eq() {
        let features = [];
        let counts = HashMap::new();
        assert!(feature_names_eq(&features, &counts));

        let features = [(1, String::from("f1"))];
        let counts = [(String::from("f1"), 0)].into_iter().collect();
        assert!(feature_names_eq(&features, &counts));

        let features = [(1, String::from("f1")), (2, String::from("f2"))];
        let counts = [(String::from("f1"), 0)].into_iter().collect();
        assert!(!feature_names_eq(&features, &counts));

        let features = [(1, String::from("f1"))];
        let counts = [(String::from("f1"), 0), (String::from("f2"), 0)]
            .into_iter()
            .collect();
        assert!(!feature_names_eq(&features, &counts));
    }
}

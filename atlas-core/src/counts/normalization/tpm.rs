use std::{collections::HashMap, io};

pub fn normalize_map(
    feature_lengths: &HashMap<String, i32>,
    counts: &HashMap<String, i32>,
) -> io::Result<HashMap<String, f64>> {
    let mut feature_names: Vec<_> = counts.keys().collect();
    feature_names.sort();

    let feature_lengths: Vec<_> = feature_names
        .iter()
        .map(|name| feature_lengths[*name])
        .collect();

    let counts: Vec<_> = feature_names.iter().map(|name| counts[*name]).collect();

    let fpkms = normalize(&feature_lengths, &counts);

    Ok(feature_names
        .into_iter()
        .zip(fpkms)
        .map(|(name, value)| (name.into(), value))
        .collect())
}

pub fn normalize(feature_lengths: &[i32], counts: &[i32]) -> Vec<f64> {
    let length_normalized_counts: Vec<_> = feature_lengths
        .iter()
        .zip(counts)
        .map(|(length, count)| {
            assert!(*length > 0);
            f64::from(*count) / f64::from(*length)
        })
        .collect();

    let sum = length_normalized_counts.iter().sum();

    length_normalized_counts
        .into_iter()
        .map(|normalized_count| calculate_tpm(normalized_count, sum))
        .collect()
}

fn calculate_tpm(n: f64, sum: f64) -> f64 {
    assert!(sum > 0.0);
    n * 1e6 / sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        fn assert_approx_eq(a: f64, b: f64) {
            const EPSILON: f64 = 1e-9;
            assert!((a - b).abs() < EPSILON);
        }

        let feature_lengths = [17711, 10946, 233];
        let counts = [610, 2, 6765];
        let sum = 610.0 / 17711.0 + 2.0 / 10946.0 + 6765.0 / 233.0;

        let actual = normalize(&feature_lengths, &counts);

        let expected = (610.0 / 17711.0) * 1e6 / sum;
        assert_approx_eq(actual[0], expected);

        let expected = (2.0 / 10946.0) * 1e6 / sum;
        assert_approx_eq(actual[1], expected);

        let expected = (6765.0 / 233.0) * 1e6 / sum;
        assert_approx_eq(actual[2], expected);
    }
}

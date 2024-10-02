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
    let sum = counts.iter().copied().map(f64::from).sum();

    feature_lengths
        .iter()
        .zip(counts)
        .map(|(length, count)| {
            assert!(*length > 0);
            calculate_fpkm(f64::from(*count), f64::from(*length), sum)
        })
        .collect()
}

fn calculate_fpkm(count: f64, length: f64, sum: f64) -> f64 {
    count * 1e9 / (length * sum)
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
        let sum = 610.0 + 2.0 + 6765.0;

        let values = normalize(&feature_lengths, &counts);

        let expected = 610.0 * 1e9 / (17711.0 * sum);
        assert_approx_eq(values[0], expected);

        let expected = 2.0 * 1e9 / (10946.0 * sum);
        assert_approx_eq(values[1], expected);

        let expected = 6765.0 * 1e9 / (233.0 * sum);
        assert_approx_eq(values[2], expected);
    }
}

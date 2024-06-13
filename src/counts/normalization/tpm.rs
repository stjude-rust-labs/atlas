use std::{collections::HashMap, io};

#[allow(dead_code)]
pub fn calculate_tpms(
    features: &HashMap<String, i32>,
    counts: &HashMap<String, i32>,
) -> io::Result<HashMap<String, f64>> {
    let length_normalized_counts: HashMap<String, f64> = counts
        .iter()
        .map(|(name, count)| {
            features
                .get(name)
                .map(|&length| {
                    assert!(length > 0);
                    (name.clone(), f64::from(*count) / f64::from(length))
                })
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing feature"))
        })
        .collect::<io::Result<_>>()?;

    let sum = length_normalized_counts.values().sum();

    let tpms = length_normalized_counts
        .into_iter()
        .map(|(name, normalized_count)| (name, calculate_tpm(normalized_count, sum)))
        .collect();

    Ok(tpms)
}

fn calculate_tpm(n: f64, sum: f64) -> f64 {
    assert!(sum > 0.0);
    n * 1e6 / sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_tpms() -> io::Result<()> {
        fn assert_approx_eq(a: f64, b: f64) {
            const EPSILON: f64 = 1e-9;
            assert!((a - b).abs() < EPSILON);
        }

        let features = [
            (String::from("f0"), 17711),
            (String::from("f1"), 10946),
            (String::from("f2"), 233),
        ]
        .into_iter()
        .collect();

        let counts = [
            (String::from("f0"), 610),
            (String::from("f1"), 2),
            (String::from("f2"), 6765),
        ]
        .into_iter()
        .collect();

        let tpms = calculate_tpms(&features, &counts)?;

        let sum = 610.0 / 17711.0 + 2.0 / 10946.0 + 6765.0 / 233.0;

        let actual = tpms["f0"];
        let expected = (610.0 / 17711.0) * 1e6 / sum;
        assert_approx_eq(actual, expected);

        let actual = tpms["f1"];
        let expected = (2.0 / 10946.0) * 1e6 / sum;
        assert_approx_eq(actual, expected);

        let actual = tpms["f2"];
        let expected = (6765.0 / 233.0) * 1e6 / sum;
        assert_approx_eq(actual, expected);

        Ok(())
    }
}

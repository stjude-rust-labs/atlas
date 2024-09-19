use std::{collections::HashMap, io};

pub fn calculate_fpkms(
    features: &HashMap<String, i32>,
    counts: &HashMap<String, i32>,
) -> io::Result<HashMap<String, f64>> {
    let sum = sum_counts(counts);

    counts
        .iter()
        .map(|(name, count)| {
            features
                .get(name)
                .map(|&length| {
                    assert!(length > 0);
                    (
                        name.clone(),
                        calculate_fpkm(f64::from(*count), f64::from(length), sum),
                    )
                })
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing feature"))
        })
        .collect()
}

fn sum_counts(counts: &HashMap<String, i32>) -> f64 {
    counts.values().copied().map(f64::from).sum()
}

fn calculate_fpkm(count: f64, length: f64, sum: f64) -> f64 {
    count * 1e9 / (length * sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_fpkms() -> io::Result<()> {
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

        let fpkms = calculate_fpkms(&features, &counts)?;

        let sum = 610.0 + 2.0 + 6765.0;

        let actual = fpkms["f0"];
        let expected = 610.0 * 1e9 / (17711.0 * sum);
        assert_approx_eq(actual, expected);

        let actual = fpkms["f1"];
        let expected = 2.0 * 1e9 / (10946.0 * sum);
        assert_approx_eq(actual, expected);

        let actual = fpkms["f2"];
        let expected = 6765.0 * 1e9 / (233.0 * sum);
        assert_approx_eq(actual, expected);

        Ok(())
    }
}

use ndarray::{Array2, Axis, Zip};

#[allow(dead_code)]
fn normalize(data: Array2<u32>) -> Array2<f64> {
    use std::f64::consts::E;

    assert!(!data.is_empty());

    // Normal to log: ln(n).
    let mut log_data = data.mapv(|n| (n as f64).ln());

    // Log mean of each feature: ln(μ).
    // SAFETY: The column count is > 0.
    let log_means = log_data.mean_axis(Axis(0)).unwrap();

    for mut row in log_data.rows_mut() {
        // Calculate ratios: ln(n / μ).
        row -= &log_means;

        // Replace all non-finite values with NaN.
        //
        // If a feature has a count of 0, ln(0) = -∞, which causes its mean to also be -∞. ln(0) -
        // -∞ = NaN, but ln(x) - -∞ = ∞ if x > 0.
        //
        // This normalizes ±∞ as NaN for future filtering.
        row.mapv_inplace(|n| if n.is_finite() { n } else { f64::NAN })
    }

    // Calculate median of ratios for each sample.
    //
    // All values are either finite or NaN. NaN values are ignored in the median calculation.
    let mut medians = log_data.map_axis(Axis(1), |row| {
        let mut values: Vec<_> = row
            .iter()
            .filter_map(|&n| if n.is_nan() { None } else { Some(n) })
            .collect();

        // SAFETY: All values are finite.
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        median(&values)
    });

    // Log to normal: e^n.
    medians.mapv_inplace(|n| E.powf(n));

    let mut normalized_data = data.mapv(|n| n as f64);

    // Normalize counts by medians.
    Zip::from(normalized_data.rows_mut())
        .and(&medians)
        .for_each(|mut row, &median| {
            row /= median;
        });

    normalized_data
}

// `values` must be non-empty and sorted.
fn median(values: &[f64]) -> f64 {
    let i = values.len() / 2;

    if values.len() % 2 == 0 {
        (values[i - 1] + values[i]) / 2.0
    } else {
        values[i]
    }
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    #[test]
    fn test_normalize() {
        fn assert_approx_eq(a: &Array2<f64>, b: &Array2<f64>) {
            const EPSILON: f64 = 1e-3;

            for (n, m) in a.iter().zip(b.iter()) {
                assert!((n - m).abs() < EPSILON);
            }
        }

        let data = array![[0, 8, 13], [21, 34, 55]];
        let actual = normalize(data);
        let expected = array![[0.0, 16.474, 26.770], [10.198, 16.511, 26.709]];
        assert_approx_eq(&actual, &expected);
    }

    #[test]
    fn test_median() {
        let values = [0.0, 1.0, 2.0];
        assert_eq!(median(&values), 1.0);

        let values = [0.0, 1.0];
        assert_eq!(median(&values), 0.5);
    }
}

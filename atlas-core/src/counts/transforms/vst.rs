use faer::{Col, Mat, MatRef};

pub fn transform(raw_counts: Vec<u32>, feature_count: usize, sample_count: usize) -> Vec<f64> {
    let raw_counts: Vec<_> = raw_counts.into_iter().map(|n| n as f64).collect();
    let counts = MatRef::from_row_major_slice(&raw_counts, feature_count, sample_count);

    let size_factors = calculate_size_factors(counts);
    let _normalized_counts = normalize(counts, &size_factors);

    todo!()
}

fn calculate_size_factors(counts: MatRef<'_, f64>) -> Vec<f64> {
    use faer::stats::{NanHandling, col_mean};

    let (feature_count, sample_count) = counts.shape();

    let log_counts = counts.map(|n| n.ln());
    let mut log_means = Col::zeros(feature_count);

    col_mean(
        log_means.as_mut(),
        log_counts.as_ref(),
        NanHandling::Propagate,
    );

    let mut size_factors = vec![0.0; sample_count];
    let mut log_ratios = Vec::new();

    for j in 0..sample_count {
        log_ratios.clear();

        for (i, log_mean) in log_means.iter().enumerate() {
            if log_mean.is_finite() {
                let log_n = log_counts[(i, j)];
                log_ratios.push(log_n - log_mean);
            }
        }

        size_factors[j] = median(&mut log_ratios).exp();
    }

    size_factors
}

fn normalize(counts: MatRef<'_, f64>, size_factors: &[f64]) -> Mat<f64> {
    let mut normalized_counts = counts.to_owned();

    for (col, size_factor) in normalized_counts.col_iter_mut().zip(size_factors) {
        for n in col.iter_mut() {
            *n /= size_factor;
        }
    }

    normalized_counts
}

fn median(values: &mut [f64]) -> f64 {
    // Assume all values are finite.
    values.sort_unstable_by(|a, b| a.total_cmp(b));

    let i = values.len() / 2;

    if values.len().is_multiple_of(2) {
        (values[i - 1] + values[i]) / 2.0
    } else {
        values[i]
    }
}

#[cfg(test)]
mod tests {
    use faer::mat;

    use super::*;

    #[test]
    fn test_calculate_median_of_ratios() {
        fn assert_approx_eq(a: &[f64], b: &[f64]) {
            const EPSILON: f64 = 1e-3;

            for (n, m) in a.iter().zip(b) {
                assert!((n - m).abs() < EPSILON);
            }
        }

        let counts = mat![[0.0, 21.0], [8.0, 34.0], [13.0, 55.0]];
        let actual = calculate_size_factors(counts.as_ref());
        let expected = [0.486, 2.059];

        assert_approx_eq(&actual, &expected);
    }

    #[test]
    fn test_median() {
        let mut values = [2.0, 0.0, 1.0];
        assert_eq!(median(&mut values), 1.0);

        let mut values = [1.0, 0.0];
        assert_eq!(median(&mut values), 0.5);
    }
}

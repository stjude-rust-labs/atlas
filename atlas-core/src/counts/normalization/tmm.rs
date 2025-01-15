//! Trimmed mean of M-values (TMM).
//!
//! Trimmed mean of M-values (TMM) is the normalization method used in [edgeR].
//! See "[A scaling normalization method for differential expression analysis of
//! RNA-seq data](10.1186/gb-2010-11-3-r25)" (2010) by Robinson and Oshlack for
//! more details.
//!
//! [edgeR]: https://bioconductor.org/packages/release/bioc/html/edgeR.html
//! [10.1186/gb-2010-11-3-r25]: https://doi.org/10.1186/gb-2010-11-3-r25

use std::collections::HashSet;

use ndarray::{Array2, ArrayView1, ArrayView2, ArrayViewMut2};
use tracing::info;

pub fn normalize(data: Array2<u32>) -> Array2<f64> {
    let mut data = data.mapv(f64::from);
    let mut normalized_data = data.clone();

    info!("normalizing counts");
    normalize_rows(normalized_data.view_mut());

    info!("finding reference sample");
    let i = find_reference_sample_index(normalized_data.view());
    info!(i, "found reference sample");

    info!("calculating scaling factors");
    let mut scaling_factors = calculate_scaling_factors(normalized_data.view(), i);

    info!("centering scaling factors");
    center(&mut scaling_factors);

    info!("scaling counts");

    for (mut row, &scaling_factor) in data.rows_mut().into_iter().zip(&scaling_factors) {
        row *= scaling_factor;
    }

    data
}

fn normalize_rows(mut data: ArrayViewMut2<'_, f64>) {
    for mut row in data.rows_mut() {
        row /= row.sum();
    }
}

fn find_reference_sample_index(data: ArrayView2<f64>) -> usize {
    // third quartile
    const Q3: f64 = 0.75;

    let q3s: Vec<_> = data
        .rows()
        .into_iter()
        .map(|row| quantile(row, Q3))
        .collect();

    let avg_q3 = q3s.iter().sum::<f64>() / (q3s.len() as f64);

    find_closest_item_index(&q3s, avg_q3)
}

// Computes the quantile at the given probability `p`.
//
// This uses the same methodology and constants as Julia, R, NumPy, etc., i.e.,
//
// ```text
// α = β = 1
// m = α + p * (1 - α - β)
// n = |x|
// j = ⌊n * p + m⌋
// γ = n * p + m - j
// Q(p) = (1 - γ) * x[j] + γ * x[j + 1]
// ```
fn quantile(row: ArrayView1<f64>, p: f64) -> f64 {
    const ALPHA: f64 = 1.0;
    const BETA: f64 = 1.0;

    assert!(!row.is_empty());
    assert!((0.0..=1.0).contains(&p));

    let mut x = row.to_vec();
    x.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let n = x.len() as f64;
    let m = ALPHA + p * (1.0 - ALPHA - BETA);
    let j = (n * p + m).floor();
    let gamma = n * p + m - j;

    let i = j as usize;
    (1.0 - gamma) * x[i - 1] + gamma * x[i]
}

fn find_closest_item_index(haystack: &[f64], needle: f64) -> usize {
    let mut min_delta = f64::MAX;
    let mut i = 0;

    for (j, &n) in haystack.iter().enumerate() {
        let delta = (n - needle).abs();

        if delta < min_delta {
            min_delta = delta;
            i = j;
        }
    }

    i
}

fn calculate_scaling_factors(data: ArrayView2<f64>, reference_index: usize) -> Vec<f64> {
    let reference_row = data.row(reference_index);

    data.rows()
        .into_iter()
        .enumerate()
        .map(|(i, relative_row)| {
            if i == reference_index {
                1.0
            } else {
                calculate_scaling_factor(reference_row, relative_row)
            }
        })
        .collect()
}

fn calculate_scaling_factor(reference_row: ArrayView1<f64>, relative_row: ArrayView1<f64>) -> f64 {
    // "By default, we time the `M_g` values by 30% and the `A_g` values by 5%..."
    const LOG2_RATIOS_TRIM_PERCENTAGE: f64 = 0.30;
    const MEAN_OF_LOGS_TRIM_PERCENTAGE: f64 = 0.05;

    // log-fold changes (M_g)
    let mut log2_ratios = Vec::with_capacity(reference_row.len());

    // absolute intensities (A_g)
    let mut mean_of_logs = Vec::with_capacity(reference_row.len());

    for (i, (&a, &b)) in reference_row.iter().zip(relative_row.iter()).enumerate() {
        if a == 0.0 || b == 0.0 {
            continue;
        }

        let n = (a / b).log2();
        log2_ratios.push((i, n));

        let u = (a.log2() + b.log2()) / 2.0;
        mean_of_logs.push((i, u));
    }

    log2_ratios.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
    mean_of_logs.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());

    let trimmed_log2_ratios = trim(&log2_ratios, LOG2_RATIOS_TRIM_PERCENTAGE);
    let trimmed_mean_of_logs = trim(&mean_of_logs, MEAN_OF_LOGS_TRIM_PERCENTAGE);

    let trimmed_mean_of_logs_ids: HashSet<_> =
        trimmed_mean_of_logs.iter().map(|(i, _)| *i).collect();

    let mut sum = 0.0;
    let mut hit_count = 0;

    for (i, n) in trimmed_log2_ratios.iter() {
        if !trimmed_mean_of_logs_ids.contains(i) {
            continue;
        }

        sum += n;
        hit_count += 1;
    }

    let mean = sum / (hit_count as f64);

    mean.exp2()
}

fn trim<T>(values: &[T], p: f64) -> &[T] {
    assert!(0.0 < p && p <= 0.5);

    let n = values.len() as f64;
    let drop_count = (n * p).round() as usize;

    let start = drop_count;
    let end = values.len() - drop_count;

    &values[start..end]
}

fn center(values: &mut [f64]) {
    let mean = geometric_mean(values);

    for n in values {
        *n /= mean;
    }
}

// https://en.wikipedia.org/wiki/Geometric_mean#Formulation_using_logarithms
fn geometric_mean(values: &[f64]) -> f64 {
    let sum: f64 = values.iter().map(|n| n.ln()).sum();
    let mean = sum / (values.len() as f64);
    mean.exp()
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;

    fn assert_approx_eq(a: f64, b: f64) {
        const EPSILON: f64 = 1e-6;
        assert!((a - b).abs() < EPSILON);
    }

    #[test]
    fn test_normalize_rows() {
        let mut x = array![[1.0, 2.0], [3.0, 4.0]];

        normalize_rows(x.view_mut());

        assert_approx_eq(x[(0, 0)], 1.0 / 3.0);
        assert_approx_eq(x[(0, 1)], 2.0 / 3.0);
        assert_approx_eq(x[(1, 0)], 3.0 / 7.0);
        assert_approx_eq(x[(1, 1)], 4.0 / 7.0);
    }

    #[test]
    fn test_quantile() {
        let x = array![1.0, 2.0, 3.0, 4.0];
        assert_approx_eq(quantile(x.view(), 0.25), 1.75);
        assert_approx_eq(quantile(x.view(), 0.50), 2.5);
        assert_approx_eq(quantile(x.view(), 0.75), 3.25);

        let x = array![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_approx_eq(quantile(x.view(), 0.25), 2.0);
        assert_approx_eq(quantile(x.view(), 0.50), 3.0);
        assert_approx_eq(quantile(x.view(), 0.75), 4.0);
    }

    #[test]
    fn test_find_closest_item_index() {
        let x = [1.0, 3.0, 4.0, 8.0];

        assert_eq!(find_closest_item_index(&x, 0.0), 0);
        assert_eq!(find_closest_item_index(&x, 1.1), 0);
        assert_eq!(find_closest_item_index(&x, 2.0), 0);
        assert_eq!(find_closest_item_index(&x, 3.0), 1);
        assert_eq!(find_closest_item_index(&x, 3.9), 2);
        assert_eq!(find_closest_item_index(&x, 9.0), 3);
    }

    #[test]
    fn test_trim() {
        let x = [0, 1, 2, 3, 4, 5, 6, 7];

        assert_eq!(trim(&x, 0.05), &x);
        assert_eq!(trim(&x, 0.25), &x[2..6]);
        assert_eq!(trim(&x, 0.30), &x[2..6]);
        assert_eq!(trim(&x, 0.35), &x[3..5]);
        assert!(trim(&x, 0.50).is_empty());
    }

    #[test]
    fn test_center() {
        let mut x = [0.4, 0.5, 0.6];

        center(&mut x);

        assert_approx_eq(x[0], 0.810960);
        assert_approx_eq(x[1], 1.013700);
        assert_approx_eq(x[2], 1.216440);
    }

    #[test]
    fn test_geometric_mean() {
        let x = [1.0, 2.0, 3.0, 4.0];

        assert_approx_eq(
            geometric_mean(&x),
            x.iter().product::<f64>().powf(1.0 / 4.0),
        );
    }
}

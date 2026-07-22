use std::f64::consts::LN_2;

use faer::{
    Col, ColRef, Mat, MatRef, RowRef,
    diag::Diag,
    prelude::Solve,
    stats::{NanHandling, col_mean},
};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use statrs::function::gamma::{digamma, ln_gamma};

pub fn transform(raw_counts: Vec<u32>, feature_count: usize, sample_count: usize) -> Vec<f64> {
    let raw_counts: Vec<_> = raw_counts.into_iter().map(|n| n as f64).collect();
    let counts = MatRef::from_row_major_slice(&raw_counts, feature_count, sample_count);

    let size_factors = calculate_size_factors(counts);
    let normalized_counts = normalize(counts, &size_factors);

    let dispersions =
        estimate_feature_dispersions(counts, normalized_counts.as_ref(), &size_factors);

    let (a0, a1) = estimate_dispersions_fit(normalized_counts.as_ref(), &dispersions);

    let stabilized_counts = stabilize_counts(normalized_counts.as_ref(), a0, a1);

    stabilized_counts
        .row_iter()
        .flat_map(|row| row.iter().copied())
        .collect()
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

fn estimate_feature_dispersions(
    counts: MatRef<'_, f64>,
    normalized_counts: MatRef<'_, f64>,
    size_factors: &[f64],
) -> Vec<f64> {
    let (feature_count, _) = counts.shape();

    (0..feature_count)
        .into_par_iter()
        .map(|i| {
            let row = counts.row(i);
            let normalized_row = normalized_counts.row(i);
            estimate_feature_dispersions_inner(row, normalized_row, size_factors)
        })
        .collect()
}

fn estimate_feature_dispersions_inner(
    counts: RowRef<'_, f64>,
    normalized_counts: RowRef<'_, f64>,
    size_factors: &[f64],
) -> f64 {
    const MIN_DISPERSION: f64 = 1e-8;
    const MIN_SAMPLE_COUNT: usize = 10;
    const MIN_MU: f64 = 0.5;

    let rough_dispersion = estimate_rough_dispersion(normalized_counts);
    let moments_dispersions = estimate_moments_dispersion(normalized_counts, size_factors);

    let (_, sample_count) = counts.shape();
    let max_dispersion = sample_count.max(MIN_SAMPLE_COUNT) as f64;

    let alpha_init = rough_dispersion
        .min(moments_dispersions)
        .clamp(MIN_DISPERSION, max_dispersion);

    let mut mus = build_normalized_mean_linear_model(normalized_counts, size_factors);

    for m in &mut mus {
        *m = m.max(MIN_MU);
    }

    let (alpha_mle, initial_lp, last_lp, did_converge) =
        fit_dispersion(counts, &mus, alpha_init, MIN_DISPERSION, max_dispersion);

    let did_not_increase = last_lp < initial_lp + initial_lp.abs() / 1e6;

    let mut result = if did_not_increase {
        alpha_init
    } else {
        alpha_mle
    };

    let needs_refit = !did_converge && result > MIN_DISPERSION * 10.0;

    if needs_refit {
        result = fit_dispersion_grid(counts, &mus, MIN_DISPERSION, max_dispersion);
    }

    result.clamp(MIN_DISPERSION, max_dispersion)
}

fn estimate_rough_dispersion(normalized_counts: RowRef<'_, f64>) -> f64 {
    const MIN_MEAN: f64 = 1.0;
    const PARAMETER_COUNT: usize = 1;
    const MIN_ALPHA: f64 = 0.0;

    let (_, sample_count) = normalized_counts.shape();

    let mus = build_mean_linear_model(normalized_counts);

    let n: f64 = mus
        .iter()
        .zip(normalized_counts.iter())
        .map(|(mean, y)| {
            let mu = mean.max(MIN_MEAN);
            (((y - mu) * (y - mu)) - mu) / (mu * mu)
        })
        .sum();

    let alpha = n / (sample_count - PARAMETER_COUNT) as f64;
    alpha.max(MIN_ALPHA)
}

fn build_mean_linear_model(normalized_counts: RowRef<'_, f64>) -> Vec<f64> {
    let mu = row_mean(normalized_counts);
    let (_, sample_count) = normalized_counts.shape();
    vec![mu; sample_count]
}

fn estimate_moments_dispersion(normalized_counts: RowRef<'_, f64>, size_factors: &[f64]) -> f64 {
    let mean = row_mean(normalized_counts);

    if mean <= 1e-10 {
        return f64::INFINITY;
    }

    let (_, sample_count) = normalized_counts.shape();

    let sum: f64 = size_factors.iter().map(|n| 1.0 / n).sum();
    let xim = sum / (sample_count as f64);

    let var = if sample_count > 1 {
        row_var(normalized_counts, mean)
    } else {
        0.0
    };

    (var - xim * mean) / (mean * mean)
}

fn build_normalized_mean_linear_model(
    normalized_counts: RowRef<'_, f64>,
    size_factors: &[f64],
) -> Vec<f64> {
    let mut mus = build_mean_linear_model(normalized_counts);

    for (mu, size_factor) in mus.iter_mut().zip(size_factors) {
        *mu *= size_factor;
    }

    mus
}

fn fit_dispersion(
    counts: RowRef<'_, f64>,
    mus: &[f64],
    alpha_init: f64,
    min_disp: f64,
    max_disp: f64,
) -> (f64, f64, f64, bool) {
    const EPSILON: f64 = 1e-4;
    const MAX_ITERATIONS: usize = 100;
    const MIN_LOG_ALPHA: f64 = -30.0;
    const MAX_LOG_ALPHA: f64 = 10.0;
    const CONVERGENCE_TOLERANCE: f64 = 1e-6;
    const KAPPA_0: f64 = 1.0;

    let min_log_alpha = (min_disp / 10.0).ln();
    let max_log_alpha = max_disp.ln();
    let mut log_alpha = alpha_init
        .max(1e-10)
        .ln()
        .clamp(MIN_LOG_ALPHA, max_log_alpha);

    let inital_lp = log_posterior(counts, mus, log_alpha);
    let mut lp = inital_lp;

    let mut dlp = d_log_posterior(counts, mus, log_alpha);

    let mut kappa = KAPPA_0;
    let mut iteration_count = 0;
    let mut accepted_count = 0;

    for _ in 0..MAX_ITERATIONS {
        iteration_count += 1;

        let a_propose = log_alpha + kappa * dlp;

        if a_propose < MIN_LOG_ALPHA {
            kappa = (MIN_LOG_ALPHA - log_alpha) / dlp;
        }

        if a_propose > MAX_LOG_ALPHA {
            kappa = (MAX_LOG_ALPHA - log_alpha) / dlp;
        }

        let theta_kappa = -log_posterior(counts, mus, log_alpha + kappa * dlp);
        let theta_hat_kappa = -lp - kappa * EPSILON * dlp * dlp;

        if theta_kappa <= theta_hat_kappa {
            accepted_count += 1;

            log_alpha += kappa * dlp;

            let lp_new = log_posterior(counts, mus, log_alpha);
            let change = lp_new - lp;

            if change < CONVERGENCE_TOLERANCE {
                lp = lp_new;
                break;
            }

            if log_alpha < min_log_alpha {
                break;
            }

            lp = lp_new;
            dlp = d_log_posterior(counts, mus, log_alpha);

            kappa = (kappa * 1.1).min(KAPPA_0);

            if accepted_count % 5 == 0 {
                kappa /= 2.0;
            }
        } else {
            kappa /= 2.0;
        }
    }

    let alpha_mle = log_alpha.exp().max(min_disp);
    let did_converge = 0 < iteration_count && iteration_count < MAX_ITERATIONS;

    (alpha_mle, inital_lp, lp, did_converge)
}

fn log_posterior(counts: RowRef<'_, f64>, mus: &[f64], log_alpha: f64) -> f64 {
    let (_, n) = counts.shape();

    let alpha = log_alpha.exp();
    let alpha_inv = 1.0 / alpha;

    let mut ll_part = 0.0;
    let mut weights = Diag::zeros(n);

    for (i, (y, mu)) in counts.iter().zip(mus).enumerate() {
        ll_part += ln_gamma(y + alpha_inv)
            - ln_gamma(alpha_inv)
            - y * (mu + alpha_inv).ln()
            - alpha_inv * (1.0 + mu * alpha).ln();

        weights[i] = 1.0 / (1.0 / mu + alpha);
    }

    let b = weights.column_vector().sum();
    let log_det = b.ln();

    let cr_term = if log_det.is_finite() {
        -0.5 * log_det
    } else {
        0.0
    };

    ll_part + cr_term
}

fn d_log_posterior(counts: RowRef<'_, f64>, mus: &[f64], log_alpha: f64) -> f64 {
    let (_, n) = counts.shape();

    let alpha = log_alpha.exp();
    let alpha_inv = 1.0 / alpha;

    let mut ll_sum = 0.0;

    for i in 0..n {
        let y = counts[i];
        let mu = mus[i].max(1e-10);

        ll_sum += digamma(alpha_inv) + (1.0 + mu * alpha).ln()
            - mu * alpha * (1.0 / (1.0 + mu * alpha))
            - digamma(y + (alpha_inv))
            + y * (1.0 / (mu + (alpha_inv)));
    }

    let ll_part = ll_sum / (alpha * alpha);

    let mut b = 0.0;
    let mut db = 0.0;

    for m in mus {
        let mu = m.max(1e-10);
        let den = 1.0 / mu + alpha;
        b += 1.0 / den;
        db += -1.0 / (den * den);
    }

    let trace = db / b;
    let cr_term = -0.5 * trace;

    (ll_part + cr_term) * alpha
}

fn fit_dispersion_grid(
    counts: RowRef<'_, f64>,
    mus: &[f64],
    min_dispersion: f64,
    max_dispersion: f64,
) -> f64 {
    let min_log_alpha = min_dispersion.ln();
    let max_log_alpha = max_dispersion.ln();

    let (best_course_log_alpha, delta) = grid_search(counts, mus, min_log_alpha, max_log_alpha);
    let fine_start = best_course_log_alpha - delta;
    let fine_end = best_course_log_alpha + delta;

    let (best_fine_log_alpha, _) = grid_search(counts, mus, fine_start, fine_end);

    best_fine_log_alpha.exp()
}

fn grid_search(counts: RowRef<'_, f64>, mus: &[f64], start: f64, end: f64) -> (f64, f64) {
    const GRID_SIZE: usize = 20;

    let (grid, delta) = linspace(start, end, GRID_SIZE);
    let mut lps = vec![0.0; GRID_SIZE];

    for (lp, &log_alpha) in lps.iter_mut().zip(&grid) {
        *lp = log_posterior(counts, mus, log_alpha);
    }

    let i = argmax(&lps);
    (grid[i], delta)
}

fn linspace(start: f64, end: f64, n: usize) -> (Vec<f64>, f64) {
    assert!(n > 1);

    let delta = (end - start) / ((n - 1) as f64);
    let grid = (0..n).map(|i| start + (i as f64) * delta).collect();

    (grid, delta)
}

fn argmax(values: &[f64]) -> usize {
    assert!(!values.is_empty());

    let mut max_i = 0;
    let mut max_value = values[0];

    for (i, &n) in values.iter().enumerate().skip(1) {
        if n > max_value {
            max_i = i;
            max_value = n;
        }
    }

    max_i
}

fn estimate_dispersions_fit(normalized_counts: MatRef<'_, f64>, dispersions: &[f64]) -> (f64, f64) {
    let (feature_count, _) = normalized_counts.shape();

    let mut means = Col::zeros(feature_count);
    col_mean(means.as_mut(), normalized_counts, NanHandling::Propagate);

    parametric_dispersion_fit(means.as_ref(), dispersions)
}

fn parametric_dispersion_fit(means: ColRef<'_, f64>, dispersions: &[f64]) -> (f64, f64) {
    let feature_count = dispersions.len();

    let mut filtered_means = Vec::with_capacity(feature_count);
    let mut filtered_dispersions = Vec::with_capacity(feature_count);

    for (&m, &d) in means.iter().zip(dispersions) {
        if m > 0.0 && d > 1e-6 && d.is_finite() {
            filtered_means.push(m);
            filtered_dispersions.push(d);
        }
    }

    assert!(filtered_dispersions.len() >= 3);

    find_parametric_coeffs(&filtered_means, &filtered_dispersions)
}

fn find_parametric_coeffs(means: &[f64], dispersions: &[f64]) -> (f64, f64) {
    const MIN_RESIDUAL: f64 = 1e-4;
    const MAX_RESIDUAL: f64 = 15.0;

    const MAX_ITERATIONS: usize = 11;
    const TOL: f64 = 1e-6;

    let (mut a0, mut a1) = (0.1, 1.0);

    for _ in 0..MAX_ITERATIONS {
        let (prev_a0, prev_a1) = (a0, a1);

        let mut filtered_means = Vec::with_capacity(means.len());
        let mut filtered_dispersions = Vec::with_capacity(dispersions.len());

        for (&mean, &disp) in means.iter().zip(dispersions) {
            let residual = disp / (a0 + a1 / mean);

            if MIN_RESIDUAL < residual && residual < MAX_RESIDUAL {
                filtered_means.push(mean);
                filtered_dispersions.push(disp);
            }
        }

        let (next_a0, next_a1, did_converge) =
            fit_gamma_glm_identity(&filtered_means, &filtered_dispersions, a0, a1);

        a0 = next_a0;
        a1 = next_a1;

        if a0 <= 0.0 || a1 <= 0.0 {
            panic!("unexpected negative coeffs");
        }

        let log_change = (a0 / prev_a0).ln().powi(2) + (a1 / prev_a1).ln().powi(2);

        if log_change < TOL && did_converge {
            return (a0, a1);
        }
    }

    panic!("unexpected nonconvergence")
}

fn fit_gamma_glm_identity(
    means: &[f64],
    dispersions: &[f64],
    mut a0: f64,
    mut a1: f64,
) -> (f64, f64, bool) {
    const MAX_ITERATIONS: usize = 100;
    const MIN_DEVIANCE_DELTA: f64 = 1e-8;

    fn deviance(means: &[f64], dispersions: &[f64], a0: f64, a1: f64) -> f64 {
        let mut dev = 0.0;

        for (mean, x) in means.iter().zip(dispersions) {
            let mu = a0 + a1 / mean;
            dev += (x / mu).ln() - ((x - mu) / mu);
        }

        -2.0 * dev
    }

    let feature_count = dispersions.len();

    let means_inv = Mat::from_fn(feature_count, 1, |i, _| 1.0 / means[i]);

    let x = Mat::from_fn(feature_count, 2, |i, j| {
        if j == 0 { 1.0 } else { means_inv[(i, 0)] }
    });

    let mut w = Diag::zeros(feature_count);
    let y = MatRef::from_column_major_slice(dispersions, feature_count, 1);

    let mut prev_dev = deviance(means, dispersions, a0, a1);
    let mut did_converge = false;

    for _ in 0..MAX_ITERATIONS {
        for (i, mean) in means.iter().enumerate() {
            let mu = a0 + a1 / mean;
            w[i] = 1.0 / (mu * mu);
        }

        let xp = x.transpose() * &w * &x;
        let beta = xp.partial_piv_lu().solve(x.transpose() * &w * y);
        a0 = beta[(0, 0)];
        a1 = beta[(1, 0)];

        let dev = deviance(means, dispersions, a0, a1);
        let dev_delta = (prev_dev - dev).abs() / dev.abs();

        if dev_delta < MIN_DEVIANCE_DELTA {
            did_converge = true;
            break;
        }

        prev_dev = dev;
    }

    (a0, a1, did_converge)
}

fn stabilize_counts(normalized_counts: MatRef<'_, f64>, a0: f64, a1: f64) -> Mat<f64> {
    normalized_counts.map(|q| {
        let a = 1.0 + a1 + 2.0 * a0 * q + 2.0 * (a0 * q * (1.0 + a1 + a0 * q)).sqrt();
        let b = 4.0 * a0;
        (a / b).ln() / LN_2
    })
}

fn row_mean(row: RowRef<'_, f64>) -> f64 {
    let sum: f64 = row.iter().sum();
    let (_, n) = row.shape();
    sum / (n as f64)
}

// sample variance
fn row_var(row: RowRef<'_, f64>, mean: f64) -> f64 {
    let a: f64 = row.iter().map(|n| (n - mean).powi(2)).sum();

    let (_, n) = row.shape();
    let b = (n - 1) as f64;

    a / b
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

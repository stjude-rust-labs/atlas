use std::{
    collections::BTreeSet,
    f64::consts::{LN_2, SQRT_2},
};

use faer::{Mat, MatRef, RowRef, prelude::Solve};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use statrs::function::erf::erfc;
use thiserror::Error;

use super::transforms::vst;

const MIN_DISPERSION: f64 = 1e-8;
const MIN_SAMPLE_COUNT: usize = 10;
const MIN_MU: f64 = 0.5;
const MIN_PRIOR_VAR: f64 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeatureResult {
    pub base_mean: f64,
    pub log2_fold_change: f64,
    pub lfc_se: f64,
    pub stat: f64,
    pub pvalue: f64,
}

#[derive(Debug, Error)]
pub enum DifferentialExpressionError {
    #[error("invalid count matrix dimensions: expected {expected}, got {actual}")]
    InvalidDimensions { expected: usize, actual: usize },
    #[error("invalid group count: expected at least 2, got {0}")]
    InvalidGroupCount(usize),
    #[error("insufficient degrees of freedom: {sample_count} samples for {group_count} groups")]
    InsufficientDegreesOfFreedom {
        sample_count: usize,
        group_count: usize,
    },
    #[error("group not found: {0}")]
    GroupNotFound(String),
    #[error("reference and target groups must be distinct: {0}")]
    IdenticalContrastGroups(String),
}

pub fn analyze(
    raw_counts: &[u32],
    feature_count: usize,
    sample_count: usize,
    groups: &[String],
    reference: Option<&str>,
    target: Option<&str>,
) -> Result<Vec<FeatureResult>, DifferentialExpressionError> {
    let expected_len = feature_count * sample_count;

    if raw_counts.len() != expected_len {
        return Err(DifferentialExpressionError::InvalidDimensions {
            expected: expected_len,
            actual: raw_counts.len(),
        });
    }

    if groups.len() != sample_count {
        return Err(DifferentialExpressionError::InvalidDimensions {
            expected: sample_count,
            actual: groups.len(),
        });
    }

    let (group_ids, group_count) = encode_groups(groups, reference, target)?;

    if sample_count <= group_count {
        return Err(DifferentialExpressionError::InsufficientDegreesOfFreedom {
            sample_count,
            group_count,
        });
    }

    let raw_counts_f64: Vec<_> = raw_counts.iter().map(|&n| n as f64).collect();
    let counts = MatRef::from_row_major_slice(&raw_counts_f64, feature_count, sample_count);

    let size_factors = vst::calculate_size_factors(counts);
    let normalized_counts = vst::normalize(counts, &size_factors);

    let mut base_means = Vec::with_capacity(feature_count);
    let mut all_zero = Vec::with_capacity(feature_count);

    for i in 0..feature_count {
        let row = counts.row(i);

        let is_zero = row.iter().all(|&y| y == 0.0);
        all_zero.push(is_zero);

        let mean = if is_zero {
            0.0
        } else {
            normalized_counts.row(i).iter().sum::<f64>() / (sample_count as f64)
        };
        base_means.push(mean);
    }

    let disp_gene_est = vst::estimate_feature_dispersions(
        counts,
        normalized_counts.as_ref(),
        &size_factors,
        &group_ids,
        group_count,
    );

    let (a0, a1) = vst::estimate_dispersions_fit(normalized_counts.as_ref(), &disp_gene_est);

    let disp_fit: Vec<f64> = base_means
        .iter()
        .zip(&all_zero)
        .map(
            |(&mean, &is_zero)| {
                if is_zero { f64::NAN } else { a0 + a1 / mean }
            },
        )
        .collect();

    let (disp_prior_var, mad) = estimate_dispersions_prior_var(
        &disp_gene_est,
        &disp_fit,
        &all_zero,
        sample_count - group_count,
    );

    let max_dispersion = sample_count.max(MIN_SAMPLE_COUNT) as f64;

    let dispersions = estimate_dispersions_map(
        counts,
        normalized_counts.as_ref(),
        &size_factors,
        &group_ids,
        group_count,
        &disp_gene_est,
        &disp_fit,
        &all_zero,
        disp_prior_var,
        mad,
        max_dispersion,
    );

    let results = (0..feature_count)
        .into_par_iter()
        .map(|i| {
            if all_zero[i] {
                FeatureResult {
                    base_mean: 0.0,
                    log2_fold_change: f64::NAN,
                    lfc_se: f64::NAN,
                    stat: f64::NAN,
                    pvalue: f64::NAN,
                }
            } else {
                fit_beta_and_wald_test(
                    counts.row(i),
                    normalized_counts.row(i),
                    &size_factors,
                    &group_ids,
                    group_count,
                    base_means[i],
                    dispersions[i],
                )
            }
        })
        .collect();

    Ok(results)
}

fn encode_groups(
    groups: &[String],
    reference: Option<&str>,
    target: Option<&str>,
) -> Result<(Vec<usize>, usize), DifferentialExpressionError> {
    let unique_groups: Vec<&str> = groups
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    if unique_groups.len() < 2 {
        return Err(DifferentialExpressionError::InvalidGroupCount(
            unique_groups.len(),
        ));
    }

    let reference_name = match reference {
        Some(name) => {
            if unique_groups.contains(&name) {
                name
            } else {
                return Err(DifferentialExpressionError::GroupNotFound(name.into()));
            }
        }
        None => unique_groups[0],
    };

    let target_name = match target {
        Some(name) => {
            if unique_groups.contains(&name) {
                name
            } else {
                return Err(DifferentialExpressionError::GroupNotFound(name.into()));
            }
        }
        None => unique_groups
            .iter()
            .copied()
            .find(|&g| g != reference_name)
            .expect("unique_groups has at least 2 distinct entries"),
    };

    if reference_name == target_name {
        return Err(DifferentialExpressionError::IdenticalContrastGroups(
            reference_name.into(),
        ));
    }

    let mut group_levels = Vec::with_capacity(unique_groups.len());
    group_levels.push(reference_name);
    group_levels.push(target_name);

    for &g in &unique_groups {
        if g != reference_name && g != target_name {
            group_levels.push(g);
        }
    }

    let group_ids = groups
        .iter()
        .map(|g| {
            group_levels
                .iter()
                .position(|&name| name == g)
                .expect("group must be in ordered levels")
        })
        .collect();

    Ok((group_ids, group_levels.len()))
}

fn estimate_dispersions_prior_var(
    disp_gene_est: &[f64],
    disp_fit: &[f64],
    all_zero: &[bool],
    dof: usize,
) -> (f64, f64) {
    const MAD_SCALE: f64 = 1.4826;
    const ABOVE_MIN_THRESHOLD: f64 = MIN_DISPERSION * 100.0;

    let var_log_disp_est = trigamma((dof as f64) / 2.0);

    let mut log_residuals: Vec<f64> = disp_gene_est
        .iter()
        .zip(disp_fit)
        .zip(all_zero)
        .filter_map(|((&ge, &fit), &is_zero)| {
            if !is_zero && ge > ABOVE_MIN_THRESHOLD && fit.is_finite() && fit > 0.0 {
                Some(ge.ln() - fit.ln())
            } else {
                None
            }
        })
        .collect();

    if log_residuals.len() < 2 {
        let mad = (MIN_PRIOR_VAR + var_log_disp_est).sqrt();
        return (MIN_PRIOR_VAR, mad);
    }

    let med = vst::median(&mut log_residuals);
    let mut abs_devs: Vec<f64> = log_residuals.iter().map(|&r| (r - med).abs()).collect();
    let mad = MAD_SCALE * vst::median(&mut abs_devs);

    let disp_prior_var = (mad * mad - var_log_disp_est).max(MIN_PRIOR_VAR);
    (disp_prior_var, mad)
}

#[allow(clippy::too_many_arguments)]
fn estimate_dispersions_map(
    counts: MatRef<'_, f64>,
    normalized_counts: MatRef<'_, f64>,
    size_factors: &[f64],
    group_ids: &[usize],
    group_count: usize,
    disp_gene_est: &[f64],
    disp_fit: &[f64],
    all_zero: &[bool],
    disp_prior_var: f64,
    mad: f64,
    max_dispersion: f64,
) -> Vec<f64> {
    const OUTLIER_SD: f64 = 2.0;

    let (feature_count, _) = counts.shape();

    (0..feature_count)
        .into_par_iter()
        .map(|i| {
            if all_zero[i] {
                return f64::NAN;
            }

            let row = counts.row(i);
            let normalized_row = normalized_counts.row(i);
            let ge = disp_gene_est[i];
            let fit = disp_fit[i];
            let log_fit = fit.ln();

            let mut mus = vst::build_normalized_mean_linear_model(
                normalized_row,
                size_factors,
                group_ids,
                group_count,
            );

            for m in &mut mus {
                *m = m.max(MIN_MU);
            }

            let alpha_init = if ge > MIN_DISPERSION * 10.0 { ge } else { fit };
            let prior = Some((log_fit, disp_prior_var));

            let (alpha_map, _, _, did_converge) = vst::fit_dispersion(
                row,
                &mus,
                group_ids,
                group_count,
                alpha_init,
                MIN_DISPERSION,
                max_dispersion,
                prior,
            );

            let mut disp_map = if did_converge {
                alpha_map
            } else {
                vst::fit_dispersion_grid(
                    row,
                    &mus,
                    group_ids,
                    group_count,
                    MIN_DISPERSION,
                    max_dispersion,
                    prior,
                )
            };

            disp_map = disp_map.clamp(MIN_DISPERSION, max_dispersion);

            if ge.ln() > log_fit + OUTLIER_SD * mad {
                ge
            } else {
                disp_map
            }
        })
        .collect()
}

fn fit_beta_and_wald_test(
    counts: RowRef<'_, f64>,
    normalized_counts: RowRef<'_, f64>,
    size_factors: &[f64],
    group_ids: &[usize],
    group_count: usize,
    base_mean: f64,
    alpha: f64,
) -> FeatureResult {
    const MAX_ITERATIONS: usize = 100;
    const DEV_TOL: f64 = 1e-8;
    const MAX_ETA: f64 = 30.0;
    const PSEUDOCOUNT: f64 = 0.1;

    let lambda = 1e-6 / (LN_2 * LN_2);
    let (_, sample_count) = counts.shape();

    let mut log_sums = vec![0.0; group_count];
    let mut group_sizes = vec![0usize; group_count];

    for (&y_norm, &g) in normalized_counts.iter().zip(group_ids) {
        log_sums[g] += (y_norm + PSEUDOCOUNT).ln();
        group_sizes[g] += 1;
    }

    let mut beta = vec![0.0; group_count];
    beta[0] = log_sums[0] / (group_sizes[0] as f64);

    for k in 1..group_count {
        beta[k] = log_sums[k] / (group_sizes[k] as f64) - beta[0];
    }

    let mut w_group = vec![0.0; group_count];
    let mut wz_group = vec![0.0; group_count];
    let mut mus = vec![0.0; sample_count];
    let mut dev_old = 0.0;

    for iter in 0..MAX_ITERATIONS {
        w_group.fill(0.0);
        wz_group.fill(0.0);

        for j in 0..sample_count {
            let g = group_ids[j];
            let eta = if g == 0 { beta[0] } else { beta[0] + beta[g] }.clamp(-MAX_ETA, MAX_ETA);
            let mu = (size_factors[j] * eta.exp()).max(MIN_MU);
            let w = mu / (1.0 + alpha * mu);
            let z = (mu / size_factors[j]).ln() + (counts[j] - mu) / mu;

            w_group[g] += w;
            wz_group[g] += w * z;
        }

        let (xtwx, rhs) = build_normal_equations(&w_group, &wz_group);
        let mut h = xtwx;
        for k in 0..group_count {
            h[(k, k)] += lambda;
        }

        let beta_mat = h.partial_piv_lu().solve(&rhs);
        for k in 0..group_count {
            beta[k] = beta_mat[(k, 0)];
        }

        for j in 0..sample_count {
            let g = group_ids[j];
            let eta = if g == 0 { beta[0] } else { beta[0] + beta[g] }.clamp(-MAX_ETA, MAX_ETA);
            mus[j] = (size_factors[j] * eta.exp()).max(MIN_MU);
        }

        let dev = nb_deviance(counts, &mus, alpha);
        if iter > 0 && (dev - dev_old).abs() / (dev.abs() + 0.1) < DEV_TOL {
            break;
        }
        dev_old = dev;
    }

    w_group.fill(0.0);

    for j in 0..sample_count {
        let g = group_ids[j];
        let eta = if g == 0 { beta[0] } else { beta[0] + beta[g] }.clamp(-MAX_ETA, MAX_ETA);
        let mu = (size_factors[j] * eta.exp()).max(MIN_MU);
        w_group[g] += mu / (1.0 + alpha * mu);
    }

    let xtwx = build_xtwx(&w_group);
    let mut h = xtwx.clone();
    for k in 0..group_count {
        h[(k, k)] += lambda;
    }

    let h_inv = h
        .partial_piv_lu()
        .solve(Mat::<f64>::identity(group_count, group_count));

    let cov = &h_inv * &xtwx * &h_inv;

    let log2_fold_change = beta[1] / LN_2;
    let lfc_se = cov[(1, 1)].max(0.0).sqrt() / LN_2;
    let stat = log2_fold_change / lfc_se;
    let pvalue = erfc(stat.abs() / SQRT_2);

    FeatureResult {
        base_mean,
        log2_fold_change,
        lfc_se,
        stat,
        pvalue,
    }
}

fn build_xtwx(w_group: &[f64]) -> Mat<f64> {
    let p = w_group.len();
    let mut xtwx = Mat::zeros(p, p);
    xtwx[(0, 0)] = w_group.iter().sum();

    for (k, &w) in w_group.iter().enumerate().skip(1) {
        xtwx[(0, k)] = w;
        xtwx[(k, 0)] = w;
        xtwx[(k, k)] = w;
    }

    xtwx
}

fn build_normal_equations(w_group: &[f64], wz_group: &[f64]) -> (Mat<f64>, Mat<f64>) {
    let p = w_group.len();
    let xtwx = build_xtwx(w_group);
    let mut rhs = Mat::zeros(p, 1);
    rhs[(0, 0)] = wz_group.iter().sum();

    for (k, &wz) in wz_group.iter().enumerate().skip(1) {
        rhs[(k, 0)] = wz;
    }

    (xtwx, rhs)
}

fn nb_deviance(counts: RowRef<'_, f64>, mus: &[f64], alpha: f64) -> f64 {
    let alpha_inv = 1.0 / alpha;
    let mut dev = 0.0;

    for (&y, &mu) in counts.iter().zip(mus) {
        if y > 0.0 {
            dev += y * (y / mu).ln();
        }

        dev -= (y + alpha_inv) * ((1.0 + alpha * y) / (1.0 + alpha * mu)).ln();
    }

    2.0 * dev
}

// See <https://en.wikipedia.org/wiki/Trigamma_function#Calculation>.
fn trigamma(mut x: f64) -> f64 {
    let mut result = 0.0;

    while x < 8.0 {
        result += 1.0 / (x * x);
        x += 1.0;
    }

    let inv = 1.0 / x;
    let inv2 = inv * inv;

    result
        + inv
        + 0.5 * inv2
        + inv
            * inv2
            * (1.0 / 6.0
                - inv2
                    * (1.0 / 30.0
                        - inv2 * (1.0 / 42.0 - inv2 * (1.0 / 30.0 - inv2 * (5.0 / 66.0)))))
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    #[test]
    fn test_trigamma() {
        assert!((trigamma(1.0) - PI * PI / 6.0).abs() < 1e-12);
        assert!((trigamma(0.5) - PI * PI / 2.0).abs() < 1e-12);

        let trigamma_5 = PI * PI / 6.0 - 1.0 - 0.25 - 1.0 / 9.0 - 1.0 / 16.0;
        assert!((trigamma(5.0) - trigamma_5).abs() < 1e-12);
    }
}

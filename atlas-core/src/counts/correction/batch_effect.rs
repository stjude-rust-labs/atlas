use faer::{MatMut, MatRef, RowRef, linalg::solvers::SolveLstsq};
use indexmap::IndexSet;

pub fn correct_raw(
    counts: &[f64],
    feature_count: usize,
    sample_count: usize,
    groups: &[String],
) -> Vec<f64> {
    let counts_mat = MatRef::from_row_major_slice(counts, feature_count, sample_count);
    correct(counts_mat, groups)
}

pub fn correct(counts: MatRef<'_, f64>, groups: &[String]) -> Vec<f64> {
    let (feature_count, sample_count) = counts.shape();

    let group_names: IndexSet<_> = groups.iter().collect();
    let group_count = group_names.len();

    let mut group_ids = Vec::with_capacity(sample_count);
    let mut group_indices = vec![Vec::new(); group_count];

    for (i, g) in groups.iter().enumerate() {
        let j = group_names.get_index_of(g).unwrap();
        group_ids.push(j);
        group_indices[j].push(i);
    }

    let mut zero_rows = vec![false; feature_count];
    let mut zero_row_count = 0;

    for (i, row) in counts.row_iter().enumerate() {
        for is in &group_indices {
            if group_row_var_with_indices(row, is) == 0.0 {
                zero_rows[i] = true;
                zero_row_count += 1;
                break;
            }
        }
    }

    let filtered_counts = if zero_row_count == 0 {
        counts.cloned()
    } else {
        let mut filtered_rows: Vec<f64> = Vec::new();

        for (i, row) in counts.row_iter().enumerate() {
            if zero_rows[i] {
                continue;
            } else {
                filtered_rows.extend(row.iter());
            }
        }

        let filtered_feature_count = feature_count - zero_row_count;

        MatRef::from_row_major_slice(&filtered_rows, filtered_feature_count, sample_count)
            .to_owned()
    };

    let (filtered_feature_count, _) = filtered_counts.shape();

    let mut raw_design = vec![0.0; sample_count * group_count];

    for (i, id) in group_ids.iter().enumerate() {
        raw_design[i * group_count + id] = 1.0;
    }

    let design = MatRef::from_row_major_slice(&raw_design, sample_count, group_count);

    let raw_weights: Vec<_> = group_indices
        .iter()
        .map(|is| (is.len() as f64) / (sample_count as f64))
        .collect();

    let weights = MatRef::from_row_major_slice(&raw_weights, group_count, 1);
    let b_hat = design.qr().solve_lstsq(filtered_counts.transpose());
    let grand_mean = weights.transpose() * &b_hat;

    let x = design * b_hat;
    let residuals = &filtered_counts - x.transpose();

    let pooled_var: Vec<_> = residuals
        .row_iter()
        .map(|row| row.iter().map(|n| n * n).sum::<f64>() / (sample_count as f64))
        .collect();

    let mut standardized_counts = filtered_counts;

    for ((row, mean), var) in standardized_counts
        .row_iter_mut()
        .zip(grand_mean.row(0).iter())
        .zip(&pooled_var)
    {
        for n in row.iter_mut() {
            *n = (*n - mean) / var.sqrt();
        }
    }

    let mut group_vars = Vec::new();

    for is in &group_indices {
        for row in standardized_counts.row_iter() {
            let var = group_row_var_with_indices(row, is);
            group_vars.push(var);
        }
    }

    let delta_hat = MatRef::from_row_major_slice(&group_vars, group_count, filtered_feature_count);

    let a_prior: Vec<_> = delta_hat.row_iter().map(row_a_prior).collect();
    let b_prior: Vec<_> = delta_hat.row_iter().map(row_b_prior).collect();

    let mut gamma_bar = Vec::new();
    let mut t2 = Vec::new();

    let gamma_hat = design.qr().solve_lstsq(standardized_counts.transpose());

    for row in gamma_hat.row_iter() {
        gamma_bar.push(row_mean(row));
        t2.push(row_var(row));
    }

    let mut group_buffers = Vec::with_capacity(group_count);

    for is in &group_indices {
        let mut buf: Vec<f64> = Vec::new();

        for &i in is {
            buf.extend(standardized_counts.col(i).iter());
        }

        group_buffers.push(buf);
    }

    let mut gamma_star = vec![Vec::new(); group_count];
    let mut delta_star = vec![Vec::new(); group_count];

    for (j, is) in group_indices.iter().enumerate() {
        let group_standardized_counts =
            MatRef::from_column_major_slice(&group_buffers[j], filtered_feature_count, is.len());

        let (g_s, d_s) = solve(
            group_standardized_counts,
            gamma_hat.row(j),
            delta_hat.row(j),
            gamma_bar[j],
            t2[j],
            a_prior[j],
            b_prior[j],
        );

        gamma_star[j] = g_s;
        delta_star[j] = d_s;
    }

    for (j, is) in group_indices.iter().enumerate() {
        let group_counts = MatMut::from_column_major_slice_mut(
            &mut group_buffers[j],
            filtered_feature_count,
            is.len(),
        );

        let g_s = &gamma_star[j];
        let d_s = &delta_star[j];

        for (row, (g, d)) in group_counts.row_iter_mut().zip(g_s.iter().zip(d_s.iter())) {
            for n in row.iter_mut() {
                *n = (*n - g) / d.sqrt();
            }
        }
    }

    for (j, is) in group_indices.iter().enumerate() {
        let group_counts = MatMut::from_column_major_slice_mut(
            &mut group_buffers[j],
            filtered_feature_count,
            is.len(),
        );

        for (row, (var, mean)) in group_counts
            .row_iter_mut()
            .zip(pooled_var.iter().zip(grand_mean.row(0).iter()))
        {
            for n in row.iter_mut() {
                *n = *n * var.sqrt() + mean;
            }
        }
    }

    let group_views: Vec<_> = group_buffers
        .iter()
        .zip(&group_indices)
        .map(|(buf, is)| MatRef::from_column_major_slice(buf, filtered_feature_count, is.len()))
        .collect();

    let mut row_iters: Vec<_> = group_views.iter().map(|mat| mat.row_iter()).collect();

    let mut raw_corrected_counts = Vec::new();

    for (i, &is_zero) in zero_rows.iter().enumerate() {
        if is_zero {
            raw_corrected_counts.extend(counts.row(i).iter());
        } else {
            for iter in &mut row_iters {
                let row = iter.next().unwrap();
                raw_corrected_counts.extend(row.iter());
            }
        }
    }

    raw_corrected_counts
}

fn row_mean(row: RowRef<'_, f64>) -> f64 {
    let sum: f64 = row.iter().sum();
    let (_, n) = row.shape();
    sum / (n as f64)
}

fn row_var(row: RowRef<'_, f64>) -> f64 {
    let mean = row_mean(row);

    let mut a = 0.0;

    for n in row.iter() {
        a += (n - mean).powi(2);
    }

    let (_, n) = row.shape();
    let b = (n - 1) as f64;

    a / b
}

fn group_row_var_with_indices(row: RowRef<'_, f64>, is: &[usize]) -> f64 {
    let count = is.len();

    assert!(count > 0);

    let mut sum = 0.0;

    for &i in is {
        sum += row.get(i);
    }

    let mean = sum / (count as f64);

    let mut a = 0.0;

    for &i in is {
        let n = *row.get(i);
        a += (n - mean).powi(2);
    }

    let b = (count - 1) as f64;

    a / b
}

fn row_a_prior(gamma_hat: RowRef<'_, f64>) -> f64 {
    let m = row_mean(gamma_hat);
    let s2 = row_var(gamma_hat);
    (2.0 * s2 + m.powi(2)) / s2
}

fn row_b_prior(gamma_hat: RowRef<'_, f64>) -> f64 {
    let m = row_mean(gamma_hat);
    let s2 = row_var(gamma_hat);
    (m * s2 + m.powi(3)) / s2
}

fn solve(
    group_standardized_counts: MatRef<'_, f64>,
    g_hat: RowRef<'_, f64>,
    d_hat: RowRef<'_, f64>,
    g_bar: f64,
    t2: f64,
    a: f64,
    b: f64,
) -> (Vec<f64>, Vec<f64>) {
    const MIN_DELTA: f64 = 0.0001;

    let (feature_count, sample_count) = group_standardized_counts.shape();
    let n = vec![sample_count as f64; feature_count];

    let mut g_old: Vec<_> = g_hat.iter().copied().collect();
    let mut d_old: Vec<_> = d_hat.iter().copied().collect();

    let mut delta = 1.0;

    while delta > MIN_DELTA {
        let g_new = posterior_mean(g_hat, g_bar, &n, &d_old, t2);

        let mut residuals = group_standardized_counts.cloned();

        for col in residuals.col_iter_mut() {
            for (n, m) in col.iter_mut().zip(&g_new) {
                *n -= m;
            }
        }

        let sum2: Vec<_> = residuals
            .row_iter()
            .map(|row| row.iter().map(|n| n.powi(2)).sum())
            .collect();

        let d_new = posterior_var(&sum2, &n, a, b);

        let rel_g = calculate_relative_change(&g_new, &g_old);
        let rel_d = calculate_relative_change(&d_new, &d_old);

        let max_g = maxf(&rel_g);
        let max_d = maxf(&rel_d);
        delta = max_g.max(max_d);

        g_old = g_new;
        d_old = d_new;
    }

    (g_old, d_old)
}

fn posterior_mean(
    g_hat: RowRef<'_, f64>,
    g_bar: f64,
    n: &[f64],
    d_old: &[f64],
    t2: f64,
) -> Vec<f64> {
    n.iter()
        .zip(g_hat.iter())
        .zip(d_old)
        .map(|((m, g), d)| (t2 * m * g + d * g_bar) / (t2 * m + d))
        .collect()
}

fn posterior_var(sum2: &[f64], n: &[f64], a: f64, b: f64) -> Vec<f64> {
    n.iter()
        .zip(sum2)
        .map(|(m, s)| (0.5 * s + b) / (m / 2.0 + a - 1.0))
        .collect()
}

fn calculate_relative_change(new: &[f64], old: &[f64]) -> Vec<f64> {
    new.iter()
        .zip(old)
        .map(|(n, o)| (n - o).abs() / o)
        .collect()
}

fn maxf(values: &[f64]) -> f64 {
    assert!(!values.is_empty());

    let mut max = values[0];

    for n in values {
        max = n.max(max);
    }

    max
}

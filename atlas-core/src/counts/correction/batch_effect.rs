use std::collections::HashSet;

use faer::{MatRef, RowRef, linalg::solvers::SolveLstsq};
use indexmap::IndexSet;

pub fn correct(counts: MatRef<'_, f64>, groups: &[String]) -> Vec<f64> {
    let (feature_count, sample_count) = counts.shape();

    let group_names: IndexSet<_> = groups.iter().collect();
    let mut zero_rows = HashSet::new();

    for (i, row) in counts.row_iter().enumerate() {
        for group_name in &group_names {
            let var = group_row_var(row, groups, group_name);

            if var == 0.0 {
                zero_rows.insert(i);
            }
        }
    }

    let filtered_counts = if zero_rows.is_empty() {
        counts.cloned()
    } else {
        let mut filtered_rows: Vec<f64> = Vec::new();

        for (i, row) in counts.row_iter().enumerate() {
            if zero_rows.contains(&i) {
                continue;
            } else {
                filtered_rows.extend(row.iter());
            }
        }

        let filtered_feature_count = feature_count - zero_rows.len();

        MatRef::from_row_major_slice(&filtered_rows, filtered_feature_count, sample_count)
            .to_owned()
    };

    let group_count = group_names.len();
    let mut raw_design = Vec::with_capacity(sample_count);

    for g in groups {
        let mut row = vec![0.0; group_count];
        let i = group_names.get_index_of(g).unwrap();
        row[i] = 1.0;
        raw_design.extend(row);
    }

    let design = MatRef::from_row_major_slice(&raw_design, sample_count, group_count);

    let group_frequency_table = build_group_frequency_table(groups, &group_names);
    let group_frequencies_sum: u32 = group_frequency_table.iter().sum();

    let raw_weights: Vec<_> = group_frequency_table
        .iter()
        .map(|n| (*n as f64) / (group_frequencies_sum as f64))
        .collect();

    let weights = MatRef::from_row_major_slice(&raw_weights, group_count, 1);
    let b_hat = design.qr().solve_lstsq(filtered_counts.transpose());
    let _grand_mean = weights.transpose() * &b_hat;

    let x = design * b_hat;
    let residuals = &filtered_counts - x.transpose();

    let rr = residuals.map(|n| n * n);
    let _pooled_var: Vec<f64> = rr.row_iter().map(row_mean).collect();

    todo!()
}

fn row_mean(row: RowRef<'_, f64>) -> f64 {
    let sum: f64 = row.iter().sum();
    let (_, n) = row.shape();
    sum / (n as f64)
}

fn group_row_var(row: RowRef<'_, f64>, groups: &[String], group_name: &str) -> f64 {
    let mut sum = 0.0;
    let mut count = 0;

    for (n, g) in row.iter().zip(groups) {
        if g == group_name {
            sum += n;
            count += 1;
        }
    }

    assert!(count > 0);

    let mean = sum / (count as f64);

    let mut a = 0.0;

    for (n, g) in row.iter().zip(groups) {
        if g == group_name {
            a += (n - mean).powi(2);
        }
    }

    let b = (count - 1) as f64;

    a / b
}

fn build_group_frequency_table(groups: &[String], group_names: &IndexSet<&String>) -> Vec<u32> {
    let mut table = vec![0; group_names.len()];

    for g in groups {
        let i = group_names.get_index_of(g).unwrap();
        table[i] += 1;
    }

    table
}

use std::collections::HashSet;

use faer::{MatRef, RowRef};
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

    let _filtered_counts = if zero_rows.is_empty() {
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

    todo!()
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

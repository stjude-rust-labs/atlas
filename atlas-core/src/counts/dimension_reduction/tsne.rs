pub fn transform(perplexity: f64, theta: f64, counts: Vec<i32>, feature_count: usize) -> Vec<f64> {
    fn euclidean_distance(a: &&[f64], b: &&[f64]) -> f64 {
        a.iter()
            .zip(b.iter())
            .map(|(p, q)| (p - q).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    let sum: u64 = counts.iter().map(|n| *n as u64).sum();

    let normalized_counts: Vec<_> = counts
        .into_iter()
        .map(|count| (count as f64) / (sum as f64))
        .collect();

    let data: Vec<_> = normalized_counts.chunks(feature_count).collect();

    bhtsne::tSNE::new(&data)
        .perplexity(perplexity)
        .barnes_hut(theta, euclidean_distance)
        .embedding()
}

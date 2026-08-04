pub fn transform(perplexity: f64, theta: f64, counts: Vec<f64>, feature_count: usize) -> Vec<f64> {
    fn euclidean_distance(a: &&[f64], b: &&[f64]) -> f64 {
        a.iter()
            .zip(b.iter())
            .map(|(p, q)| (p - q).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    let sum: f64 = counts.iter().sum();
    let normalized_counts: Vec<_> = counts.into_iter().map(|count| count / sum).collect();
    let data: Vec<_> = normalized_counts.chunks(feature_count).collect();

    bhtsne::tSNE::<f64, &[f64], 2>::new(&data)
        .perplexity(perplexity)
        .barnes_hut(theta, euclidean_distance)
        .embedding()
}

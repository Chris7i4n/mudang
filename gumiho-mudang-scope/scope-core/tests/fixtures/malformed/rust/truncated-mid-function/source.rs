pub struct OrderProcessor {
    pub total: i64,
}

impl OrderProcessor {
    pub fn compute_total(&self, items: &[i64]) -> i64 {
        let mut sum = 0i64;
        for item in items {
            sum +=

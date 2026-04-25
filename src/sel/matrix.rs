//! Minimal matrix math for NL models.
//!
//! COULD MOVE TO MIRROR: Nothing. Pure math, no domain semantics.
//! STAYS IN SPECTRAL: Runtime computation. Mirror proves, spectral runs.
//!
//! No external linear algebra crate. These are tiny matrices (100x32, 32x50).
//! Pure Rust. The whole thing fits in L1 cache.

// Matmul loops over fixed-size arrays — iterator style obscures the linear algebra.
#![allow(clippy::needless_range_loop)]

/// Matrix multiply: output = weights * input + bias.
///
/// `weights` is row-major, shape (out_dim x in_dim).
/// `input` has length in_dim.
/// `bias` has length out_dim.
/// Returns vector of length out_dim.
pub fn matmul(weights: &[f64], input: &[f64], bias: &[f64], out_dim: usize, in_dim: usize) -> Vec<f64> {
    assert_eq!(weights.len(), out_dim * in_dim);
    assert_eq!(input.len(), in_dim);
    assert_eq!(bias.len(), out_dim);

    let mut output = vec![0.0; out_dim];
    for i in 0..out_dim {
        let mut sum = bias[i];
        for j in 0..in_dim {
            sum += weights[i * in_dim + j] * input[j];
        }
        output[i] = sum;
    }
    output
}

/// Element-wise ReLU: max(0, x).
pub fn relu(v: &[f64]) -> Vec<f64> {
    v.iter().map(|&x| x.max(0.0)).collect()
}

/// Softmax over a vector. Numerically stable (subtract max first).
pub fn softmax(logits: &[f64]) -> Vec<f64> {
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = logits.iter().map(|&x| (x - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

/// Index of the maximum element. Returns 0 for empty slices.
pub fn argmax(v: &[f64]) -> usize {
    if v.is_empty() {
        return 0;
    }
    let mut best_idx = 0;
    let mut best_val = v[0];
    for i in 1..v.len() {
        if v[i] > best_val {
            best_val = v[i];
            best_idx = i;
        }
    }
    best_idx
}

/// Cross-entropy gradient: softmax output - one-hot target.
///
/// `predicted` is the softmax output.
/// `target_idx` is the index of the correct class.
/// Returns gradient vector (same length as predicted).
pub fn cross_entropy_gradient(predicted: &[f64], target_idx: usize) -> Vec<f64> {
    let mut grad = predicted.to_vec();
    grad[target_idx] -= 1.0;
    grad
}

/// Element-wise subtraction: a - b.
pub fn matrix_subtract(a: &[f64], b: &[f64]) -> Vec<f64> {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b.iter()).map(|(&x, &y)| x - y).collect()
}

/// Scalar multiplication: a * scalar.
pub fn matrix_scale(a: &[f64], scalar: f64) -> Vec<f64> {
    a.iter().map(|&x| x * scalar).collect()
}

/// Outer product: a (len m) x b (len n) -> matrix (m x n), row-major.
pub fn outer_product(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut result = vec![0.0; a.len() * b.len()];
    for i in 0..a.len() {
        for j in 0..b.len() {
            result[i * b.len() + j] = a[i] * b[j];
        }
    }
    result
}

/// Simple LCG RNG for weight initialization. No external dependency.
pub struct SimpleRng(pub u64);

impl SimpleRng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Box-Muller transform for normal distribution.
    pub fn next_normal(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-10);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matmul_identity() {
        // 2x2 identity matrix * [3.0, 4.0] + [0.0, 0.0] = [3.0, 4.0]
        let weights = vec![1.0, 0.0, 0.0, 1.0];
        let input = vec![3.0, 4.0];
        let bias = vec![0.0, 0.0];
        let result = matmul(&weights, &input, &bias, 2, 2);
        assert_eq!(result, vec![3.0, 4.0]);
    }

    #[test]
    fn matmul_with_bias() {
        let weights = vec![1.0, 0.0, 0.0, 1.0];
        let input = vec![3.0, 4.0];
        let bias = vec![1.0, 2.0];
        let result = matmul(&weights, &input, &bias, 2, 2);
        assert_eq!(result, vec![4.0, 6.0]);
    }

    #[test]
    fn matmul_non_square() {
        // 3x2 matrix * [1.0, 2.0] + [0,0,0]
        let weights = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let input = vec![1.0, 2.0];
        let bias = vec![0.0, 0.0, 0.0];
        let result = matmul(&weights, &input, &bias, 3, 2);
        assert_eq!(result, vec![5.0, 11.0, 17.0]);
    }

    #[test]
    fn relu_positive_and_negative() {
        let v = vec![-1.0, 0.0, 1.0, -0.5, 2.0];
        let result = relu(&v);
        assert_eq!(result, vec![0.0, 0.0, 1.0, 0.0, 2.0]);
    }

    #[test]
    fn softmax_sums_to_one() {
        let logits = vec![1.0, 2.0, 3.0];
        let probs = softmax(&logits);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn softmax_max_has_highest_prob() {
        let logits = vec![0.0, 0.0, 100.0, 0.0];
        let probs = softmax(&logits);
        assert!(probs[2] > 0.99);
    }

    #[test]
    fn softmax_uniform_on_equal() {
        let logits = vec![1.0, 1.0, 1.0];
        let probs = softmax(&logits);
        for &p in &probs {
            assert!((p - 1.0 / 3.0).abs() < 1e-10);
        }
    }

    #[test]
    fn argmax_basic() {
        assert_eq!(argmax(&[1.0, 3.0, 2.0]), 1);
        assert_eq!(argmax(&[5.0, 1.0, 2.0]), 0);
        assert_eq!(argmax(&[1.0, 2.0, 5.0]), 2);
    }

    #[test]
    fn argmax_empty() {
        assert_eq!(argmax(&[]), 0);
    }

    #[test]
    fn cross_entropy_gradient_shape() {
        let predicted = vec![0.7, 0.2, 0.1];
        let grad = cross_entropy_gradient(&predicted, 0);
        assert_eq!(grad.len(), 3);
        // grad[0] = 0.7 - 1.0 = -0.3
        assert!((grad[0] - (-0.3)).abs() < 1e-10);
        // grad[1] = 0.2
        assert!((grad[1] - 0.2).abs() < 1e-10);
    }

    #[test]
    fn matrix_subtract_basic() {
        let a = vec![3.0, 5.0, 7.0];
        let b = vec![1.0, 2.0, 3.0];
        let result = matrix_subtract(&a, &b);
        assert_eq!(result, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn matrix_scale_basic() {
        let a = vec![1.0, 2.0, 3.0];
        let result = matrix_scale(&a, 2.0);
        assert_eq!(result, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn outer_product_basic() {
        let a = vec![1.0, 2.0];
        let b = vec![3.0, 4.0, 5.0];
        let result = outer_product(&a, &b);
        assert_eq!(result, vec![3.0, 4.0, 5.0, 6.0, 8.0, 10.0]);
    }

    #[test]
    fn rng_deterministic() {
        let mut r1 = SimpleRng(42);
        let mut r2 = SimpleRng(42);
        for _ in 0..10 {
            assert_eq!(r1.next_u64(), r2.next_u64());
        }
    }

    #[test]
    fn rng_normal_distribution() {
        let mut rng = SimpleRng(42);
        let mut sum = 0.0;
        let n = 1000;
        for _ in 0..n {
            sum += rng.next_normal();
        }
        let mean = sum / n as f64;
        // Mean should be near zero for a normal distribution
        assert!(mean.abs() < 0.2, "mean was {}", mean);
    }
}

fn mat_mult(a: &Vec<Vec<f64>>, b: &Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    let rows = a.len();
    let cols = b[0].len();
    let mut result = vec![vec![0.0; cols]; rows];

    for i in 0..rows {
        for j in 0..cols {
            for k in 0..b.len() {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}

/// Transpose a matrix
fn transpose(mat: &Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    let rows = mat.len();
    let cols = mat[0].len();
    let mut transposed = vec![vec![0.0; rows]; cols];

    for i in 0..rows {
        for j in 0..cols {
            transposed[j][i] = mat[i][j];
        }
    }
    transposed
}

/// Identity matrix generator (for inversion)
fn identity_matrix(size: usize) -> Vec<Vec<f64>> {
    let mut identity = vec![vec![0.0; size]; size];
    for i in 0..size {
        identity[i][i] = 1.0;
    }
    identity
}

/// Invert a matrix using Gaussian elimination
fn invert_matrix(matrix: &Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    let n = matrix.len();
    let mut a = matrix.clone();
    let mut inv = identity_matrix(n);

    for i in 0..n {
        let mut max_row = i;
        for k in i+1..n {
            if a[k][i].abs() > a[max_row][i].abs() {
                max_row = k;
            }
        }
        a.swap(i, max_row);
        inv.swap(i, max_row);

        let diag = a[i][i];
        for j in 0..n {
            a[i][j] /= diag;
            inv[i][j] /= diag;
        }

        for k in 0..n {
            if k != i {
                let factor = a[k][i];
                for j in 0..n {
                    a[k][j] -= factor * a[i][j];
                    inv[k][j] -= factor * inv[i][j];
                }
            }
        }
    }
    inv
}

/// Black-Litterman Model Implementation
fn black_litterman(
    sigma: &Vec<Vec<f64>>, // Covariance matrix (Σ)
    market_weights: &Vec<f64>, // Market capitalization weights (w_m)
    tau: f64, // Small scaling factor
    p: &Vec<Vec<f64>>, // Views matrix (P)
    q: &Vec<f64>, // Views vector (Q)
    omega: &Vec<Vec<f64>> // Uncertainty matrix (Ω)
) -> Vec<f64> {
    let pi = mat_mult(&vec![market_weights.clone()], &mat_mult(&sigma, &vec![vec![tau]; sigma.len()]))[0];

    let tau_sigma_inv = invert_matrix(&mat_mult(&sigma, &vec![vec![tau]; sigma.len()]));
    let omega_inv = invert_matrix(omega);

    let pt_omega_inv = mat_mult(&transpose(p), &omega_inv);
    let posterior_cov = invert_matrix(&mat_mult(&pt_omega_inv, p));

    let posterior_mean = mat_mult(&posterior_cov, &vec![
        mat_mult(&tau_sigma_inv, &vec![pi.clone()])[0]
            .iter()
            .zip(mat_mult(&pt_omega_inv, &vec![q.clone()])[0].iter())
            .map(|(x, y)| x + y)
            .collect()
    ])[0];

    posterior_mean
}
// Matrix multiplication
fn mat_mult(a: &[Vec<f64>], b: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    // Handle empty matrices
    if a.is_empty() || b.is_empty() || a[0].is_empty() || b[0].is_empty() {
        eprintln!("Empty matrix in multiplication");
        return None;
    }
    
    let rows = a.len();
    let cols = b[0].len();
    let a_cols = a[0].len();
    let b_rows = b.len();
    
    // Validate dimensions
    if a_cols != b_rows {
        eprintln!("Matrix dimensions don't match for multiplication: {} != {}", a_cols, b_rows);
        return None;
    }
    
    // Create result matrix with pre-allocated capacity
    let mut result = vec![vec![0.0; cols]; rows];
    
    // Perform multiplication (cache-friendly ordering)
    for i in 0..rows {
        for k in 0..a_cols {
            let a_ik = a[i][k];
            for j in 0..cols {
                result[i][j] += a_ik * b[k][j];
            }
        }
    }
    
    Some(result)
}

// Transpose a matrix
fn transpose(mat: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    // Handle empty matrix
    if mat.is_empty() || mat[0].is_empty() {
        eprintln!("Empty matrix in transpose");
        return None;
    }
    
    let rows = mat.len();
    let cols = mat[0].len();
    
    // Validate consistent row lengths
    if mat.iter().any(|row| row.len() != cols) {
        eprintln!("Inconsistent row lengths in transpose");
        return None;
    }
    
    // Create transposed matrix
    let mut transposed = vec![vec![0.0; rows]; cols];
    
    // Perform transposition
    for i in 0..rows {
        for j in 0..cols {
            transposed[j][i] = mat[i][j];
        }
    }
    
    Some(transposed)
}

// Identity matrix generator
fn identity_matrix(size: usize) -> Vec<Vec<f64>> {
    let mut identity = vec![vec![0.0; size]; size];
    for i in 0..size {
        identity[i][i] = 1.0;
    }
    identity
}

// Invert a matrix using Gaussian elimination
fn invert_matrix(matrix: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    // Handle empty matrix
    if matrix.is_empty() || matrix[0].is_empty() {
        eprintln!("Empty matrix in inversion");
        return None;
    }
    
    let n = matrix.len();
    
    // Verify square matrix
    if matrix.iter().any(|row| row.len() != n) {
        eprintln!("Cannot invert non-square matrix");
        return None;
    }
    
    // Create working copies
    let mut a = matrix.to_vec();
    let mut inv = identity_matrix(n);
    
    // Gaussian elimination with partial pivoting
    for i in 0..n {
        // Find pivot with maximum absolute value
        let mut max_val = 0.0;
        let mut max_row = i;
        
        for k in i..n {
            let abs_val = a[k][i].abs();
            if abs_val > max_val {
                max_val = abs_val;
                max_row = k;
            }
        }
        
        // Check if matrix is singular
        if max_val < 1e-10 {
            eprintln!("Matrix is nearly singular, inversion may be unstable");
            return None;
        }
        
        // Swap rows
        if max_row != i {
            a.swap(i, max_row);
            inv.swap(i, max_row);
        }
        
        // Scale the pivot row
        let diag = a[i][i];
        for j in 0..n {
            a[i][j] /= diag;
            inv[i][j] /= diag;
        }
        
        // Eliminate other rows
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
    
    // Verify the result by checking that A * A^-1 ≈ I
    let a_identity = mat_mult(matrix, &inv);
    if let Some(a_id) = a_identity {
        let is_close_to_identity = a_id.iter().enumerate().all(|(i, row)| {
            row.iter().enumerate().all(|(j, &val)| {
                if i == j {
                    (val - 1.0).abs() < 1e-8
                } else {
                    val.abs() < 1e-8
                }
            })
        });
        
        if !is_close_to_identity {
            eprintln!("Warning: Matrix inversion may be numerically unstable");
        }
    }
    
    Some(inv)
}

fn to_column_vector(vec: &[f64]) -> Vec<Vec<f64>> {
    vec.iter().map(|&x| vec![x]).collect()
}

// Black-Litterman Model Implementation
pub fn black_litterman(
    sigma: &[Vec<f64>], // Covariance matrix (Σ)
    market_weights: &[f64], // Market capitalization weights (w_m)
    tau: f64, // Small scaling factor
    p: &[Vec<f64>], // Views matrix (P)
    q: &[f64], // Views vector (Q)
    omega: &[Vec<f64>] // Uncertainty matrix (Ω)
) -> Vec<f64> {
    // Check if inputs are valid and have compatible dimensions
    if sigma.is_empty() || market_weights.is_empty() || p.is_empty() || q.is_empty() || omega.is_empty() {
        eprintln!("Empty inputs to black_litterman");
        return Vec::new();
    }
    
    // Validate dimensions
    let n = sigma.len();
    if sigma[0].len() != n {
        eprintln!("Covariance matrix must be square");
        return Vec::new();
    }
    
    if market_weights.len() != n {
        eprintln!("Market weights dimension doesn't match covariance matrix");
        return Vec::new();
    }
    
    if p[0].len() != n {
        eprintln!("Views matrix column dimension doesn't match covariance matrix");
        return Vec::new();
    }
    
    let k = p.len(); // Number of views
    if omega.len() != k || omega[0].len() != k || q.len() != k {
        eprintln!("Views dimensions mismatch");
        return Vec::new();
    }
    
    // Create tau*sigma matrix
    let tau_sigma: Vec<Vec<f64>> = sigma.iter().map(|row| 
        row.iter().map(|&val| val * tau).collect()
    ).collect();
    
    // Calculate pi (implied excess equilibrium returns)
    let mut pi = vec![0.0; n];
    
    // This is more efficient than matrix multiplication for this specific case
    for i in 0..n {
        for j in 0..n {
            pi[i] += tau_sigma[i][j] * market_weights[j];
        }
    }
    
    // Calculate inverse of tau*sigma
    let tau_sigma_inv = match invert_matrix(&tau_sigma) {
        Some(result) => result,
        None => {
            eprintln!("Failed to invert tau*sigma matrix");
            return Vec::new();
        }
    };
    
    // Calculate inverse of omega
    let omega_inv = match invert_matrix(omega) {
        Some(result) => result,
        None => {
            eprintln!("Failed to invert omega matrix");
            return Vec::new();
        }
    };
    
    // Compute P' (transpose of P)
    let p_transposed = match transpose(p) {
        Some(result) => result,
        None => {
            eprintln!("Failed to transpose P matrix");
            return Vec::new();
        }
    };
    
    // Calculate P' * Omega^-1
    let pt_omega_inv = match mat_mult(&p_transposed, &omega_inv) {
        Some(result) => result,
        None => {
            eprintln!("Failed in P' * Omega^-1 calculation");
            return Vec::new();
        }
    };
    
    // Calculate P' * Omega^-1 * P
    let p_pt_omega_inv = match mat_mult(&pt_omega_inv, p) {
        Some(result) => result,
        None => {
            eprintln!("Failed in P' * Omega^-1 * P calculation");
            return Vec::new();
        }
    };
    
    // Calculate (tau*Sigma)^-1 + P' * Omega^-1 * P
    let mut posterior_precision = p_pt_omega_inv;
    for i in 0..n {
        for j in 0..n {
            posterior_precision[i][j] += tau_sigma_inv[i][j];
        }
    }
    
    // Calculate posterior covariance
    let posterior_cov = match invert_matrix(&posterior_precision) {
        Some(result) => result,
        None => {
            eprintln!("Failed to invert posterior precision matrix");
            return Vec::new();
        }
    };
    
    // Calculate P' * Omega^-1 * q
    let q_col = to_column_vector(q);
    let second_term_result = match mat_mult(&pt_omega_inv, &q_col) {
        Some(result) => result,
        None => {
            eprintln!("Failed in second term calculation");
            return Vec::new();
        }
    };
    
    // Calculate (tau*Sigma)^-1 * pi
    let mut first_term_result = vec![vec![0.0; 1]; n];
    for i in 0..n {
        for j in 0..n {
            first_term_result[i][0] += tau_sigma_inv[i][j] * pi[j];
        }
    }
    
    // Combine terms into one vector
    let mut combined_terms = vec![0.0; n];
    for i in 0..n {
        combined_terms[i] = first_term_result[i][0] + second_term_result[i][0];
    }
    
    // Calculate posterior mean
    let mut posterior_mean = vec![0.0; n];
    for i in 0..n {
        for j in 0..n {
            posterior_mean[i] += posterior_cov[i][j] * combined_terms[j];
        }
    }
    
    posterior_mean
}

// Mean-variance optimization for portfolio allocation
pub fn mvo(cov: &[Vec<f64>], arv: Vec<f64>) -> Vec<f64> {
    // Check if inputs are valid and have compatible dimensions
    if cov.is_empty() || arv.is_empty() {
        eprintln!("Empty inputs to MVO");
        return Vec::new();
    }
    
    // Validate dimensions
    let n = cov.len();
    if cov[0].len() != n || arv.len() != n {
        eprintln!("Incompatible dimensions in MVO inputs");
        return Vec::new();
    }
    
    // Calculate inverse of covariance matrix
    let cov_inv = match invert_matrix(cov) {
        Some(result) => result,
        None => {
            eprintln!("Failed to invert covariance matrix");
            return Vec::new();
        }
    };
    
    // Calculate optimal weights (more efficient direct calculation)
    let mut weights = vec![0.0; n];
    for i in 0..n {
        for j in 0..n {
            weights[i] += cov_inv[i][j] * arv[j];
        }
    }
    
    // Normalize weights to sum to 1.0
    let sum: f64 = weights.iter().sum();
    if sum.abs() > 1e-10 {
        for i in 0..n {
            weights[i] /= sum;
        }
    }
    
    weights
}
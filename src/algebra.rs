use std::ops;

#[derive(Debug, PartialEq, Clone)]
pub struct Matrix(f32, f32, f32, f32, f32, f32);

impl Default for Matrix {
    fn default() -> Self {
        Self(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }
}

impl Matrix {
    fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self(a, b, c, d, e, f)
    }
}

impl ops::Mul<Matrix> for Matrix {
    type Output = Matrix;
    fn mul(self, rhs: Matrix) -> Self::Output {
        Matrix::new(
            self.0 * rhs.0 + self.1 * rhs.2,
            self.0 * rhs.1 + self.1 * rhs.3,
            self.2 * rhs.0 + self.3 * rhs.2,
            self.2 * rhs.1 + self.3 * rhs.3,
            self.4 * rhs.0 + self.5 * rhs.2 + rhs.4,
            self.4 * rhs.1 + self.5 * rhs.3 + rhs.5,
        )
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_matrix_mul() {
        let id = Matrix::default();
        let m1 = Matrix::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        assert_eq!(m1.clone() * id, m1)
    }
}

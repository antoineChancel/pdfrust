use std::{fmt::Display, ops};

#[derive(Debug, PartialEq, Clone)]
pub enum Number {
    Integer(i16),
    Real(f32),
}

impl From<Number> for f32 {
    fn from(value: Number) -> Self {
        match value {
            Number::Integer(i) => i as f32,
            Number::Real(f) => f,
        }
    }
}

impl From<Number> for usize {
    fn from(value: Number) -> Self {
        match value {
            Number::Integer(n) => n as usize,
            Number::Real(r) => r as usize,
        }
    }
}

impl std::ops::Div for Number {
    type Output = Number;
    fn div(self, rhs: Self) -> Self::Output {
        match self {
            Number::Integer(a) => match rhs {
                Number::Integer(b) => Number::Real(f32::from(a) / f32::from(b)),
                Number::Real(b) => Number::Real(f32::from(a) / b),
            },
            Number::Real(a) => match rhs {
                Number::Integer(b) => Number::Real(a / f32::from(b)),
                Number::Real(b) => Number::Real(a / b),
            },
        }
    }
}

impl std::ops::Neg for Number {
    type Output = Number;
    fn neg(self) -> Self::Output {
        match self {
            Number::Integer(i) => Number::Integer(-i),
            Number::Real(f) => Number::Real(-f),
        }
    }
}

impl std::ops::Add for Number {
    type Output = Number;
    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Number::Integer(a) => match rhs {
                Number::Integer(b) => Number::Integer(a + b),
                Number::Real(b) => Number::Real(f32::from(a) + b),
            },
            Number::Real(a) => match rhs {
                Number::Integer(b) => Number::Real(a + f32::from(b)),
                Number::Real(b) => Number::Real(a + b),
            },
        }
    }
}

impl std::ops::Mul for Number {
    type Output = Number;
    fn mul(self, rhs: Self) -> Self::Output {
        match self {
            Number::Integer(a) => match rhs {
                Number::Integer(b) => Number::Integer(a * b),
                Number::Real(b) => Number::Real(f32::from(a) * b),
            },
            Number::Real(a) => match rhs {
                Number::Integer(b) => Number::Real(a * f32::from(b)),
                Number::Real(b) => Number::Real(a * b),
            },
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Matrix(f32, f32, f32, f32, f32, f32);

impl Display for Matrix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}, {}, {}, {}, {})",
            self.0, self.1, self.2, self.3, self.4, self.5
        )
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }
}

impl Matrix {
    pub fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self(a, b, c, d, e, f)
    }
}

impl From<[Number; 6]> for Matrix {
    fn from(value: [Number; 6]) -> Self {
        Self(
            f32::from(value[0].clone()),
            f32::from(value[1].clone()),
            f32::from(value[2].clone()),
            f32::from(value[3].clone()),
            f32::from(value[4].clone()),
            f32::from(value[5].clone()),
        )
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

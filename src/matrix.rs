use std::error::Error;
use std::fmt::{self, Display};

use crate::problem::EPSILON;

#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    data: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatrixError {
    NonRectangular,
    NotSquare,
    IncompatibleDimensions,
    Singular,
}

impl Display for MatrixError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NonRectangular => "matriz inválida: linhas com tamanhos diferentes",
            Self::NotSquare => "a matriz precisa ser quadrada",
            Self::IncompatibleDimensions => "as matrizes não possuem dimensões compatíveis",
            Self::Singular => "a matriz não possui inversa",
        })
    }
}

impl Error for MatrixError {}

impl Matrix {
    pub fn new(rows: usize, columns: usize, initial_value: f64) -> Self {
        Self {
            data: vec![vec![initial_value; columns]; rows],
        }
    }

    pub fn from_rows(data: Vec<Vec<f64>>) -> Result<Self, MatrixError> {
        let matrix = Self { data };
        if matrix.is_rectangular() {
            Ok(matrix)
        } else {
            Err(MatrixError::NonRectangular)
        }
    }

    pub fn rows(&self) -> usize {
        self.data.len()
    }

    pub fn columns(&self) -> usize {
        self.data.first().map_or(0, Vec::len)
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_rectangular(&self) -> bool {
        self.data.iter().all(|row| row.len() == self.columns())
    }

    pub fn is_square(&self) -> bool {
        !self.is_empty() && self.is_rectangular() && self.rows() == self.columns()
    }

    pub fn get(&self, row: usize, column: usize) -> f64 {
        self.data[row][column]
    }

    pub fn set(&mut self, row: usize, column: usize, value: f64) {
        self.data[row][column] = value;
    }

    fn make_minor(&self, removed_row: usize, removed_column: usize) -> Self {
        let mut data = Vec::new();

        for row in 0..self.rows() {
            if row != removed_row {
                let mut minor_row = Vec::new();
                for column in 0..self.columns() {
                    if column != removed_column {
                        minor_row.push(self.data[row][column]);
                    }
                }
                data.push(minor_row);
            }
        }

        Self { data }
    }

    pub fn determinant(&self) -> Result<f64, MatrixError> {
        if !self.is_square() {
            return Err(MatrixError::NotSquare);
        }
        match self.rows() {
            1 => Ok(self.data[0][0]),
            2 => Ok(self.data[0][0] * self.data[1][1] - self.data[0][1] * self.data[1][0]),
            _ => {
                let mut determinant = 0.0;
                for column in 0..self.columns() {
                    let sign = if column % 2 == 0 { 1.0 } else { -1.0 };
                    determinant +=
                        self.data[0][column] * sign * self.make_minor(0, column).determinant()?;
                }
                Ok(determinant)
            }
        }
    }

    pub fn multiply(&self, other: &Self) -> Result<Self, MatrixError> {
        if self.columns() != other.rows() {
            return Err(MatrixError::IncompatibleDimensions);
        }
        let mut result = Self::new(self.rows(), other.columns(), 0.0);
        for row in 0..self.rows() {
            for column in 0..other.columns() {
                for index in 0..self.columns() {
                    result.data[row][column] += self.data[row][index] * other.data[index][column];
                }
            }
        }
        Ok(result)
    }

    pub fn multiply_scalar(&self, scalar: f64) -> Self {
        let mut result = self.clone();
        for row in 0..result.rows() {
            for column in 0..result.columns() {
                result.data[row][column] *= scalar;
            }
        }
        result
    }

    pub fn transpose(&self) -> Self {
        let mut result = Self::new(self.columns(), self.rows(), 0.0);
        for row in 0..self.rows() {
            for column in 0..self.columns() {
                result.data[column][row] = self.data[row][column];
            }
        }
        result
    }

    pub fn solve(&self, right_hand_side: &Self) -> Result<Self, MatrixError> {
        if !self.is_square() {
            return Err(MatrixError::NotSquare);
        }
        if right_hand_side.rows() != self.rows() {
            return Err(MatrixError::IncompatibleDimensions);
        }

        let size = self.rows();
        let mut coefficients = self.clone();
        let mut constants = right_hand_side.clone();

        // Transforma a matriz de coeficientes em uma matriz triangular superior.
        for pivot_column in 0..size {
            let mut pivot_row = pivot_column;
            for row in (pivot_column + 1)..size {
                if coefficients.data[row][pivot_column].abs()
                    > coefficients.data[pivot_row][pivot_column].abs()
                {
                    pivot_row = row;
                }
            }

            if coefficients.data[pivot_row][pivot_column].abs() <= EPSILON {
                return Err(MatrixError::Singular);
            }

            coefficients.data.swap(pivot_column, pivot_row);
            constants.data.swap(pivot_column, pivot_row);

            for row in (pivot_column + 1)..size {
                let factor = coefficients.data[row][pivot_column]
                    / coefficients.data[pivot_column][pivot_column];
                coefficients.data[row][pivot_column] = 0.0;

                for column in (pivot_column + 1)..size {
                    coefficients.data[row][column] -=
                        factor * coefficients.data[pivot_column][column];
                }
                for column in 0..constants.columns() {
                    constants.data[row][column] -= factor * constants.data[pivot_column][column];
                }
            }
        }

        // Resolve as incógnitas da última linha para a primeira.
        let mut solution = Self::new(size, constants.columns(), 0.0);
        for row in (0..size).rev() {
            for right_column in 0..constants.columns() {
                let mut value = constants.data[row][right_column];
                for column in (row + 1)..size {
                    value -= coefficients.data[row][column] * solution.data[column][right_column];
                }
                solution.data[row][right_column] = value / coefficients.data[row][row];
            }
        }

        Ok(solution)
    }

    pub fn cofactor_matrix(&self) -> Result<Self, MatrixError> {
        if !self.is_square() {
            return Err(MatrixError::NotSquare);
        }
        if self.rows() == 1 {
            return Ok(Self::new(1, 1, 1.0));
        }
        let mut result = Self::new(self.rows(), self.columns(), 0.0);
        for row in 0..self.rows() {
            for column in 0..self.columns() {
                let sign = if (row + column) % 2 == 0 { 1.0 } else { -1.0 };
                result.data[row][column] = sign * self.make_minor(row, column).determinant()?;
            }
        }
        Ok(result)
    }

    pub fn adjugate(&self) -> Result<Self, MatrixError> {
        Ok(self.cofactor_matrix()?.transpose())
    }

    pub fn inverse(&self) -> Result<Self, MatrixError> {
        let determinant = self.determinant()?;
        if determinant.abs() <= EPSILON {
            return Err(MatrixError::Singular);
        }
        Ok(self.adjugate()?.multiply_scalar(1.0 / determinant))
    }
}

#[cfg(test)]
mod tests {
    use super::{Matrix, MatrixError};

    fn matrix(rows: &[&[f64]]) -> Matrix {
        Matrix::from_rows(rows.iter().map(|row| row.to_vec()).collect()).unwrap()
    }

    fn assert_matrix_approximately_eq(left: &Matrix, right: &Matrix) {
        assert_eq!(left.rows(), right.rows());
        assert_eq!(left.columns(), right.columns());
        for row in 0..left.rows() {
            for column in 0..left.columns() {
                assert!((left.data[row][column] - right.data[row][column]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn validates_rows() {
        assert_eq!(
            Matrix::from_rows(vec![vec![1.0], vec![1.0, 2.0]]),
            Err(MatrixError::NonRectangular)
        );
    }

    #[test]
    fn calculates_determinants() {
        assert_eq!(matrix(&[&[5.0]]).determinant().unwrap(), 5.0);
        assert_eq!(
            matrix(&[&[1.0, 2.0], &[3.0, 4.0]]).determinant().unwrap(),
            -2.0
        );
        assert_eq!(
            matrix(&[&[6.0, 1.0, 1.0], &[4.0, -2.0, 5.0], &[2.0, 8.0, 7.0]])
                .determinant()
                .unwrap(),
            -306.0
        );
    }

    #[test]
    fn multiplies_and_transposes() {
        let left = matrix(&[&[1.0, 2.0], &[3.0, 4.0]]);
        let right = matrix(&[&[2.0], &[1.0]]);
        assert_eq!(left.multiply(&right).unwrap(), matrix(&[&[4.0], &[10.0]]));
        assert_eq!(right.transpose(), matrix(&[&[2.0, 1.0]]));
    }

    #[test]
    fn calculates_cofactor_adjugate_and_inverse() {
        let value = matrix(&[&[4.0, 7.0], &[2.0, 6.0]]);
        assert_eq!(
            value.cofactor_matrix().unwrap(),
            matrix(&[&[6.0, -2.0], &[-7.0, 4.0]])
        );
        assert_eq!(
            value.adjugate().unwrap(),
            matrix(&[&[6.0, -7.0], &[-2.0, 4.0]])
        );
        assert_matrix_approximately_eq(
            &value.inverse().unwrap(),
            &matrix(&[&[0.6, -0.7], &[-0.2, 0.4]]),
        );
    }

    #[test]
    fn solves_linear_systems_with_partial_pivoting() {
        // Sistema linear:
        //  2x +  y -  z =   8
        // -3x -  y + 2z = -11
        // -2x +  y + 2z =  -3
        let coefficients = matrix(&[&[2.0, 1.0, -1.0], &[-3.0, -1.0, 2.0], &[-2.0, 1.0, 2.0]]);
        let constants = matrix(&[&[8.0], &[-11.0], &[-3.0]]);

        let solution = coefficients.solve(&constants).unwrap();

        assert_matrix_approximately_eq(&solution, &matrix(&[&[2.0], &[3.0], &[-1.0]]));
        assert_matrix_approximately_eq(&coefficients.multiply(&solution).unwrap(), &constants);
    }

    #[test]
    fn swaps_rows_when_the_pivot_is_zero() {
        let coefficients = matrix(&[&[0.0, 2.0], &[1.0, 3.0]]);
        let constants = matrix(&[&[4.0], &[7.0]]);

        let solution = coefficients.solve(&constants).unwrap();

        assert_matrix_approximately_eq(&solution, &matrix(&[&[1.0], &[2.0]]));
    }

    #[test]
    fn rejects_invalid_linear_systems() {
        assert_eq!(
            matrix(&[&[1.0, 2.0]]).solve(&matrix(&[&[1.0]])),
            Err(MatrixError::NotSquare)
        );
        assert_eq!(
            matrix(&[&[1.0, 0.0], &[0.0, 1.0]]).solve(&matrix(&[&[1.0]])),
            Err(MatrixError::IncompatibleDimensions)
        );
        assert_eq!(
            matrix(&[&[1.0, 2.0], &[2.0, 4.0]]).solve(&matrix(&[&[3.0], &[6.0]])),
            Err(MatrixError::Singular)
        );
    }

    #[test]
    fn rejects_invalid_inverse_and_dimensions() {
        assert_eq!(
            matrix(&[&[1.0, 2.0], &[2.0, 4.0]]).inverse(),
            Err(MatrixError::Singular)
        );
        assert_eq!(
            matrix(&[&[1.0, 1.0], &[1.0, 1.0 + 1e-10]]).inverse(),
            Err(MatrixError::Singular)
        );
        assert_eq!(
            matrix(&[&[1.0, 2.0]]).inverse(),
            Err(MatrixError::NotSquare)
        );
        assert_eq!(
            matrix(&[&[1.0, 2.0]]).multiply(&matrix(&[&[1.0, 2.0]])),
            Err(MatrixError::IncompatibleDimensions)
        );
    }
}

use std::error::Error;
use std::fmt::{self, Display};

use crate::matrix::Matrix;
use crate::problem::{Problem, Relation, Sense, VariableKind};

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixForm {
    pub a: Matrix,
    pub basic_matrix: Matrix,
    pub b: Matrix,
    pub c: Matrix,
    pub variables: Vec<usize>,
    pub variable_kinds: Vec<VariableKind>,
    pub original_sense: Sense,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatrixFormError {
    NotNormalized,
    InvalidBasicMatrix,
    UnknownVariable { variable: usize },
}

impl Display for MatrixFormError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotNormalized => formatter
                .write_str("o problema precisa estar normalizado antes da conversão matricial"),
            Self::InvalidBasicMatrix => formatter.write_str(
                "não foi possível montar a matriz básica: deve existir uma variável de folga ou excesso por restrição",
            ),
            Self::UnknownVariable { variable } => {
                write!(formatter, "x_{variable} não possui tipo registrado")
            }
        }
    }
}

impl Error for MatrixFormError {}

fn variable_index(variables: &[usize], variable: usize) -> Result<usize, MatrixFormError> {
    for (index, current) in variables.iter().enumerate() {
        if *current == variable {
            return Ok(index);
        }
    }
    Err(MatrixFormError::UnknownVariable { variable })
}

impl MatrixForm {
    pub fn from_problem(problem: &Problem) -> Result<Self, MatrixFormError> {
        if problem.sense != Sense::Min {
            return Err(MatrixFormError::NotNormalized);
        }
        for constraint in &problem.constraints {
            if constraint.relation != Relation::Equal {
                return Err(MatrixFormError::NotNormalized);
            }
        }

        let mut variables = Vec::new();
        let mut variable_kinds = Vec::new();
        for (variable, kind) in &problem.variable_kinds {
            variables.push(*variable);
            variable_kinds.push(*kind);
        }

        let mut a = Matrix::new(problem.constraints.len(), variables.len(), 0.0);
        let mut b = Matrix::new(problem.constraints.len(), 1, 0.0);
        let mut c = Matrix::new(variables.len(), 1, 0.0);

        for row in 0..problem.constraints.len() {
            let constraint = &problem.constraints[row];
            b.set(row, 0, constraint.rhs);

            for term in &constraint.terms {
                let column = variable_index(&variables, term.variable)?;
                a.set(row, column, term.coefficient);
            }
        }

        for term in &problem.objective {
            let row = variable_index(&variables, term.variable)?;
            c.set(row, 0, term.coefficient);
        }

        let mut basic_columns = Vec::new();
        for (column, kind) in variable_kinds.iter().enumerate() {
            if *kind != VariableKind::Original {
                basic_columns.push(column);
            }
        }

        if basic_columns.len() != problem.constraints.len() {
            return Err(MatrixFormError::InvalidBasicMatrix);
        }

        let mut basic_matrix =
            Matrix::new(problem.constraints.len(), problem.constraints.len(), 0.0);
        for (basic_column, a_column) in basic_columns.iter().enumerate() {
            for row in 0..problem.constraints.len() {
                basic_matrix.set(row, basic_column, a.get(row, *a_column));
            }
        }

        Ok(Self {
            a,
            basic_matrix,
            b,
            c,
            variables,
            variable_kinds,
            original_sense: problem.original_sense,
        })
    }

    pub fn restore_objective_value(&self, normalized_value: f64) -> f64 {
        match self.original_sense {
            Sense::Max => -normalized_value,
            Sense::Min => normalized_value,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::normalizer::normalize;
    use crate::problem::{Sense, VariableKind};
    use crate::problem_parser::parse_problem;

    use super::{MatrixForm, MatrixFormError};

    #[test]
    fn converts_normalized_problem_to_a_b_and_c() {
        let problem = parse_problem(
            "max z = 5x_1 + 4x_2\n\
             6x_1 + 4x_2 <= 24\n\
             x_1 + 2x_2 >= 6\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let form = MatrixForm::from_problem(&normalized).unwrap();

        assert_eq!(form.variables, vec![1, 2, 3, 4]);
        assert_eq!(
            form.variable_kinds,
            vec![
                VariableKind::Original,
                VariableKind::Original,
                VariableKind::Slack,
                VariableKind::Excess
            ]
        );
        assert_eq!(form.a.rows(), 2);
        assert_eq!(form.a.columns(), 4);
        assert_eq!(form.a.get(0, 0), 6.0);
        assert_eq!(form.a.get(0, 1), 4.0);
        assert_eq!(form.a.get(0, 2), 1.0);
        assert_eq!(form.a.get(1, 0), 1.0);
        assert_eq!(form.a.get(1, 1), 2.0);
        assert_eq!(form.a.get(1, 3), -1.0);
        assert_eq!(form.basic_matrix.rows(), 2);
        assert_eq!(form.basic_matrix.columns(), 2);
        assert_eq!(form.basic_matrix.get(0, 0), 1.0);
        assert_eq!(form.basic_matrix.get(0, 1), 0.0);
        assert_eq!(form.basic_matrix.get(1, 0), 0.0);
        assert_eq!(form.basic_matrix.get(1, 1), -1.0);
        assert_eq!(form.b.get(0, 0), 24.0);
        assert_eq!(form.b.get(1, 0), 6.0);
        assert_eq!(form.c.get(0, 0), -5.0);
        assert_eq!(form.c.get(1, 0), -4.0);
        assert_eq!(form.c.get(2, 0), 0.0);
        assert_eq!(form.c.get(3, 0), 0.0);
        assert_eq!(normalized.variable_kinds[&3], VariableKind::Slack);
        assert_eq!(normalized.variable_kinds[&4], VariableKind::Excess);
    }

    #[test]
    fn restores_original_objective_value() {
        let max = normalize(&parse_problem("max z = 2x_1\n").unwrap()).unwrap();
        let min = normalize(&parse_problem("min z = 2x_1\n").unwrap()).unwrap();

        let max_form = MatrixForm::from_problem(&max).unwrap();
        let min_form = MatrixForm::from_problem(&min).unwrap();

        assert_eq!(max_form.original_sense, Sense::Max);
        assert_eq!(max_form.restore_objective_value(-10.0), 10.0);
        assert_eq!(min_form.restore_objective_value(10.0), 10.0);
    }

    #[test]
    fn rejects_problem_without_one_basic_variable_per_constraint() {
        let problem = parse_problem("min z = x_1\nx_1 = 2\n").unwrap();
        let normalized = normalize(&problem).unwrap();

        assert_eq!(
            MatrixForm::from_problem(&normalized),
            Err(MatrixFormError::InvalidBasicMatrix)
        );
    }
}

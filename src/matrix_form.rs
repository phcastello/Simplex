use std::error::Error;
use std::fmt::{self, Display};

use crate::matrix::{Matrix, MatrixError};
use crate::problem::{EPSILON, Problem, Relation, Sense, VariableBound, VariableKind};

pub use crate::simplex::{
    Direction, LeavingVariable, Ratio, RatioTest, ReducedCost, SimplexPhase, SimplexResult,
    SimplexState, SimplexStatus,
};

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixForm {
    pub a: Matrix,
    pub b: Matrix,
    pub c: Matrix,
    pub variables: Vec<usize>,
    pub variable_kinds: Vec<VariableKind>,
    pub fixed_zero_variables: Vec<usize>,
    pub original_sense: Sense,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatrixFormError {
    NotNormalized,
    MissingInitialBasis { rows: Vec<usize> },
    UnknownVariable { variable: usize },
}

impl Display for MatrixFormError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotNormalized => formatter
                .write_str("o problema precisa estar normalizado antes da conversão matricial"),
            Self::MissingInitialBasis { rows } => {
                write!(formatter, "linhas sem variável básica natural: ")?;
                for (index, row) in rows.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(formatter, "{}", row + 1)?;
                }
                Ok(())
            }
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
        let mut fixed_zero_variables = Vec::new();
        for (variable, kind) in &problem.variable_kinds {
            if problem.variable_bounds.get(variable) == Some(&VariableBound::FixedZero) {
                fixed_zero_variables.push(*variable);
                continue;
            }
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

        Ok(Self {
            a,
            b,
            c,
            variables,
            variable_kinds,
            fixed_zero_variables,
            original_sense: problem.original_sense,
        })
    }

    pub(crate) fn find_natural_slack_basis(&self) -> Vec<Option<usize>> {
        let mut basis = vec![None; self.a.rows()];

        for (row, basis_column) in basis.iter_mut().enumerate() {
            for column in 0..self.variables.len() {
                if self.variable_kinds[column] != VariableKind::Slack {
                    continue;
                }
                if self.is_positive_identity_column(column, row) {
                    *basis_column = Some(column);
                    break;
                }
            }
        }

        basis
    }

    pub fn natural_slack_basis(&self) -> Vec<Option<usize>> {
        self.find_natural_slack_basis()
    }

    fn is_positive_identity_column(&self, column: usize, identity_row: usize) -> bool {
        for row in 0..self.a.rows() {
            let value = self.a.get(row, column);

            if row == identity_row {
                if (value - 1.0).abs() > EPSILON {
                    return false;
                }
            } else if value.abs() > EPSILON {
                return false;
            }
        }

        true
    }

    pub fn restore_objective_value(&self, normalized_value: f64) -> f64 {
        match self.original_sense {
            Sense::Max => -normalized_value,
            Sense::Min => normalized_value,
        }
    }

    pub fn objective_value(&self, state: &SimplexState) -> Result<f64, MatrixError> {
        let solution = state.solution(self)?;
        let mut value = 0.0;

        for row in 0..self.c.rows() {
            value += self.c.get(row, 0) * solution.get(row, 0);
        }

        Ok(self.restore_objective_value(value))
    }

    pub(crate) fn debug_validate_state(&self, state: &SimplexState) {
        debug_assert_eq!(state.basic_columns.len(), self.a.rows());
        debug_assert_eq!(state.active_costs.rows(), self.variables.len());
        debug_assert_eq!(state.active_costs.columns(), 1);

        let mut seen = vec![false; self.variables.len()];

        for column in &state.basic_columns {
            debug_assert!(*column < self.variables.len());
            debug_assert!(!seen[*column]);
            seen[*column] = true;
        }

        for column in &state.non_basic_columns {
            debug_assert!(*column < self.variables.len());
            debug_assert!(!seen[*column]);
            seen[*column] = true;
        }

        for was_seen in seen {
            debug_assert!(was_seen);
        }

        let basic_matrix = state.basic_matrix(self);
        debug_assert_eq!(basic_matrix.rows(), self.a.rows());
        debug_assert_eq!(basic_matrix.columns(), state.basic_columns.len());
        debug_assert_eq!(basic_matrix.rows(), basic_matrix.columns());
    }
}

#[cfg(test)]
mod tests {
    use crate::normalizer::normalize;
    use crate::problem::{Sense, VariableKind};
    use crate::problem_parser::parse_problem;

    use super::{MatrixForm, MatrixFormError, SimplexPhase, SimplexStatus};

    fn form_from_text(text: &str) -> MatrixForm {
        let problem = parse_problem(text).unwrap();
        let normalized = normalize(&problem).unwrap();
        MatrixForm::from_problem(&normalized).unwrap()
    }

    #[test]
    fn converts_normalized_problem_to_a_b_and_c_without_execution_state() {
        let form = form_from_text(
            "max z = 5x_1 + 4x_2\n\
             6x_1 + 4x_2 <= 24\n\
             x_1 + 2x_2 >= 6\n",
        );

        assert_eq!(form.variables, vec![1, 2, 3, 4]);
        assert_eq!(
            form.variable_kinds,
            vec![
                VariableKind::Original,
                VariableKind::Original,
                VariableKind::Slack,
                VariableKind::Excess,
            ]
        );
        assert_eq!(form.a.rows(), 2);
        assert_eq!(form.a.columns(), 4);
        assert_eq!(form.a.get(0, 2), 1.0);
        assert_eq!(form.a.get(1, 3), -1.0);
        assert_eq!(form.c.get(0, 0), -5.0);
        assert_eq!(form.c.get(1, 0), -4.0);
    }

    #[test]
    fn restores_original_objective_value() {
        let max = form_from_text("max z = 2x_1\nx_1 <= 5\n");
        let min = form_from_text("min z = 2x_1\nx_1 <= 5\n");

        assert_eq!(max.original_sense, Sense::Max);
        assert_eq!(max.restore_objective_value(-10.0), 10.0);
        assert_eq!(min.restore_objective_value(10.0), 10.0);
    }

    #[test]
    fn finds_natural_slack_basis_line_by_line() {
        let form = form_from_text(
            "min z = x_1 + x_2\n\
             x_1 <= 5\n\
             x_1 + x_2 >= 4\n\
             x_2 <= 7\n",
        );
        let natural_basis = form.find_natural_slack_basis();

        assert_eq!(natural_basis, vec![Some(2), None, Some(4)]);
        assert_eq!(
            form.prepare_current_phase_two_state(),
            Err(MatrixFormError::MissingInitialBasis { rows: vec![1] })
        );
    }

    #[test]
    fn current_phase_two_state_uses_only_valid_slacks_as_basic() {
        let form = form_from_text(
            "max z = 3x_1 + 2x_2\n\
             x_1 + x_2 <= 4\n\
             x_1 <= 2\n\
             x_2 <= 3\n",
        );
        let state = form.prepare_current_phase_two_state().unwrap();

        assert_eq!(state.basic_columns, vec![2, 3, 4]);
        assert_eq!(state.non_basic_columns, vec![0, 1]);
        assert_eq!(state.phase, SimplexPhase::PhaseTwo);
        assert!(state.is_basic_solution_feasible(&form).unwrap());
    }

    #[test]
    fn simplex_solves_problem_that_needs_only_phase_two() {
        let form = form_from_text(
            "max z = 3x_1 + 2x_2\n\
             x_1 + x_2 <= 4\n\
             x_1 <= 2\n\
             x_2 <= 3\n",
        );

        let result = form.solve_simplex().unwrap();
        let solution = result.state.solution(&result.form).unwrap();

        assert_eq!(result.status, SimplexStatus::Optimal);
        assert_eq!(solution.get(0, 0), 2.0);
        assert_eq!(solution.get(1, 0), 2.0);
        assert_eq!(result.form.objective_value(&result.state).unwrap(), 10.0);
    }

    #[test]
    fn phase_one_makes_greater_equal_problem_feasible() {
        let form = form_from_text(
            "min z = x_1 + x_2\n\
             x_1 + x_2 >= 4\n",
        );

        let result = form.solve_simplex().unwrap();
        let solution = result.state.solution(&result.form).unwrap();

        assert_eq!(result.status, SimplexStatus::Optimal);
        assert!(solution.get(0, 0) + solution.get(1, 0) >= 4.0 - 1e-9);
        assert!(result.form.objective_value(&result.state).unwrap() <= 4.0 + 1e-9);
    }

    #[test]
    fn phase_one_reports_infeasible_problem() {
        let form = form_from_text(
            "max z = 4x_1 + 3x_2\n\
             x_1 + 3x_2 <= 7\n\
             2x_1 + 2x_2 = 8\n\
             x_1 + x_2 <= -3\n\
             x_2 <= 2\n",
        );

        let result = form.solve_simplex().unwrap();

        assert_eq!(result.status, SimplexStatus::Infeasible);
    }

    #[test]
    fn keeps_fixed_zero_original_variable_out_of_matrix_columns() {
        let form = form_from_text(
            "max z = 3x_1 + 3x_2 + 13x_3\n\
             -3x_1 + 6x_2 + 7x_3 <= 8\n\
             6x_1 - 3x_2 + 7x_3 <= 8\n\
             x_1 <= 2\n\
             x_3 <= 0\n\
             x_1, x_2, x_3 >= 0\n",
        );

        assert_eq!(form.fixed_zero_variables, vec![3]);
        assert!(!form.variables.contains(&3));
        assert_eq!(form.a.columns(), form.variables.len());
    }
}

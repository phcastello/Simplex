use std::error::Error;
use std::fmt::{self, Display};

use crate::matrix::{Matrix, MatrixError};
use crate::problem::{EPSILON, Problem, Relation, Sense, VariableKind};

#[derive(Clone, Debug, PartialEq)]
pub struct ReducedCost {
    pub variable: usize,
    pub column: usize,
    pub value: f64,
    pub improves_objective: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Direction {
    pub entering_variable: usize,
    pub entering_column_index: usize,
    pub reduced_cost: f64,
    pub entering_column: Matrix,
    pub y: Matrix,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ratio {
    pub basic_variable: usize,
    pub basic_row: usize,
    pub basic_value: f64,
    pub direction_value: f64,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RatioTest {
    pub ratios: Vec<Ratio>,
    pub is_unbounded: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeavingVariable {
    pub variable: usize,
    pub basic_row: usize,
    pub theta: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimplexStatus {
    Optimal,
    Unbounded,
    InfeasibleInitialBase,
    IterationLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimplexResult {
    pub status: SimplexStatus,
    pub iterations: usize,
    pub form: MatrixForm,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixForm {
    pub a: Matrix,
    pub basic_matrix: Matrix,
    pub b: Matrix,
    pub c: Matrix,
    pub variables: Vec<usize>,
    pub variable_kinds: Vec<VariableKind>,
    pub basic_columns: Vec<usize>,
    pub non_basic_columns: Vec<usize>,
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
        let mut non_basic_columns = Vec::new();
        for (column, kind) in variable_kinds.iter().enumerate() {
            if *kind == VariableKind::Original {
                non_basic_columns.push(column);
            } else {
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
            basic_columns,
            non_basic_columns,
            original_sense: problem.original_sense,
        })
    }

    pub fn restore_objective_value(&self, normalized_value: f64) -> f64 {
        match self.original_sense {
            Sense::Max => -normalized_value,
            Sense::Min => normalized_value,
        }
    }

    pub fn basic_solution(&self) -> Result<Matrix, MatrixError> {
        self.basic_matrix.solve(&self.b)
    }

    pub fn solution(&self) -> Result<Matrix, MatrixError> {
        let basic_solution = self.basic_solution()?;
        let mut solution = Matrix::new(self.variables.len(), 1, 0.0);

        for row in 0..self.basic_columns.len() {
            let column = self.basic_columns[row];
            let value = basic_solution.get(row, 0);
            solution.set(column, 0, value);
        }

        Ok(solution)
    }

    pub fn is_basic_solution_feasible(&self) -> Result<bool, MatrixError> {
        let basic_solution = self.basic_solution()?;

        for row in 0..basic_solution.rows() {
            if basic_solution.get(row, 0) < 0.0 {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn basic_costs(&self) -> Matrix {
        let mut basic_costs = Matrix::new(self.basic_columns.len(), 1, 0.0);

        for row in 0..self.basic_columns.len() {
            let column_in_c = self.basic_columns[row];
            let cost = self.c.get(column_in_c, 0);
            basic_costs.set(row, 0, cost);
        }

        basic_costs
    }

    pub fn lambda(&self) -> Result<Matrix, MatrixError> {
        let transposed_basic_matrix = self.basic_matrix.transpose();
        let basic_costs = self.basic_costs();

        transposed_basic_matrix.solve(&basic_costs)
    }

    pub fn reduced_costs(&self) -> Result<Vec<ReducedCost>, MatrixError> {
        let lambda = self.lambda()?;
        let mut reduced_costs = Vec::new();

        for column in &self.non_basic_columns {
            let variable = self.variables[*column];
            let objective_cost = self.c.get(*column, 0);
            let mut lambda_times_column = 0.0;

            for row in 0..self.a.rows() {
                let lambda_value = lambda.get(row, 0);
                let column_value = self.a.get(row, *column);
                lambda_times_column += lambda_value * column_value;
            }

            let reduced_cost = objective_cost - lambda_times_column;
            let improves_objective = reduced_cost < -EPSILON;

            reduced_costs.push(ReducedCost {
                variable,
                column: *column,
                value: reduced_cost,
                improves_objective,
            });
        }

        Ok(reduced_costs)
    }

    pub fn entering_variable(&self) -> Result<Option<ReducedCost>, MatrixError> {
        let reduced_costs = self.reduced_costs()?;
        let mut chosen_variable: Option<ReducedCost> = None;

        for reduced_cost in reduced_costs {
            if !reduced_cost.improves_objective {
                continue;
            }

            let should_choose = match &chosen_variable {
                Some(current) => reduced_cost.value < current.value,
                None => true,
            };

            if should_choose {
                chosen_variable = Some(reduced_cost);
            }
        }

        Ok(chosen_variable)
    }

    pub fn direction(&self) -> Result<Option<Direction>, MatrixError> {
        let entering_variable = match self.entering_variable()? {
            Some(variable) => variable,
            None => return Ok(None),
        };

        let mut entering_column = Matrix::new(self.a.rows(), 1, 0.0);
        for row in 0..self.a.rows() {
            let value = self.a.get(row, entering_variable.column);
            entering_column.set(row, 0, value);
        }

        let y = self.basic_matrix.solve(&entering_column)?;

        Ok(Some(Direction {
            entering_variable: entering_variable.variable,
            entering_column_index: entering_variable.column,
            reduced_cost: entering_variable.value,
            entering_column,
            y,
        }))
    }

    pub fn basic_solution_after_step(
        &self,
        direction: &Direction,
        theta: f64,
    ) -> Result<Matrix, MatrixError> {
        let basic_solution = self.basic_solution()?;
        let mut new_basic_solution = Matrix::new(basic_solution.rows(), 1, 0.0);

        for row in 0..basic_solution.rows() {
            let current_value = basic_solution.get(row, 0);
            let direction_value = direction.y.get(row, 0);
            let new_value = current_value - direction_value * theta;
            new_basic_solution.set(row, 0, new_value);
        }

        Ok(new_basic_solution)
    }

    pub fn ratio_test(&self, direction: &Direction) -> Result<RatioTest, MatrixError> {
        let basic_solution = self.basic_solution()?;
        let mut ratios = Vec::new();

        for row in 0..direction.y.rows() {
            let direction_value = direction.y.get(row, 0);

            if direction_value > EPSILON {
                let basic_column = self.basic_columns[row];
                let basic_variable = self.variables[basic_column];
                let basic_value = basic_solution.get(row, 0);
                let ratio = basic_value / direction_value;

                ratios.push(Ratio {
                    basic_variable,
                    basic_row: row,
                    basic_value,
                    direction_value,
                    value: ratio,
                });
            }
        }

        let is_unbounded = ratios.is_empty();

        Ok(RatioTest {
            ratios,
            is_unbounded,
        })
    }

    pub fn leaving_variable(&self, ratio_test: &RatioTest) -> Option<LeavingVariable> {
        let mut chosen_variable: Option<LeavingVariable> = None;

        for ratio in &ratio_test.ratios {
            let should_choose = match &chosen_variable {
                Some(current) => ratio.value < current.theta,
                None => true,
            };

            if should_choose {
                chosen_variable = Some(LeavingVariable {
                    variable: ratio.basic_variable,
                    basic_row: ratio.basic_row,
                    theta: ratio.value,
                });
            }
        }

        chosen_variable
    }

    pub fn rebuild_basic_matrix(&mut self) {
        let number_of_rows = self.a.rows();
        let number_of_basic_columns = self.basic_columns.len();
        let mut basic_matrix = Matrix::new(number_of_rows, number_of_basic_columns, 0.0);

        for basic_column in 0..number_of_basic_columns {
            let column_in_a = self.basic_columns[basic_column];

            for row in 0..number_of_rows {
                let value = self.a.get(row, column_in_a);
                basic_matrix.set(row, basic_column, value);
            }
        }

        self.basic_matrix = basic_matrix;
    }

    pub fn change_basis(&mut self, direction: &Direction, leaving_variable: &LeavingVariable) {
        let leaving_column = self.basic_columns[leaving_variable.basic_row];
        let entering_column = direction.entering_column_index;

        self.basic_columns[leaving_variable.basic_row] = entering_column;

        for position in 0..self.non_basic_columns.len() {
            if self.non_basic_columns[position] == entering_column {
                self.non_basic_columns[position] = leaving_column;
                break;
            }
        }

        self.rebuild_basic_matrix();
    }

    pub fn objective_value(&self) -> Result<f64, MatrixError> {
        let basic_solution = self.basic_solution()?;
        let basic_costs = self.basic_costs();
        let mut value = 0.0;

        for row in 0..basic_solution.rows() {
            value += basic_costs.get(row, 0) * basic_solution.get(row, 0);
        }

        Ok(self.restore_objective_value(value))
    }

    pub fn solve_simplex(mut self) -> Result<SimplexResult, MatrixError> {
        const MAX_ITERATIONS: usize = 1_000;

        if !self.is_basic_solution_feasible()? {
            return Ok(SimplexResult {
                status: SimplexStatus::InfeasibleInitialBase,
                iterations: 0,
                form: self,
            });
        }

        for iteration in 0..MAX_ITERATIONS {
            let direction = match self.direction()? {
                Some(direction) => direction,
                None => {
                    return Ok(SimplexResult {
                        status: SimplexStatus::Optimal,
                        iterations: iteration,
                        form: self,
                    });
                }
            };

            let ratio_test = self.ratio_test(&direction)?;
            if ratio_test.is_unbounded {
                return Ok(SimplexResult {
                    status: SimplexStatus::Unbounded,
                    iterations: iteration,
                    form: self,
                });
            }

            let leaving_variable = match self.leaving_variable(&ratio_test) {
                Some(variable) => variable,
                None => {
                    return Ok(SimplexResult {
                        status: SimplexStatus::Unbounded,
                        iterations: iteration,
                        form: self,
                    });
                }
            };
            self.change_basis(&direction, &leaving_variable);
        }

        Ok(SimplexResult {
            status: SimplexStatus::IterationLimit,
            iterations: MAX_ITERATIONS,
            form: self,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::matrix::Matrix;
    use crate::normalizer::normalize;
    use crate::problem::{Sense, VariableKind};
    use crate::problem_parser::parse_problem;

    use super::{MatrixForm, MatrixFormError, SimplexStatus};

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
        assert_eq!(form.basic_columns, vec![2, 3]);
        assert_eq!(form.non_basic_columns, vec![0, 1]);
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

    #[test]
    fn calculates_a_feasible_basic_solution() {
        let problem = parse_problem(
            "min z = x_1 + x_2\n\
             x_1 + x_2 <= 4\n\
             x_1 + 2x_2 <= 6\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let form = MatrixForm::from_problem(&normalized).unwrap();

        let basic_solution = form.basic_solution().unwrap();

        assert_eq!(basic_solution.get(0, 0), 4.0);
        assert_eq!(basic_solution.get(1, 0), 6.0);
        assert!(form.is_basic_solution_feasible().unwrap());
    }

    #[test]
    fn identifies_an_infeasible_basic_solution() {
        let problem = parse_problem(
            "min z = x_1 + x_2\n\
             x_1 + x_2 >= 4\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let form = MatrixForm::from_problem(&normalized).unwrap();

        let basic_solution = form.basic_solution().unwrap();

        assert_eq!(basic_solution.get(0, 0), -4.0);
        assert!(!form.is_basic_solution_feasible().unwrap());
    }

    #[test]
    fn calculates_lambda_and_reduced_costs() {
        let problem = parse_problem(
            "min z = -3x_1 - 2x_2\n\
             x_1 + x_2 <= 4\n\
             2x_1 + x_2 <= 5\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let mut form = MatrixForm::from_problem(&normalized).unwrap();

        form.basic_matrix = Matrix::from_rows(vec![vec![1.0, 1.0], vec![2.0, 1.0]]).unwrap();
        form.basic_columns = vec![0, 1];
        form.non_basic_columns = vec![2, 3];

        let basic_costs = form.basic_costs();
        let lambda = form.lambda().unwrap();
        let reduced_costs = form.reduced_costs().unwrap();

        assert_eq!(basic_costs.get(0, 0), -3.0);
        assert_eq!(basic_costs.get(1, 0), -2.0);
        assert_eq!(lambda.get(0, 0), -1.0);
        assert_eq!(lambda.get(1, 0), -1.0);
        assert_eq!(reduced_costs.len(), 2);
        assert_eq!(reduced_costs[0].variable, 3);
        assert_eq!(reduced_costs[0].column, 2);
        assert_eq!(reduced_costs[0].value, 1.0);
        assert!(!reduced_costs[0].improves_objective);
        assert_eq!(reduced_costs[1].variable, 4);
        assert_eq!(reduced_costs[1].column, 3);
        assert_eq!(reduced_costs[1].value, 1.0);
        assert!(!reduced_costs[1].improves_objective);
    }

    #[test]
    fn identifies_non_basic_variable_that_does_not_improve_objective() {
        let problem = parse_problem(
            "min z = 3x_1\n\
             x_1 <= 4\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let form = MatrixForm::from_problem(&normalized).unwrap();

        let reduced_costs = form.reduced_costs().unwrap();

        assert_eq!(reduced_costs[0].value, 3.0);
        assert!(!reduced_costs[0].improves_objective);
    }

    #[test]
    fn chooses_entering_variable_and_calculates_direction() {
        let problem = parse_problem(
            "max z = 5x_1 + 4x_2\n\
             6x_1 + 4x_2 <= 24\n\
             x_1 + 2x_2 <= 6\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let form = MatrixForm::from_problem(&normalized).unwrap();

        let direction = form.direction().unwrap().unwrap();

        assert_eq!(direction.entering_variable, 1);
        assert_eq!(direction.reduced_cost, -5.0);
        assert_eq!(direction.entering_column.get(0, 0), 6.0);
        assert_eq!(direction.entering_column.get(1, 0), 1.0);
        assert_eq!(direction.y.get(0, 0), 6.0);
        assert_eq!(direction.y.get(1, 0), 1.0);

        let new_basic_solution = form.basic_solution_after_step(&direction, 2.0).unwrap();
        assert_eq!(new_basic_solution.get(0, 0), 12.0);
        assert_eq!(new_basic_solution.get(1, 0), 4.0);
    }

    #[test]
    fn has_no_direction_when_no_variable_improves_objective() {
        let problem = parse_problem(
            "min z = 3x_1\n\
             x_1 <= 4\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let form = MatrixForm::from_problem(&normalized).unwrap();

        assert_eq!(form.direction().unwrap(), None);
    }

    #[test]
    fn calculates_ratios_for_positive_direction_values() {
        let problem = parse_problem(
            "max z = 3x_1 + 2x_2\n\
             x_1 + x_2 <= 4\n\
             x_1 <= 2\n\
             x_2 <= 3\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let form = MatrixForm::from_problem(&normalized).unwrap();
        let direction = form.direction().unwrap().unwrap();

        let ratio_test = form.ratio_test(&direction).unwrap();

        assert!(!ratio_test.is_unbounded);
        assert_eq!(ratio_test.ratios.len(), 2);
        assert_eq!(ratio_test.ratios[0].basic_variable, 3);
        assert_eq!(ratio_test.ratios[0].basic_row, 0);
        assert_eq!(ratio_test.ratios[0].basic_value, 4.0);
        assert_eq!(ratio_test.ratios[0].direction_value, 1.0);
        assert_eq!(ratio_test.ratios[0].value, 4.0);
        assert_eq!(ratio_test.ratios[1].basic_variable, 4);
        assert_eq!(ratio_test.ratios[1].basic_row, 1);
        assert_eq!(ratio_test.ratios[1].basic_value, 2.0);
        assert_eq!(ratio_test.ratios[1].direction_value, 1.0);
        assert_eq!(ratio_test.ratios[1].value, 2.0);
    }

    #[test]
    fn identifies_unbounded_problem_when_no_direction_value_is_positive() {
        let problem = parse_problem(
            "max z = x_1\n\
             -x_1 <= 1\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let form = MatrixForm::from_problem(&normalized).unwrap();
        let direction = form.direction().unwrap().unwrap();

        let ratio_test = form.ratio_test(&direction).unwrap();

        assert_eq!(direction.y.get(0, 0), -1.0);
        assert!(ratio_test.ratios.is_empty());
        assert!(ratio_test.is_unbounded);
    }

    #[test]
    fn chooses_leaving_variable_and_changes_basis() {
        let problem = parse_problem(
            "max z = 3x_1 + 2x_2\n\
             x_1 + x_2 <= 4\n\
             x_1 <= 2\n\
             x_2 <= 3\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let mut form = MatrixForm::from_problem(&normalized).unwrap();
        let direction = form.direction().unwrap().unwrap();
        let ratio_test = form.ratio_test(&direction).unwrap();

        let leaving_variable = form.leaving_variable(&ratio_test).unwrap();

        assert_eq!(leaving_variable.variable, 4);
        assert_eq!(leaving_variable.basic_row, 1);
        assert_eq!(leaving_variable.theta, 2.0);

        form.change_basis(&direction, &leaving_variable);

        assert_eq!(form.basic_columns, vec![2, 0, 4]);
        assert_eq!(form.non_basic_columns, vec![3, 1]);
        assert_eq!(form.basic_matrix.get(0, 0), 1.0);
        assert_eq!(form.basic_matrix.get(0, 1), 1.0);
        assert_eq!(form.basic_matrix.get(1, 1), 1.0);
        assert_eq!(form.basic_matrix.get(2, 2), 1.0);
    }

    #[test]
    fn repeats_iterations_until_optimal_solution() {
        let problem = parse_problem(
            "max z = 3x_1 + 2x_2\n\
             x_1 + x_2 <= 4\n\
             x_1 <= 2\n\
             x_2 <= 3\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let form = MatrixForm::from_problem(&normalized).unwrap();

        let result = form.solve_simplex().unwrap();
        let basic_solution = result.form.basic_solution().unwrap();
        let solution = result.form.solution().unwrap();

        assert_eq!(result.status, SimplexStatus::Optimal);
        assert_eq!(result.iterations, 2);
        assert_eq!(result.form.basic_columns, vec![1, 0, 4]);
        assert_eq!(basic_solution.get(0, 0), 2.0);
        assert_eq!(basic_solution.get(1, 0), 2.0);
        assert_eq!(basic_solution.get(2, 0), 1.0);
        assert_eq!(solution.get(0, 0), 2.0);
        assert_eq!(solution.get(1, 0), 2.0);
        assert_eq!(solution.get(2, 0), 0.0);
        assert_eq!(solution.get(3, 0), 0.0);
        assert_eq!(solution.get(4, 0), 1.0);
        assert_eq!(result.form.objective_value().unwrap(), 10.0);
    }

    #[test]
    fn stops_iterations_when_problem_is_unbounded() {
        let problem = parse_problem(
            "max z = x_1\n\
             -x_1 <= 1\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let form = MatrixForm::from_problem(&normalized).unwrap();

        let result = form.solve_simplex().unwrap();

        assert_eq!(result.status, SimplexStatus::Unbounded);
        assert_eq!(result.iterations, 0);
    }

    #[test]
    fn does_not_iterate_with_infeasible_initial_base() {
        let problem = parse_problem(
            "min z = x_1\n\
             x_1 >= 1\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        let form = MatrixForm::from_problem(&normalized).unwrap();

        let result = form.solve_simplex().unwrap();

        assert_eq!(result.status, SimplexStatus::InfeasibleInitialBase);
        assert_eq!(result.iterations, 0);
    }
}

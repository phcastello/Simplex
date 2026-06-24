use crate::matrix::{Matrix, MatrixError};
use crate::matrix_form::MatrixForm;
use crate::problem::EPSILON;

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
    Infeasible,
    IterationLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimplexPhase {
    PhaseOne,
    PhaseTwo,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimplexResult {
    pub status: SimplexStatus,
    pub iterations: usize,
    pub phase_one_iterations: usize,
    pub phase_two_iterations: usize,
    pub phase_one_objective: Option<f64>,
    pub form: MatrixForm,
    pub state: SimplexState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimplexState {
    pub active_costs: Matrix,
    pub basic_columns: Vec<usize>,
    pub non_basic_columns: Vec<usize>,
    pub phase: SimplexPhase,
    pub iterations: usize,
}

pub(crate) fn run_simplex_iterations(
    form: &MatrixForm,
    mut state: SimplexState,
    max_iterations: usize,
) -> Result<SimplexResult, MatrixError> {
    form.debug_validate_state(&state);

    for iteration in 0..max_iterations {
        state.iterations = iteration;

        let direction = match state.direction(form)? {
            Some(direction) => direction,
            None => {
                return Ok(SimplexResult {
                    status: SimplexStatus::Optimal,
                    iterations: state.iterations,
                    phase_one_iterations: if state.phase == SimplexPhase::PhaseOne {
                        state.iterations
                    } else {
                        0
                    },
                    phase_two_iterations: if state.phase == SimplexPhase::PhaseTwo {
                        state.iterations
                    } else {
                        0
                    },
                    phase_one_objective: None,
                    form: form.clone(),
                    state,
                });
            }
        };

        let ratio_test = state.ratio_test(form, &direction)?;
        if ratio_test.is_unbounded {
            return Ok(SimplexResult {
                status: SimplexStatus::Unbounded,
                iterations: state.iterations,
                phase_one_iterations: if state.phase == SimplexPhase::PhaseOne {
                    state.iterations
                } else {
                    0
                },
                phase_two_iterations: if state.phase == SimplexPhase::PhaseTwo {
                    state.iterations
                } else {
                    0
                },
                phase_one_objective: None,
                form: form.clone(),
                state,
            });
        }

        let leaving_variable = match state.leaving_variable(&ratio_test) {
            Some(variable) => variable,
            None => {
                return Ok(SimplexResult {
                    status: SimplexStatus::Unbounded,
                    iterations: state.iterations,
                    phase_one_iterations: if state.phase == SimplexPhase::PhaseOne {
                        state.iterations
                    } else {
                        0
                    },
                    phase_two_iterations: if state.phase == SimplexPhase::PhaseTwo {
                        state.iterations
                    } else {
                        0
                    },
                    phase_one_objective: None,
                    form: form.clone(),
                    state,
                });
            }
        };
        state.change_basis(&direction, &leaving_variable);
        form.debug_validate_state(&state);
    }

    state.iterations = max_iterations;
    Ok(SimplexResult {
        status: SimplexStatus::IterationLimit,
        iterations: state.iterations,
        phase_one_iterations: if state.phase == SimplexPhase::PhaseOne {
            state.iterations
        } else {
            0
        },
        phase_two_iterations: if state.phase == SimplexPhase::PhaseTwo {
            state.iterations
        } else {
            0
        },
        phase_one_objective: None,
        form: form.clone(),
        state,
    })
}

impl SimplexState {
    pub fn basic_solution(&self, form: &MatrixForm) -> Result<Matrix, MatrixError> {
        if form.a.rows() == 0 {
            return Ok(Matrix::new(0, 1, 0.0));
        }

        self.basic_matrix(form).solve(&form.b)
    }

    pub fn solution(&self, form: &MatrixForm) -> Result<Matrix, MatrixError> {
        let basic_solution = self.basic_solution(form)?;
        let mut solution = Matrix::new(form.variables.len(), 1, 0.0);

        for row in 0..self.basic_columns.len() {
            let column = self.basic_columns[row];
            let value = basic_solution.get(row, 0);
            solution.set(column, 0, value);
        }

        Ok(solution)
    }

    pub fn is_basic_solution_feasible(&self, form: &MatrixForm) -> Result<bool, MatrixError> {
        let basic_solution = self.basic_solution(form)?;

        for row in 0..basic_solution.rows() {
            if basic_solution.get(row, 0) < -EPSILON {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn basic_costs(&self) -> Matrix {
        let mut basic_costs = Matrix::new(self.basic_columns.len(), 1, 0.0);

        for row in 0..self.basic_columns.len() {
            let column_in_c = self.basic_columns[row];
            let cost = self.active_costs.get(column_in_c, 0);
            basic_costs.set(row, 0, cost);
        }

        basic_costs
    }

    pub fn lambda(&self, form: &MatrixForm) -> Result<Matrix, MatrixError> {
        let transposed_basic_matrix = self.basic_matrix(form).transpose();
        let basic_costs = self.basic_costs();

        transposed_basic_matrix.solve(&basic_costs)
    }

    pub fn reduced_costs(&self, form: &MatrixForm) -> Result<Vec<ReducedCost>, MatrixError> {
        let lambda = self.lambda(form)?;
        let mut reduced_costs = Vec::new();

        for column in &self.non_basic_columns {
            let variable = form.variables[*column];
            let objective_cost = self.active_costs.get(*column, 0);
            let mut lambda_times_column = 0.0;

            for row in 0..form.a.rows() {
                let lambda_value = lambda.get(row, 0);
                let column_value = form.a.get(row, *column);
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

    pub fn entering_variable(&self, form: &MatrixForm) -> Result<Option<ReducedCost>, MatrixError> {
        let reduced_costs = self.reduced_costs(form)?;
        let mut chosen_variable: Option<ReducedCost> = None;

        for reduced_cost in reduced_costs {
            if !reduced_cost.improves_objective {
                continue;
            }

            let should_choose = match &chosen_variable {
                Some(current) => {
                    reduced_cost.value < current.value - EPSILON
                        || ((reduced_cost.value - current.value).abs() <= EPSILON
                            && reduced_cost.variable < current.variable)
                }
                None => true,
            };

            if should_choose {
                chosen_variable = Some(reduced_cost);
            }
        }

        Ok(chosen_variable)
    }

    pub fn direction(&self, form: &MatrixForm) -> Result<Option<Direction>, MatrixError> {
        let entering_variable = match self.entering_variable(form)? {
            Some(variable) => variable,
            None => return Ok(None),
        };

        let mut entering_column = Matrix::new(form.a.rows(), 1, 0.0);
        for row in 0..form.a.rows() {
            let value = form.a.get(row, entering_variable.column);
            entering_column.set(row, 0, value);
        }

        let y = self.basic_matrix(form).solve(&entering_column)?;

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
        form: &MatrixForm,
        direction: &Direction,
        theta: f64,
    ) -> Result<Matrix, MatrixError> {
        let basic_solution = self.basic_solution(form)?;
        let mut new_basic_solution = Matrix::new(basic_solution.rows(), 1, 0.0);

        for row in 0..basic_solution.rows() {
            let current_value = basic_solution.get(row, 0);
            let direction_value = direction.y.get(row, 0);
            let new_value = current_value - direction_value * theta;
            new_basic_solution.set(row, 0, new_value);
        }

        Ok(new_basic_solution)
    }

    pub fn ratio_test(
        &self,
        form: &MatrixForm,
        direction: &Direction,
    ) -> Result<RatioTest, MatrixError> {
        let basic_solution = self.basic_solution(form)?;
        let mut ratios = Vec::new();

        for row in 0..direction.y.rows() {
            let direction_value = direction.y.get(row, 0);

            if direction_value > EPSILON {
                let basic_column = self.basic_columns[row];
                let basic_variable = form.variables[basic_column];
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
                Some(current) => {
                    ratio.value < current.theta - EPSILON
                        || ((ratio.value - current.theta).abs() <= EPSILON
                            && ratio.basic_variable < current.variable)
                }
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

    pub fn basic_matrix(&self, form: &MatrixForm) -> Matrix {
        let number_of_rows = form.a.rows();
        let number_of_basic_columns = self.basic_columns.len();
        let mut basic_matrix = Matrix::new(number_of_rows, number_of_basic_columns, 0.0);

        for basic_column in 0..number_of_basic_columns {
            let column_in_a = self.basic_columns[basic_column];

            for row in 0..number_of_rows {
                let value = form.a.get(row, column_in_a);
                basic_matrix.set(row, basic_column, value);
            }
        }

        basic_matrix
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
    }

    pub fn active_objective_value(&self, form: &MatrixForm) -> Result<f64, MatrixError> {
        let basic_solution = self.basic_solution(form)?;
        let basic_costs = self.basic_costs();
        let mut value = 0.0;

        for row in 0..basic_solution.rows() {
            value += basic_costs.get(row, 0) * basic_solution.get(row, 0);
        }

        Ok(value)
    }
}

use crate::matrix::MatrixError;
use crate::matrix_form::{MatrixForm, MatrixFormError};
use crate::problem::EPSILON;
use crate::simplex::{
    SimplexPhase, SimplexResult, SimplexState, SimplexStatus, run_simplex_iterations,
};

impl MatrixForm {
    pub fn prepare_current_phase_two_state(&self) -> Result<SimplexState, MatrixFormError> {
        let natural_basis = self.find_natural_slack_basis();

        let mut basic_columns = vec![0; self.a.rows()];
        let mut missing_rows = Vec::new();
        for row in 0..natural_basis.len() {
            match natural_basis[row] {
                Some(column) => basic_columns[row] = column,
                None => missing_rows.push(row),
            }
        }

        if !missing_rows.is_empty() {
            return Err(MatrixFormError::MissingInitialBasis { rows: missing_rows });
        }

        let mut non_basic_columns = Vec::new();
        for column in 0..self.variables.len() {
            if !basic_columns.contains(&column) {
                non_basic_columns.push(column);
            }
        }

        Ok(SimplexState {
            active_costs: self.c.clone(),
            basic_columns,
            non_basic_columns,
            phase: SimplexPhase::PhaseTwo,
            iterations: 0,
        })
    }

    fn phase_two_state_from(&self, previous: &SimplexState) -> SimplexState {
        let mut non_basic_columns = Vec::new();
        for column in 0..self.variables.len() {
            if !previous.basic_columns.contains(&column) {
                non_basic_columns.push(column);
            }
        }

        SimplexState {
            active_costs: self.c.clone(),
            basic_columns: previous.basic_columns.clone(),
            non_basic_columns,
            phase: SimplexPhase::PhaseTwo,
            iterations: 0,
        }
    }

    pub fn solve_simplex(self) -> Result<SimplexResult, MatrixError> {
        const MAX_ITERATIONS: usize = 1_000;

        if self.a.rows() == 0 {
            return Ok(self.solve_without_constraints());
        }

        let natural_basis = self.find_natural_slack_basis();
        let has_missing_basis = natural_basis.iter().any(Option::is_none);

        let (working_form, phase_one_state) = if has_missing_basis {
            self.add_artificial_columns(natural_basis)
        } else {
            let phase_two_state = self
                .prepare_current_phase_two_state()
                .map_err(|_| MatrixError::IncompatibleDimensions)?;
            let result = run_simplex_iterations(&self, phase_two_state, MAX_ITERATIONS)?;
            return Ok(SimplexResult {
                status: result.status,
                iterations: result.state.iterations,
                phase_one_iterations: 0,
                phase_two_iterations: result.state.iterations,
                phase_one_objective: None,
                form: self,
                state: result.state,
            });
        };

        let phase_one_result =
            run_simplex_iterations(&working_form, phase_one_state, MAX_ITERATIONS)?;

        if phase_one_result.status == SimplexStatus::IterationLimit {
            return Ok(phase_one_result);
        }
        if phase_one_result.status == SimplexStatus::Unbounded {
            return Err(MatrixError::InternalPhaseOneUnbounded);
        }

        let w = phase_one_result
            .state
            .active_objective_value(&phase_one_result.form)?;
        if w > EPSILON {
            return Ok(SimplexResult {
                status: SimplexStatus::Infeasible,
                iterations: phase_one_result.state.iterations,
                phase_one_iterations: phase_one_result.state.iterations,
                phase_two_iterations: 0,
                phase_one_objective: Some(w),
                form: phase_one_result.form,
                state: phase_one_result.state,
            });
        }

        let phase_one_iterations = phase_one_result.state.iterations;
        let mut form = phase_one_result.form;
        let mut state = phase_one_result.state;

        if !form.remove_artificials_after_phase_one(&mut state)? {
            return Err(MatrixError::ArtificialRemovalInconsistent);
        }

        let phase_two_state = form.phase_two_state_from(&state);
        let mut phase_two_result = run_simplex_iterations(&form, phase_two_state, MAX_ITERATIONS)?;
        let phase_two_iterations = phase_two_result.state.iterations;
        phase_two_result.state.iterations += phase_one_iterations;
        phase_two_result.iterations = phase_two_result.state.iterations;
        phase_two_result.phase_one_iterations = phase_one_iterations;
        phase_two_result.phase_two_iterations = phase_two_iterations;
        phase_two_result.phase_one_objective = Some(w);

        Ok(phase_two_result)
    }

    fn solve_without_constraints(self) -> SimplexResult {
        let mut non_basic_columns = Vec::new();
        for column in 0..self.variables.len() {
            non_basic_columns.push(column);
        }

        let mut status = SimplexStatus::Optimal;
        for row in 0..self.c.rows() {
            if self.c.get(row, 0) < -EPSILON {
                status = SimplexStatus::Unbounded;
                break;
            }
        }

        let state = SimplexState {
            active_costs: self.c.clone(),
            basic_columns: Vec::new(),
            non_basic_columns,
            phase: SimplexPhase::PhaseTwo,
            iterations: 0,
        };

        SimplexResult {
            status,
            iterations: 0,
            phase_one_iterations: 0,
            phase_two_iterations: 0,
            phase_one_objective: None,
            form: self,
            state,
        }
    }
}

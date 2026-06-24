use crate::matrix::{Matrix, MatrixError};
use crate::matrix_form::MatrixForm;
use crate::problem::{EPSILON, VariableKind};
use crate::simplex::{SimplexPhase, SimplexState};

impl MatrixForm {
    pub(crate) fn add_artificial_columns(
        &self,
        natural_basis: Vec<Option<usize>>,
    ) -> (MatrixForm, SimplexState) {
        let mut missing_rows = Vec::new();
        for (row, basis_column) in natural_basis.iter().enumerate() {
            if basis_column.is_none() {
                missing_rows.push(row);
            }
        }

        let old_columns = self.a.columns();
        let new_columns = old_columns + missing_rows.len();
        let mut a = Matrix::new(self.a.rows(), new_columns, 0.0);

        for row in 0..self.a.rows() {
            for column in 0..self.a.columns() {
                a.set(row, column, self.a.get(row, column));
            }
        }

        let mut variables = self.variables.clone();
        let mut variable_kinds = self.variable_kinds.clone();
        let mut original_costs = Matrix::new(new_columns, 1, 0.0);
        let mut phase_one_costs = Matrix::new(new_columns, 1, 0.0);

        for row in 0..self.c.rows() {
            original_costs.set(row, 0, self.c.get(row, 0));
        }

        let mut highest_variable = variables.iter().copied().max().unwrap_or(0);
        let mut basic_columns = vec![0; self.a.rows()];

        for (row, basis_column) in natural_basis.iter().enumerate() {
            if let Some(column) = basis_column {
                basic_columns[row] = *column;
            }
        }

        for (artificial_index, row) in missing_rows.iter().enumerate() {
            let column = old_columns + artificial_index;

            a.set(*row, column, 1.0);
            highest_variable += 1;
            variables.push(highest_variable);
            variable_kinds.push(VariableKind::Artificial);
            phase_one_costs.set(column, 0, 1.0);
            basic_columns[*row] = column;
        }

        let mut non_basic_columns = Vec::new();
        for column in 0..new_columns {
            if !basic_columns.contains(&column) {
                non_basic_columns.push(column);
            }
        }

        let phase_one_form = MatrixForm {
            a,
            b: self.b.clone(),
            c: original_costs,
            variables,
            variable_kinds,
            fixed_zero_variables: self.fixed_zero_variables.clone(),
            original_sense: self.original_sense,
        };
        let phase_one_state = SimplexState {
            active_costs: phase_one_costs,
            basic_columns,
            non_basic_columns,
            phase: SimplexPhase::PhaseOne,
            iterations: 0,
        };

        (phase_one_form, phase_one_state)
    }

    pub(crate) fn remove_artificials_after_phase_one(
        &mut self,
        state: &mut SimplexState,
    ) -> Result<bool, MatrixError> {
        let mut row = 0;

        while row < state.basic_columns.len() {
            let basic_column = state.basic_columns[row];

            if self.variable_kinds[basic_column] != VariableKind::Artificial {
                row += 1;
                continue;
            }

            let basic_solution = state.basic_solution(self)?;
            let artificial_value = basic_solution.get(row, 0);
            if artificial_value > EPSILON {
                return Ok(false);
            }

            match self.find_replacement_for_artificial(state, row)? {
                Some(replacement_column) => {
                    for position in 0..state.non_basic_columns.len() {
                        if state.non_basic_columns[position] == replacement_column {
                            state.non_basic_columns[position] = basic_column;
                            break;
                        }
                    }
                    state.basic_columns[row] = replacement_column;
                    row += 1;
                }
                None => {
                    self.remove_row(row, state);
                    if !state.non_basic_columns.contains(&basic_column) {
                        state.non_basic_columns.push(basic_column);
                    }
                }
            }
        }

        let mut artificial_columns = Vec::new();
        for column in 0..self.variable_kinds.len() {
            if self.variable_kinds[column] == VariableKind::Artificial {
                artificial_columns.push(column);
            }
        }

        artificial_columns.sort_by(|left, right| right.cmp(left));
        for column in artificial_columns {
            self.remove_column(column, state);
        }

        state.active_costs = self.c.clone();
        state.phase = SimplexPhase::PhaseTwo;
        state.iterations = 0;
        self.debug_validate_state(state);

        Ok(true)
    }

    fn find_replacement_for_artificial(
        &self,
        state: &SimplexState,
        artificial_row: usize,
    ) -> Result<Option<usize>, MatrixError> {
        let basic_matrix = state.basic_matrix(self);

        for column in &state.non_basic_columns {
            if self.variable_kinds[*column] == VariableKind::Artificial {
                continue;
            }

            let column_matrix = self.column_matrix(*column);
            let direction = basic_matrix.solve(&column_matrix)?;
            if direction.get(artificial_row, 0).abs() > EPSILON {
                return Ok(Some(*column));
            }
        }

        Ok(None)
    }

    fn column_matrix(&self, column: usize) -> Matrix {
        let mut result = Matrix::new(self.a.rows(), 1, 0.0);

        for row in 0..self.a.rows() {
            result.set(row, 0, self.a.get(row, column));
        }

        result
    }

    fn remove_row(&mut self, removed_row: usize, state: &mut SimplexState) {
        let mut new_a = Matrix::new(self.a.rows() - 1, self.a.columns(), 0.0);
        let mut new_b = Matrix::new(self.b.rows() - 1, 1, 0.0);
        let mut new_row = 0;

        for row in 0..self.a.rows() {
            if row == removed_row {
                continue;
            }

            for column in 0..self.a.columns() {
                new_a.set(new_row, column, self.a.get(row, column));
            }
            new_b.set(new_row, 0, self.b.get(row, 0));
            new_row += 1;
        }

        self.a = new_a;
        self.b = new_b;
        state.basic_columns.remove(removed_row);
    }

    fn remove_column(&mut self, removed_column: usize, state: &mut SimplexState) {
        let mut new_a = Matrix::new(self.a.rows(), self.a.columns() - 1, 0.0);
        for row in 0..self.a.rows() {
            let mut new_column = 0;
            for column in 0..self.a.columns() {
                if column == removed_column {
                    continue;
                }
                new_a.set(row, new_column, self.a.get(row, column));
                new_column += 1;
            }
        }

        self.a = new_a;
        self.c = remove_cost_row(&self.c, removed_column);
        state.active_costs = remove_cost_row(&state.active_costs, removed_column);
        self.variables.remove(removed_column);
        self.variable_kinds.remove(removed_column);

        for column in &mut state.basic_columns {
            if *column > removed_column {
                *column -= 1;
            }
        }

        let mut new_non_basic_columns = Vec::new();
        for column in &state.non_basic_columns {
            if *column == removed_column {
                continue;
            }
            if *column > removed_column {
                new_non_basic_columns.push(*column - 1);
            } else {
                new_non_basic_columns.push(*column);
            }
        }
        state.non_basic_columns = new_non_basic_columns;
        self.debug_validate_state(state);
    }
}

fn remove_cost_row(costs: &Matrix, removed_row: usize) -> Matrix {
    let mut result = Matrix::new(costs.rows() - 1, 1, 0.0);
    let mut new_row = 0;

    for row in 0..costs.rows() {
        if row == removed_row {
            continue;
        }

        result.set(new_row, 0, costs.get(row, 0));
        new_row += 1;
    }

    result
}

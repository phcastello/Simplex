use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display};

use crate::matrix::MatrixError;
use crate::matrix_form::{MatrixForm, MatrixFormError, SimplexResult, SimplexStatus};
use crate::normalizer::{NormalizeError, normalize};
use crate::problem::{
    Constraint, EPSILON, Problem, Relation, Sense, Term, VariableBound, VariableKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchAndBoundStatus {
    Optimal,
    Infeasible,
    NodeLimit,
    SimplexIterationLimit,
    RelaxationUnbounded,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntegerSolution {
    pub objective_value: f64,
    pub variable_values: BTreeMap<usize, f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BranchConstraint {
    pub variable: usize,
    pub relation: Relation,
    pub rhs: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeFinalAction {
    Branched,
    PrunedInfeasible,
    PrunedByBound,
    IntegerSolution,
    IterationLimit,
    RelaxationUnbounded,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeReport {
    pub id: usize,
    pub parent_id: Option<usize>,
    pub depth: usize,
    pub branch_constraint: Option<BranchConstraint>,
    pub relaxation_status: SimplexStatus,
    pub relaxation_objective: Option<f64>,
    pub original_variable_values: BTreeMap<usize, f64>,
    pub branched_variable: Option<usize>,
    pub left_bound: Option<f64>,
    pub right_bound: Option<f64>,
    pub final_action: NodeFinalAction,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchAndBoundResult {
    pub status: BranchAndBoundStatus,
    pub solution: Option<IntegerSolution>,
    pub explored_nodes: usize,
    pub created_nodes: usize,
    pub pruned_infeasible: usize,
    pub pruned_by_bound: usize,
    pub integer_solution_nodes: usize,
    pub total_simplex_iterations: usize,
    pub node_reports: Vec<NodeReport>,
}

#[derive(Debug)]
pub enum BranchAndBoundError {
    DuplicateIntegerVariable { variable: usize },
    InvalidIntegerVariable { variable: usize },
    Normalize(NormalizeError),
    MatrixForm(MatrixFormError),
    Matrix(MatrixError),
    NumericalInconsistency { message: String },
}

impl Display for BranchAndBoundError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateIntegerVariable { variable } => {
                write!(
                    formatter,
                    "x_{variable} aparece mais de uma vez na lista de inteiras"
                )
            }
            Self::InvalidIntegerVariable { variable } => write!(
                formatter,
                "x_{variable} não existe no problema original ou não é uma variável original"
            ),
            Self::Normalize(error) => write!(formatter, "erro ao normalizar nó: {error}"),
            Self::MatrixForm(error) => {
                write!(
                    formatter,
                    "erro ao converter nó para forma matricial: {error}"
                )
            }
            Self::Matrix(error) => write!(formatter, "erro matricial ao resolver nó: {error}"),
            Self::NumericalInconsistency { message } => {
                write!(formatter, "inconsistência numérica: {message}")
            }
        }
    }
}

impl Error for BranchAndBoundError {}

impl From<NormalizeError> for BranchAndBoundError {
    fn from(error: NormalizeError) -> Self {
        Self::Normalize(error)
    }
}

impl From<MatrixFormError> for BranchAndBoundError {
    fn from(error: MatrixFormError) -> Self {
        Self::MatrixForm(error)
    }
}

impl From<MatrixError> for BranchAndBoundError {
    fn from(error: MatrixError) -> Self {
        Self::Matrix(error)
    }
}

#[derive(Clone, Debug)]
struct BranchNode {
    id: usize,
    parent_id: Option<usize>,
    depth: usize,
    problem: Problem,
    branch_constraint: Option<BranchConstraint>,
}

pub fn solve_all_integer_variables(
    problem: &Problem,
    max_nodes: usize,
) -> Result<BranchAndBoundResult, BranchAndBoundError> {
    let mut integer_variables = Vec::new();

    for (variable, kind) in &problem.variable_kinds {
        if *kind == VariableKind::Original {
            integer_variables.push(*variable);
        }
    }

    solve_branch_and_bound(problem, &integer_variables, max_nodes)
}

pub fn solve_branch_and_bound(
    problem: &Problem,
    integer_variables: &[usize],
    max_nodes: usize,
) -> Result<BranchAndBoundResult, BranchAndBoundError> {
    let integer_variables = validate_integer_variables(problem, integer_variables)?;

    let mut stack = vec![BranchNode {
        id: 0,
        parent_id: None,
        depth: 0,
        problem: problem.clone(),
        branch_constraint: None,
    }];

    let mut result = BranchAndBoundResult {
        status: BranchAndBoundStatus::Infeasible,
        solution: None,
        explored_nodes: 0,
        created_nodes: 1,
        pruned_infeasible: 0,
        pruned_by_bound: 0,
        integer_solution_nodes: 0,
        total_simplex_iterations: 0,
        node_reports: Vec::new(),
    };

    let mut next_node_id = 1;

    while !stack.is_empty() {
        if result.explored_nodes >= max_nodes {
            result.status = BranchAndBoundStatus::NodeLimit;
            return Ok(result);
        }

        let node = stack.pop().expect("a pilha foi testada como não vazia");
        result.explored_nodes += 1;

        let simplex_result = solve_relaxation(&node.problem)?;
        result.total_simplex_iterations += simplex_result.iterations;

        match simplex_result.status {
            SimplexStatus::Infeasible => {
                result.pruned_infeasible += 1;
                result.node_reports.push(make_report(
                    &node,
                    &simplex_result,
                    None,
                    None,
                    None,
                    None,
                    NodeFinalAction::PrunedInfeasible,
                )?);
            }
            SimplexStatus::IterationLimit => {
                result.node_reports.push(make_report(
                    &node,
                    &simplex_result,
                    None,
                    None,
                    None,
                    None,
                    NodeFinalAction::IterationLimit,
                )?);
                result.status = BranchAndBoundStatus::SimplexIterationLimit;
                return Ok(result);
            }
            SimplexStatus::Unbounded => {
                result.node_reports.push(make_report(
                    &node,
                    &simplex_result,
                    None,
                    None,
                    None,
                    None,
                    NodeFinalAction::RelaxationUnbounded,
                )?);
                result.status = BranchAndBoundStatus::RelaxationUnbounded;
                return Ok(result);
            }
            SimplexStatus::Optimal => {
                process_optimal_node(
                    problem,
                    &integer_variables,
                    node,
                    simplex_result,
                    &mut result,
                    &mut next_node_id,
                    &mut stack,
                )?;
            }
        }
    }

    if result.solution.is_some() {
        result.status = BranchAndBoundStatus::Optimal;
    } else {
        result.status = BranchAndBoundStatus::Infeasible;
    }

    Ok(result)
}

fn process_optimal_node(
    original_problem: &Problem,
    integer_variables: &[usize],
    node: BranchNode,
    simplex_result: SimplexResult,
    result: &mut BranchAndBoundResult,
    next_node_id: &mut usize,
    stack: &mut Vec<BranchNode>,
) -> Result<(), BranchAndBoundError> {
    let node_bound = simplex_result.form.objective_value(&simplex_result.state)?;

    if cannot_improve_incumbent(
        original_problem.original_sense,
        node_bound,
        result.solution.as_ref(),
    ) {
        result.pruned_by_bound += 1;
        result.node_reports.push(make_report(
            &node,
            &simplex_result,
            Some(node_bound),
            None,
            None,
            None,
            NodeFinalAction::PrunedByBound,
        )?);
        return Ok(());
    }

    let original_values = extract_original_values(&simplex_result)?;
    match choose_fractional_variable(&original_values, integer_variables) {
        None => {
            let candidate_values = rounded_integer_solution(&original_values, integer_variables);
            if !satisfies_original_constraints(original_problem, &candidate_values) {
                return Err(BranchAndBoundError::NumericalInconsistency {
                    message: "solução inteira arredondada viola as restrições originais"
                        .to_string(),
                });
            }

            let objective_value = calculate_original_objective(original_problem, &candidate_values);
            let candidate = IntegerSolution {
                objective_value,
                variable_values: candidate_values,
            };

            if is_better_solution(
                original_problem.original_sense,
                candidate.objective_value,
                result.solution.as_ref(),
            ) {
                result.solution = Some(candidate);
            }

            result.integer_solution_nodes += 1;
            result.node_reports.push(make_report(
                &node,
                &simplex_result,
                Some(node_bound),
                None,
                None,
                None,
                NodeFinalAction::IntegerSolution,
            )?);
        }
        Some(variable) => {
            let value = *original_values.get(&variable).ok_or_else(|| {
                BranchAndBoundError::NumericalInconsistency {
                    message: format!("x_{variable} não aparece na solução da relaxação"),
                }
            })?;
            let left_bound = value.floor();
            let right_bound = left_bound + 1.0;

            let left_child = create_child(
                &node,
                *next_node_id,
                variable,
                Relation::LessOrEqual,
                left_bound,
            );
            *next_node_id += 1;

            let right_child = create_child(
                &node,
                *next_node_id,
                variable,
                Relation::GreaterOrEqual,
                right_bound,
            );
            *next_node_id += 1;

            result.created_nodes += 2;
            result.node_reports.push(make_report(
                &node,
                &simplex_result,
                Some(node_bound),
                Some(variable),
                Some(left_bound),
                Some(right_bound),
                NodeFinalAction::Branched,
            )?);

            stack.push(right_child);
            stack.push(left_child);
        }
    }

    Ok(())
}

fn validate_integer_variables(
    problem: &Problem,
    integer_variables: &[usize],
) -> Result<Vec<usize>, BranchAndBoundError> {
    let mut seen = BTreeSet::new();
    let mut validated = Vec::new();

    for variable in integer_variables {
        if !seen.insert(*variable) {
            return Err(BranchAndBoundError::DuplicateIntegerVariable {
                variable: *variable,
            });
        }

        if problem.variable_kinds.get(variable) != Some(&VariableKind::Original) {
            return Err(BranchAndBoundError::InvalidIntegerVariable {
                variable: *variable,
            });
        }

        validated.push(*variable);
    }

    validated.sort();
    Ok(validated)
}

fn solve_relaxation(problem: &Problem) -> Result<SimplexResult, BranchAndBoundError> {
    let normalized = normalize(problem)?;
    let form = MatrixForm::from_problem(&normalized)?;
    Ok(form.solve_simplex()?)
}

fn extract_original_values(
    result: &SimplexResult,
) -> Result<BTreeMap<usize, f64>, BranchAndBoundError> {
    let solution = result.state.solution(&result.form)?;
    let mut values = BTreeMap::new();

    for column in 0..result.form.variables.len() {
        if result.form.variable_kinds[column] == VariableKind::Original {
            let variable = result.form.variables[column];
            values.insert(variable, clean_zero(solution.get(column, 0)));
        }
    }

    for variable in &result.form.fixed_zero_variables {
        values.insert(*variable, 0.0);
    }

    Ok(values)
}

fn choose_fractional_variable(
    values: &BTreeMap<usize, f64>,
    integer_variables: &[usize],
) -> Option<usize> {
    for variable in integer_variables {
        let value = values.get(variable).copied().unwrap_or(0.0);
        if !is_integral_value(value) {
            return Some(*variable);
        }
    }

    None
}

fn rounded_integer_solution(
    values: &BTreeMap<usize, f64>,
    integer_variables: &[usize],
) -> BTreeMap<usize, f64> {
    let mut rounded = values.clone();

    for variable in integer_variables {
        let value = rounded.get(variable).copied().unwrap_or(0.0);
        if is_integral_value(value) {
            rounded.insert(*variable, clean_zero(value.round()));
        }
    }

    rounded
}

fn calculate_original_objective(problem: &Problem, values: &BTreeMap<usize, f64>) -> f64 {
    let mut objective_value = 0.0;

    for term in &problem.objective {
        let variable_value = values.get(&term.variable).copied().unwrap_or(0.0);
        objective_value += term.coefficient * variable_value;
    }

    clean_zero(objective_value)
}

fn satisfies_original_constraints(problem: &Problem, values: &BTreeMap<usize, f64>) -> bool {
    for constraint in &problem.constraints {
        let mut lhs = 0.0;

        for term in &constraint.terms {
            let variable_value = values.get(&term.variable).copied().unwrap_or(0.0);
            lhs += term.coefficient * variable_value;
        }

        let satisfied = match constraint.relation {
            Relation::LessOrEqual => lhs <= constraint.rhs + EPSILON,
            Relation::GreaterOrEqual => lhs >= constraint.rhs - EPSILON,
            Relation::Equal => (lhs - constraint.rhs).abs() <= EPSILON,
        };

        if !satisfied {
            return false;
        }
    }

    satisfies_original_bounds(problem, values)
}

fn satisfies_original_bounds(problem: &Problem, values: &BTreeMap<usize, f64>) -> bool {
    for (variable, bound) in &problem.variable_bounds {
        let value = values.get(variable).copied().unwrap_or(0.0);

        let satisfied = match bound {
            VariableBound::NonNegative => value >= -EPSILON,
            VariableBound::NonPositive => value <= EPSILON,
            VariableBound::FixedZero => value.abs() <= EPSILON,
        };

        if !satisfied {
            return false;
        }
    }

    true
}

fn is_better_solution(
    sense: Sense,
    candidate_value: f64,
    incumbent: Option<&IntegerSolution>,
) -> bool {
    let Some(incumbent) = incumbent else {
        return true;
    };

    match sense {
        Sense::Max => candidate_value > incumbent.objective_value + EPSILON,
        Sense::Min => candidate_value < incumbent.objective_value - EPSILON,
    }
}

fn cannot_improve_incumbent(
    sense: Sense,
    node_bound: f64,
    incumbent: Option<&IntegerSolution>,
) -> bool {
    let Some(incumbent) = incumbent else {
        return false;
    };

    match sense {
        Sense::Max => node_bound <= incumbent.objective_value + EPSILON,
        Sense::Min => node_bound >= incumbent.objective_value - EPSILON,
    }
}

fn create_child(
    parent: &BranchNode,
    id: usize,
    variable: usize,
    relation: Relation,
    rhs: f64,
) -> BranchNode {
    let mut problem = parent.problem.clone();
    problem.constraints.push(Constraint {
        terms: vec![Term {
            variable,
            coefficient: 1.0,
        }],
        relation,
        rhs,
    });

    BranchNode {
        id,
        parent_id: Some(parent.id),
        depth: parent.depth + 1,
        problem,
        branch_constraint: Some(BranchConstraint {
            variable,
            relation,
            rhs,
        }),
    }
}

fn make_report(
    node: &BranchNode,
    simplex_result: &SimplexResult,
    relaxation_objective: Option<f64>,
    branched_variable: Option<usize>,
    left_bound: Option<f64>,
    right_bound: Option<f64>,
    final_action: NodeFinalAction,
) -> Result<NodeReport, BranchAndBoundError> {
    let original_variable_values = if simplex_result.status == SimplexStatus::Optimal {
        extract_original_values(simplex_result)?
    } else {
        BTreeMap::new()
    };

    Ok(NodeReport {
        id: node.id,
        parent_id: node.parent_id,
        depth: node.depth,
        branch_constraint: node.branch_constraint,
        relaxation_status: simplex_result.status,
        relaxation_objective,
        original_variable_values,
        branched_variable,
        left_bound,
        right_bound,
        final_action,
    })
}

fn is_integral_value(value: f64) -> bool {
    (value - value.round()).abs() <= EPSILON
}

fn clean_zero(value: f64) -> f64 {
    if value.abs() < EPSILON { 0.0 } else { value }
}

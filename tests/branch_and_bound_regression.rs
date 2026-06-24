use simplex::branch_and_bound::{
    BranchAndBoundStatus, NodeFinalAction, solve_all_integer_variables, solve_branch_and_bound,
};
use simplex::problem::EPSILON;
use simplex::problem_parser::parse_problem;

const TOLERANCE: f64 = 1e-6;

fn assert_close(label: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "{label}: esperado {expected}, obtido {actual}"
    );
}

fn value(result: &simplex::branch_and_bound::BranchAndBoundResult, variable: usize) -> f64 {
    *result
        .solution
        .as_ref()
        .unwrap()
        .variable_values
        .get(&variable)
        .unwrap()
}

#[test]
fn linear_relaxation_is_already_integer() {
    let problem = parse_problem(
        "max z = 3x_1 + 2x_2\n\
         x_1 <= 2\n\
         x_2 <= 3\n",
    )
    .unwrap();

    let result = solve_all_integer_variables(&problem, 100).unwrap();

    assert_eq!(result.status, BranchAndBoundStatus::Optimal);
    assert_eq!(result.explored_nodes, 1);
    assert_eq!(result.created_nodes, 1);
    assert!(
        result
            .node_reports
            .iter()
            .all(|report| report.final_action != NodeFinalAction::Branched)
    );
    assert_close("z", result.solution.as_ref().unwrap().objective_value, 12.0);
    assert_close("x_1", value(&result, 1), 2.0);
    assert_close("x_2", value(&result, 2), 3.0);
}

#[test]
fn classic_integer_example_from_notes() {
    let problem = parse_problem(
        "max z = 5x_1 + 8x_2\n\
         x_1 + x_2 <= 6\n\
         5x_1 + 9x_2 <= 45\n",
    )
    .unwrap();

    let result = solve_branch_and_bound(&problem, &[1, 2], 1_000).unwrap();

    assert_eq!(result.status, BranchAndBoundStatus::Optimal);
    assert!(result.created_nodes > 1);
    assert_close("z", result.solution.as_ref().unwrap().objective_value, 40.0);
    assert_close("x_1", value(&result, 1), 0.0);
    assert_close("x_2", value(&result, 2), 5.0);
}

#[test]
fn second_integer_example() {
    let problem = parse_problem(
        "max z = 5x_1 + 4x_2\n\
         x_1 + x_2 <= 5\n\
         10x_1 + 6x_2 <= 45\n",
    )
    .unwrap();

    let result = solve_branch_and_bound(&problem, &[1, 2], 1_000).unwrap();

    assert_eq!(result.status, BranchAndBoundStatus::Optimal);
    assert_close("z", result.solution.as_ref().unwrap().objective_value, 23.0);
    assert_close("x_1", value(&result, 1), 3.0);
    assert_close("x_2", value(&result, 2), 2.0);
}

#[test]
fn minimization_uses_minimization_bounds() {
    let problem = parse_problem(
        "min z = -5x_1 - x_2\n\
         7x_1 - 5x_2 <= 13\n\
         3x_1 + 2x_2 <= 17\n",
    )
    .unwrap();

    let result = solve_branch_and_bound(&problem, &[1, 2], 1_000).unwrap();

    assert_eq!(result.status, BranchAndBoundStatus::Optimal);
    assert_close(
        "z",
        result.solution.as_ref().unwrap().objective_value,
        -19.0,
    );
    assert_close("x_1", value(&result, 1), 3.0);
    assert_close("x_2", value(&result, 2), 4.0);
}

#[test]
fn feasible_relaxation_but_integer_problem_is_infeasible() {
    let problem = parse_problem(
        "min z = x_1\n\
         x_1 = 0.5\n",
    )
    .unwrap();

    let result = solve_branch_and_bound(&problem, &[1], 100).unwrap();

    assert_eq!(result.status, BranchAndBoundStatus::Infeasible);
    assert!(result.solution.is_none());
    assert_eq!(result.pruned_infeasible, 2);
}

#[test]
fn fixed_zero_original_variable_is_integral_and_not_branched() {
    let problem = parse_problem(
        "max z = x_1 + x_2\n\
         x_1 <= 1.5\n\
         x_1 >= 0\n\
         x_2 <= 0\n\
         x_2 >= 0\n",
    )
    .unwrap();

    let result = solve_all_integer_variables(&problem, 100).unwrap();

    assert_eq!(result.status, BranchAndBoundStatus::Optimal);
    assert_close("x_2", value(&result, 2), 0.0);
    assert!(
        result
            .node_reports
            .iter()
            .all(|report| report.branched_variable != Some(2))
    );
}

#[test]
fn only_selected_variables_are_integer() {
    let problem = parse_problem(
        "max z = x_1 + x_2\n\
         x_1 <= 2.5\n\
         x_2 <= 2.5\n\
         x_1 + x_2 <= 4.8\n",
    )
    .unwrap();

    let result = solve_branch_and_bound(&problem, &[1], 100).unwrap();

    assert_eq!(result.status, BranchAndBoundStatus::Optimal);
    assert_close("x_1", value(&result, 1), 2.0);
    assert!((value(&result, 2) - value(&result, 2).round()).abs() > EPSILON);
    assert!(
        result
            .node_reports
            .iter()
            .all(|report| report.branched_variable != Some(2))
    );
}

#[test]
fn node_limit_preserves_incumbent_without_claiming_optimality() {
    let problem = parse_problem(
        "max z = x_1\n\
         x_1 <= 1.5\n",
    )
    .unwrap();

    let result = solve_branch_and_bound(&problem, &[1], 2).unwrap();

    assert_eq!(result.status, BranchAndBoundStatus::NodeLimit);
    assert!(result.solution.is_some());
    assert_close(
        "incumbent z",
        result.solution.as_ref().unwrap().objective_value,
        1.0,
    );
}

#[test]
fn branches_only_on_original_integer_variables() {
    let problem = parse_problem(
        "max z = 5x_1 + 8x_2\n\
         x_1 + x_2 <= 6\n\
         5x_1 + 9x_2 <= 45\n",
    )
    .unwrap();

    let result = solve_branch_and_bound(&problem, &[1, 2], 1_000).unwrap();

    for report in &result.node_reports {
        if let Some(variable) = report.branched_variable {
            assert!(
                variable == 1 || variable == 2,
                "ramificou sobre variável auxiliar x_{variable}"
            );
        }
    }
}

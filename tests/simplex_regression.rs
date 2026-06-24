use simplex::matrix_form::{MatrixForm, SimplexResult, SimplexStatus};
use simplex::normalizer::normalize;
use simplex::problem::{Constraint, EPSILON, Problem, Relation, VariableKind};
use simplex::problem_parser::parse_problem;

const ITERATION_LIMIT: usize = 1_000;
const TOLERANCE: f64 = 1e-6;

struct ExpectedOptimal<'a> {
    name: &'static str,
    text: &'a str,
    z: f64,
    original_variables: &'static [(usize, f64)],
}

fn solve(text: &str) -> (Problem, SimplexResult) {
    let problem = parse_problem(text).unwrap();
    let normalized = normalize(&problem).unwrap();
    let form = MatrixForm::from_problem(&normalized).unwrap();
    let result = form.solve_simplex().unwrap();

    (problem, result)
}

fn value_of_variable(result: &SimplexResult, variable: usize) -> f64 {
    if result.form.fixed_zero_variables.contains(&variable) {
        return 0.0;
    }

    let solution = result.state.solution(&result.form).unwrap();

    for column in 0..result.form.variables.len() {
        if result.form.variables[column] == variable {
            return solution.get(column, 0);
        }
    }

    panic!("x_{variable} não encontrada na solução");
}

fn constraint_value(constraint: &Constraint, result: &SimplexResult) -> f64 {
    let mut value = 0.0;

    for term in &constraint.terms {
        value += term.coefficient * value_of_variable(result, term.variable);
    }

    value
}

fn assert_original_constraints_are_satisfied(problem: &Problem, result: &SimplexResult) {
    for constraint in &problem.constraints {
        let value = constraint_value(constraint, result);

        match constraint.relation {
            Relation::LessOrEqual => {
                assert!(value <= constraint.rhs + EPSILON);
            }
            Relation::GreaterOrEqual => {
                assert!(value >= constraint.rhs - EPSILON);
            }
            Relation::Equal => {
                assert!((value - constraint.rhs).abs() <= EPSILON);
            }
        }
    }
}

fn assert_close(label: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "{label}: esperado {expected}, obtido {actual}"
    );
}

fn assert_optimal_case(case: ExpectedOptimal<'_>) {
    let (problem, result) = solve(case.text);

    assert_eq!(result.status, SimplexStatus::Optimal, "{}", case.name);
    assert!(result.iterations < ITERATION_LIMIT, "{}", case.name);
    assert_close(
        case.name,
        result.form.objective_value(&result.state).unwrap(),
        case.z,
    );

    for (variable, expected_value) in case.original_variables {
        let label = format!("{} x_{}", case.name, variable);
        assert_close(
            &label,
            value_of_variable(&result, *variable),
            *expected_value,
        );
    }

    assert_original_constraints_are_satisfied(&problem, &result);
}

fn problem_text_from_data_file(case_name: &str) -> String {
    let data = include_str!("../data/testes_simplex.txt");
    let mut found_case = false;
    let mut lines = Vec::new();

    for line in data.lines() {
        if line.starts_with(case_name) {
            found_case = true;
            continue;
        }

        if found_case {
            if line.trim().is_empty() {
                break;
            }
            lines.push(line);
        }
    }

    assert!(
        found_case,
        "{case_name} não encontrado em data/testes_simplex.txt"
    );
    lines.join("\n")
}

fn assert_no_artificial_columns(result: &SimplexResult) {
    assert!(
        result
            .form
            .variable_kinds
            .iter()
            .all(|kind| *kind != VariableKind::Artificial)
    );
}

#[test]
fn phase_one_test_a_single_greater_or_equal_constraint() {
    let (problem, result) = solve(
        "min z = x_1\n\
         x_1 >= 1\n",
    );

    assert_eq!(result.status, SimplexStatus::Optimal);
    assert!(result.phase_one_iterations < ITERATION_LIMIT);
    assert!(result.phase_two_iterations < ITERATION_LIMIT);
    assert_eq!(
        result
            .phase_one_objective
            .map(|value| value.abs() <= EPSILON),
        Some(true)
    );
    assert!(result.form.variable_kinds.contains(&VariableKind::Excess));
    assert_no_artificial_columns(&result);
    assert_close(
        "teste A z",
        result.form.objective_value(&result.state).unwrap(),
        1.0,
    );
    assert_close("teste A x_1", value_of_variable(&result, 1), 1.0);
    assert_original_constraints_are_satisfied(&problem, &result);
}

#[test]
fn phase_one_test_b_feasible_equality() {
    let (problem, result) = solve(
        "min z = x_1\n\
         x_1 = 2\n",
    );

    assert_eq!(result.status, SimplexStatus::Optimal);
    assert!(result.phase_one_iterations < ITERATION_LIMIT);
    assert_eq!(
        result
            .phase_one_objective
            .map(|value| value.abs() <= EPSILON),
        Some(true)
    );
    assert_no_artificial_columns(&result);
    assert_close(
        "teste B z",
        result.form.objective_value(&result.state).unwrap(),
        2.0,
    );
    assert_close("teste B x_1", value_of_variable(&result, 1), 2.0);
    assert_original_constraints_are_satisfied(&problem, &result);
}

#[test]
fn phase_one_test_c_simple_infeasible_problem() {
    let (_problem, result) = solve(
        "min z = x_1\n\
         x_1 >= 2\n\
         x_1 <= 1\n",
    );

    assert_eq!(result.status, SimplexStatus::Infeasible);
    assert!(result.phase_one_objective.unwrap() > EPSILON);
}

#[test]
fn phase_one_test_d_known_optimum_1_l() {
    let (problem, result) = solve(
        "max z = 3x_1 + 3x_2 + 0x_3\n\
         x_1 + 3x_3 <= 5\n\
         x_2 <= 5\n\
         3x_1 + 2x_3 >= 6\n\
         x_1 + x_2 <= 10\n",
    );

    assert_eq!(result.status, SimplexStatus::Optimal);
    assert_close(
        "teste D z",
        result.form.objective_value(&result.state).unwrap(),
        30.0,
    );
    assert_close("teste D x_1", value_of_variable(&result, 1), 5.0);
    assert_close("teste D x_2", value_of_variable(&result, 2), 5.0);
    assert_close("teste D x_3", value_of_variable(&result, 3), 0.0);
    assert_no_artificial_columns(&result);
    assert_original_constraints_are_satisfied(&problem, &result);
}

#[test]
fn phase_one_test_e_1_m_is_infeasible() {
    let (_problem, result) = solve(
        "max z = 4x_1 + 3x_2\n\
         x_1 + 3x_2 <= 7\n\
         2x_1 + 2x_2 = 8\n\
         x_1 + x_2 <= -3\n\
         x_2 <= 2\n",
    );

    assert_eq!(result.status, SimplexStatus::Infeasible);
    assert!(result.phase_one_objective.unwrap() > EPSILON);
}

#[test]
fn phase_one_test_f_2_i_has_optimum_15() {
    let (problem, result) = solve(
        "max z = 3x_1 + 3x_2 + 13x_3\n\
         -3x_1 + 6x_2 + 7x_3 <= 8\n\
         6x_1 - 3x_2 + 7x_3 <= 8\n\
         x_1 <= 2\n\
         x_3 >= 1\n",
    );

    assert_eq!(result.status, SimplexStatus::Optimal);
    assert_close(
        "teste F z",
        result.form.objective_value(&result.state).unwrap(),
        15.0,
    );
    assert_close("teste F x_1", value_of_variable(&result, 1), 1.0 / 3.0);
    assert_close("teste F x_2", value_of_variable(&result, 2), 1.0 / 3.0);
    assert_close("teste F x_3", value_of_variable(&result, 3), 1.0);
    assert_no_artificial_columns(&result);
    assert_original_constraints_are_satisfied(&problem, &result);
}

#[test]
fn phase_one_test_g_2_l_fixes_x3_at_zero() {
    let (problem, result) = solve(
        "max z = 3x_1 + 3x_2 + 13x_3\n\
         -3x_1 + 6x_2 + 7x_3 <= 8\n\
         6x_1 - 3x_2 + 7x_3 <= 8\n\
         x_1 <= 2\n\
         x_3 <= 0\n\
         x_1, x_2, x_3 >= 0\n",
    );

    assert_eq!(result.status, SimplexStatus::Optimal);
    assert_close(
        "teste G z",
        result.form.objective_value(&result.state).unwrap(),
        13.0,
    );
    assert_close("teste G x_1", value_of_variable(&result, 1), 2.0);
    assert_close("teste G x_2", value_of_variable(&result, 2), 7.0 / 3.0);
    assert_close("teste G x_3", value_of_variable(&result, 3), 0.0);
    assert_original_constraints_are_satisfied(&problem, &result);
}

#[test]
fn freezes_manual_optimal_cases_that_need_phase_one() {
    let cases = [
        ExpectedOptimal {
            name: "1_g",
            text: "min z = -1x_1 + 2x_2\n\
                   x_1 + x_2 >= 1\n\
                   -5x_1 + 2x_2 >= -10\n\
                   3x_1 + 5x_2 >= 15\n",
            z: 10.0 / 31.0,
            original_variables: &[(1, 80.0 / 31.0), (2, 45.0 / 31.0)],
        },
        ExpectedOptimal {
            name: "1_j",
            text: "min z = 4x_1 - 12x_2\n\
                   2x_1 + x_2 >= 6\n\
                   x_1 + 3x_2 <= 8\n\
                   x_1 >= 4\n",
            z: 0.0,
            original_variables: &[(1, 4.0), (2, 4.0 / 3.0)],
        },
        ExpectedOptimal {
            name: "1_k",
            text: "min z = -1x_1 -1x_2 + 0x_3\n\
                   x_1 + x_3 >= 1\n\
                   x_1 -3x_2 -1x_3 >= 1\n\
                   x_1 -1x_2 + 5x_3 >= 5\n\
                   x_1 + x_2 + x_3 <= 5\n",
            z: -5.0,
            original_variables: &[(1, 5.0), (2, 0.0), (3, 0.0)],
        },
    ];

    for case in cases {
        assert_optimal_case(case);
    }
}

#[test]
fn freezes_manual_unbounded_and_infeasible_cases() {
    let (_problem, result) = solve(
        "max z = 2x_1 + 2x_2\n\
         -0.5x_1 + x_2 <= 2\n\
         x_1 -1x_2 >= -1\n",
    );
    assert_eq!(result.status, SimplexStatus::Unbounded, "1_h");
    assert_eq!(result.phase_one_objective, None, "1_h não deve usar Fase I");

    let (_problem, result) = solve(
        "max z = 4x_1 + 3x_2\n\
         x_1 + 3x_2 <= 7\n\
         2x_1 + 2x_2 = 8\n\
         x_1 + x_2 <= -3\n\
         x_2 <= 2\n",
    );
    assert_eq!(result.status, SimplexStatus::Infeasible, "2_f");

    let (_problem, result) = solve(
        "max z = 4x_1 + 8x_2\n\
         3x_1 + 2x_2 = 18\n\
         x_1 + x_2 <= 5\n\
         x_1 <= 4\n",
    );
    assert_eq!(result.status, SimplexStatus::Infeasible, "2_g");
}

#[test]
fn removes_degenerate_basic_artificial_from_redundant_restriction() {
    let (problem, result) = solve(
        "min z = x_1\n\
         x_1 = 1\n\
         2x_1 = 2\n",
    );

    assert_eq!(result.status, SimplexStatus::Optimal);
    assert_eq!(
        result
            .phase_one_objective
            .map(|value| value.abs() <= EPSILON),
        Some(true)
    );
    assert_eq!(result.form.a.rows(), 1);
    assert_no_artificial_columns(&result);
    assert_close(
        "restrição redundante x_1",
        value_of_variable(&result, 1),
        1.0,
    );
    assert_original_constraints_are_satisfied(&problem, &result);
}

#[test]
fn ratio_tie_terminates_with_expected_optimum() {
    let (problem, result) = solve(
        "max z = x_1 + x_2\n\
         x_1 <= 1\n\
         x_2 <= 1\n\
         x_1 + x_2 <= 1\n",
    );

    assert_eq!(result.status, SimplexStatus::Optimal);
    assert!(result.iterations < ITERATION_LIMIT);
    assert_close(
        "empate razão z",
        result.form.objective_value(&result.state).unwrap(),
        1.0,
    );
    assert_original_constraints_are_satisfied(&problem, &result);
}

#[test]
fn problem_without_constraints_is_handled_explicitly() {
    let (_problem, result) = solve("max z = x_1\n");

    assert_eq!(result.status, SimplexStatus::Unbounded);
    assert_eq!(result.iterations, 0);
}

#[test]
fn freezes_current_phase_two_optimal_cases() {
    let cases = [
        ExpectedOptimal {
            name: "1_a",
            text: "max z = x_1 + x_2\n\
                   2x_1 + 1x_2 <= 18\n\
                   -x_1 + 2x_2 <= 4\n\
                   3x_1 -6x_2 <= 12\n",
            z: 11.6,
            original_variables: &[(1, 6.4), (2, 5.2)],
        },
        ExpectedOptimal {
            name: "1_b",
            text: "max z = 6x_1 + 2x_2\n\
                   3x_1 + x_2 <= 33\n\
                   x_1 + x_2 <= 13\n",
            z: 66.0,
            original_variables: &[(1, 11.0), (2, 0.0)],
        },
        ExpectedOptimal {
            name: "1_c",
            text: "max z = x_1 + x_2\n\
                   2x_1 + x_2 <= 8\n\
                   x_1 + 2x_2 <= 3\n",
            z: 3.0,
            original_variables: &[(1, 3.0), (2, 0.0)],
        },
        ExpectedOptimal {
            name: "1_d",
            text: "max z = 3x_1 + x_2\n\
                   2x_1 + x_2 <= 30\n\
                   x_1 + 4x_2 <= 40\n",
            z: 45.0,
            original_variables: &[(1, 15.0), (2, 0.0)],
        },
        ExpectedOptimal {
            name: "1_e",
            text: "max z = 2x_1 + 5x_2\n\
                   3x_1 + 10x_2 <= 600\n\
                   x_1 + 2x_2 <= 162\n",
            z: 352.5,
            original_variables: &[(1, 105.0), (2, 28.5)],
        },
        ExpectedOptimal {
            name: "1_f",
            text: "min z = -1x_1 + 2x_2\n\
                   -2x_1 + x_2 <= 3\n\
                   3x_1 + 4x_2 <= 5\n\
                   x_1 -1x_2 <= 2\n",
            z: -1.6666666666666667,
            original_variables: &[(1, 1.6666666666666667), (2, 0.0)],
        },
        ExpectedOptimal {
            name: "1_i",
            text: "max z = x_1 + x_2\n\
                   2x_1 + x_2 <= 18\n\
                   -x_1 + 2x_2 <= 4\n\
                   3x_1 - 6x_2 >= -12\n",
            z: 11.6,
            original_variables: &[(1, 6.4), (2, 5.2)],
        },
        ExpectedOptimal {
            name: "2_a",
            text: "max z = 3x_1 + x_2\n\
                   2x_1 + x_2 <= 30\n\
                   x_1 + 4x_2 <= 40\n",
            z: 45.0,
            original_variables: &[(1, 15.0), (2, 0.0)],
        },
        ExpectedOptimal {
            name: "2_b",
            text: "max z = 2x_1 + 5x_2\n\
                   3x_1 + 10x_2 <= 600\n\
                   x_1 + 2x_2 <= 162\n",
            z: 352.5,
            original_variables: &[(1, 105.0), (2, 28.5)],
        },
        ExpectedOptimal {
            name: "2_h",
            text: "min z = -2.5x_1 -5.7x_2 -1x_3 -2x_4\n\
                   x_2 + x_3 <= 18\n\
                   -1.5x_1 + 2.5x_2 <= 4\n\
                   3x_1 -6x_2 + x_4 <= 12.7\n",
            z: -248.33333333333337,
            original_variables: &[(1, 27.333333333333336), (2, 18.0), (3, 0.0), (4, 38.7)],
        },
        ExpectedOptimal {
            name: "2_j",
            text: "max z = 4x_1 + 5x_2 + 9x_3 + 11x_4\n\
                   x_1 + x_2 + x_3 + x_4 <= 15\n\
                   7x_1 + 5x_2 + 3x_3 + 2x_4 <= 120\n\
                   3x_1 + 5x_2 + 10x_3 + 15x_4 <= 100\n",
            z: 99.28571428571429,
            original_variables: &[
                (1, 7.142857142857142),
                (2, 0.0),
                (3, 7.857142857142858),
                (4, 0.0),
            ],
        },
        ExpectedOptimal {
            name: "2_k",
            text: "min z = -5x_1 - 3x_2\n\
                   3x_1 + 5x_2 <= 15\n\
                   5x_1 + 2x_2 <= 10\n",
            z: -12.368421052631579,
            original_variables: &[(1, 1.0526315789473681), (2, 2.368421052631579)],
        },
    ];

    for case in cases {
        assert_optimal_case(case);
    }
}

#[test]
fn accepts_2_d_from_data_file_with_unicode_minus() {
    let text = problem_text_from_data_file("2_d");

    assert_optimal_case(ExpectedOptimal {
        name: "2_d",
        text: &text,
        z: 11.6,
        original_variables: &[(1, 6.4), (2, 5.2)],
    });
}

#[test]
fn freezes_optimal_solution_found_at_initial_solution() {
    assert_optimal_case(ExpectedOptimal {
        name: "initial_solution_is_optimal",
        text: "min z = x_1 + x_2\n\
               x_1 <= 2\n\
               x_2 <= 3\n",
        z: 0.0,
        original_variables: &[(1, 0.0), (2, 0.0)],
    });
}

#[test]
fn freezes_unbounded_problem_without_phase_one() {
    let (_problem, result) = solve(
        "max z = x_1\n\
         -x_1 <= 1\n",
    );

    assert_eq!(result.status, SimplexStatus::Unbounded);
    assert!(result.iterations < ITERATION_LIMIT);
}

#[test]
fn phase_one_solves_problem_with_artificial_variable_and_removes_it() {
    let (problem, result) = solve(
        "max z = 3x_1 + 3x_2 + 13x_3\n\
         -3x_1 + 6x_2 + 7x_3 <= 8\n\
         6x_1 - 3x_2 + 7x_3 <= 8\n\
         x_1 <= 2\n\
         x_3 >= 1\n",
    );

    assert_eq!(result.status, SimplexStatus::Optimal);
    assert!(result.phase_one_iterations < ITERATION_LIMIT);
    assert!(result.phase_two_iterations < ITERATION_LIMIT);
    assert_close(
        "2_i z",
        result.form.objective_value(&result.state).unwrap(),
        15.0,
    );
    assert_close("2_i x_1", value_of_variable(&result, 1), 1.0 / 3.0);
    assert_close("2_i x_2", value_of_variable(&result, 2), 1.0 / 3.0);
    assert_close("2_i x_3", value_of_variable(&result, 3), 1.0);
    assert!(
        result
            .form
            .variable_kinds
            .iter()
            .all(|kind| *kind != VariableKind::Artificial)
    );
    assert_original_constraints_are_satisfied(&problem, &result);
}

#[test]
fn phase_one_reports_infeasible_original_problem() {
    let (_problem, result) = solve(
        "max z = 4x_1 + 3x_2\n\
         x_1 + 3x_2 <= 7\n\
         2x_1 + 2x_2 = 8\n\
         x_1 + x_2 <= -3\n\
         x_2 <= 2\n",
    );

    assert_eq!(result.status, SimplexStatus::Infeasible);
}

#[test]
fn fixed_zero_variable_returns_clean_zero_in_solution() {
    let (problem, result) = solve(
        "max z = 3x_1 + 3x_2 + 13x_3\n\
         -3x_1 + 6x_2 + 7x_3 <= 8\n\
         6x_1 - 3x_2 + 7x_3 <= 8\n\
         x_1 <= 2\n\
         x_3 <= 0\n\
         x_1, x_2, x_3 >= 0\n",
    );

    assert_eq!(result.status, SimplexStatus::Optimal);
    assert_close(
        "2_l z",
        result.form.objective_value(&result.state).unwrap(),
        13.0,
    );
    assert_close("2_l x_1", value_of_variable(&result, 1), 2.0);
    assert_close("2_l x_2", value_of_variable(&result, 2), 7.0 / 3.0);
    assert_close("2_l x_3", value_of_variable(&result, 3), 0.0);
    assert_original_constraints_are_satisfied(&problem, &result);
}

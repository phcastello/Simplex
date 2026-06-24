use std::error::Error;
use std::fmt::{self, Display};

use crate::problem::{
    Constraint, EPSILON, Problem, Relation, Sense, Term, VariableBound, VariableKind,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NormalizeError {
    NonPositiveVariable { variable: usize },
}

impl Display for NormalizeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositiveVariable { variable } => write!(
                formatter,
                "a normalização atual ainda não suporta variável não positiva isolada: use x_{variable} = -x'_{variable}, com x'_{variable} >= 0"
            ),
        }
    }
}

impl Error for NormalizeError {}

fn invert_relation(relation: Relation) -> Relation {
    match relation {
        Relation::LessOrEqual => Relation::GreaterOrEqual,
        Relation::GreaterOrEqual => Relation::LessOrEqual,
        Relation::Equal => Relation::Equal,
    }
}

fn normalize_rhs(constraint: &Constraint) -> Constraint {
    let mut normalized = constraint.clone();

    if normalized.rhs < -EPSILON {
        normalized.rhs *= -1.0;
        normalized.relation = invert_relation(normalized.relation);
        for term in &mut normalized.terms {
            term.coefficient *= -1.0;
        }
    } else if normalized.rhs.abs() <= EPSILON {
        normalized.rhs = 0.0;
    }

    normalized
}

fn remove_fixed_zero_terms(terms: &mut Vec<Term>, problem: &Problem) {
    let mut kept_terms = Vec::new();

    for term in terms.iter() {
        if problem.variable_bounds.get(&term.variable) != Some(&VariableBound::FixedZero) {
            kept_terms.push(term.clone());
        }
    }

    *terms = kept_terms;
}

pub fn normalize(problem: &Problem) -> Result<Problem, NormalizeError> {
    for (variable, bound) in &problem.variable_bounds {
        if *bound == VariableBound::NonPositive {
            return Err(NormalizeError::NonPositiveVariable {
                variable: *variable,
            });
        }
    }

    let mut normalized = problem.clone();
    normalized.sense = Sense::Min;
    normalized.constraints.clear();

    if problem.sense == Sense::Max {
        for term in &mut normalized.objective {
            term.coefficient *= -1.0;
        }
    }
    remove_fixed_zero_terms(&mut normalized.objective, problem);

    let mut highest_variable = problem.variable_bounds.keys().copied().max().unwrap_or(0);
    for term in &problem.objective {
        highest_variable = highest_variable.max(term.variable);
    }
    for constraint in &problem.constraints {
        for term in &constraint.terms {
            highest_variable = highest_variable.max(term.variable);
        }
    }

    for constraint in &problem.constraints {
        let mut normalized_constraint = normalize_rhs(constraint);
        remove_fixed_zero_terms(&mut normalized_constraint.terms, problem);

        if normalized_constraint.relation != Relation::Equal {
            highest_variable += 1;
            let coefficient = match normalized_constraint.relation {
                Relation::LessOrEqual => 1.0,
                Relation::GreaterOrEqual => -1.0,
                Relation::Equal => unreachable!(),
            };
            let variable_kind = match normalized_constraint.relation {
                Relation::LessOrEqual => VariableKind::Slack,
                Relation::GreaterOrEqual => VariableKind::Excess,
                Relation::Equal => unreachable!(),
            };
            normalized_constraint.terms.push(Term {
                variable: highest_variable,
                coefficient,
            });
            normalized
                .variable_bounds
                .insert(highest_variable, VariableBound::NonNegative);
            normalized
                .variable_kinds
                .insert(highest_variable, variable_kind);
        }

        normalized_constraint.relation = Relation::Equal;
        normalized.constraints.push(normalized_constraint);
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use crate::problem::{Relation, Sense, Term, VariableBound, VariableKind};
    use crate::problem_parser::parse_problem;

    use super::{NormalizeError, normalize};

    fn coefficient(terms: &[Term], variable: usize) -> Option<f64> {
        terms
            .iter()
            .find(|term| term.variable == variable)
            .map(|term| term.coefficient)
    }

    #[test]
    fn converts_maximization_to_minimization() {
        let problem = parse_problem("max z = 5x_1 - 2x_2\n").unwrap();
        let normalized = normalize(&problem).unwrap();

        assert_eq!(normalized.sense, Sense::Min);
        assert_eq!(normalized.original_sense, Sense::Max);
        assert_eq!(coefficient(&normalized.objective, 1), Some(-5.0));
        assert_eq!(coefficient(&normalized.objective, 2), Some(2.0));
    }

    #[test]
    fn preserves_minimization_objective() {
        let problem = parse_problem("min z = 5x_1 - 2x_2\n").unwrap();
        let normalized = normalize(&problem).unwrap();

        assert_eq!(normalized.sense, Sense::Min);
        assert_eq!(normalized.original_sense, Sense::Min);
        assert_eq!(coefficient(&normalized.objective, 1), Some(5.0));
        assert_eq!(coefficient(&normalized.objective, 2), Some(-2.0));
    }

    #[test]
    fn normalizes_negative_rhs_before_adding_slack_or_excess() {
        let problem = parse_problem(
            "min z = x_1 + x_2\n\
             -2x_1 + x_2 <= -4\n\
             -2x_1 + x_2 >= -4\n\
             -x_1 = -3\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();
        assert!(
            normalized
                .constraints
                .iter()
                .all(|constraint| constraint.rhs >= 0.0)
        );

        let less_or_equal = &normalized.constraints[0];
        assert_eq!(less_or_equal.rhs, 4.0);
        assert_eq!(less_or_equal.relation, Relation::Equal);
        assert_eq!(coefficient(&less_or_equal.terms, 1), Some(2.0));
        assert_eq!(coefficient(&less_or_equal.terms, 2), Some(-1.0));
        assert_eq!(coefficient(&less_or_equal.terms, 3), Some(-1.0));

        let greater_or_equal = &normalized.constraints[1];
        assert_eq!(greater_or_equal.rhs, 4.0);
        assert_eq!(coefficient(&greater_or_equal.terms, 1), Some(2.0));
        assert_eq!(coefficient(&greater_or_equal.terms, 2), Some(-1.0));
        assert_eq!(coefficient(&greater_or_equal.terms, 4), Some(1.0));

        let equality = &normalized.constraints[2];
        assert_eq!(equality.rhs, 3.0);
        assert_eq!(
            equality.terms,
            vec![Term {
                variable: 1,
                coefficient: 1.0
            }]
        );
    }

    #[test]
    fn adds_slack_and_excess_variables_after_original_variables() {
        let problem = parse_problem(
            "min z = x_1 + x_2\n\
             x_1 + x_2 <= 4\n\
             x_1 + x_2 >= 4\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();

        assert_eq!(coefficient(&normalized.constraints[0].terms, 3), Some(1.0));
        assert_eq!(coefficient(&normalized.constraints[1].terms, 4), Some(-1.0));
        assert_eq!(
            normalized.variable_bounds.get(&3),
            Some(&VariableBound::NonNegative)
        );
        assert_eq!(
            normalized.variable_bounds.get(&4),
            Some(&VariableBound::NonNegative)
        );
        assert_eq!(
            normalized.variable_kinds.get(&3),
            Some(&VariableKind::Slack)
        );
        assert_eq!(
            normalized.variable_kinds.get(&4),
            Some(&VariableKind::Excess)
        );
        assert!(
            normalized
                .constraints
                .iter()
                .all(|constraint| constraint.relation == Relation::Equal)
        );
    }

    #[test]
    fn rejects_non_positive_variables() {
        let problem = parse_problem("min z = x_1\nx_1 <= 0\n").unwrap();

        assert_eq!(
            normalize(&problem),
            Err(NormalizeError::NonPositiveVariable { variable: 1 })
        );
    }

    #[test]
    fn removes_fixed_zero_variables_from_objective_and_constraints() {
        let problem = parse_problem(
            "max z = 3x_1 + 13x_3\n\
             -3x_1 + 7x_3 <= 8\n\
             x_1 <= 2\n\
             x_3 <= 0\n\
             x_1, x_3 >= 0\n",
        )
        .unwrap();
        let normalized = normalize(&problem).unwrap();

        assert_eq!(
            normalized.variable_bounds.get(&3),
            Some(&VariableBound::FixedZero)
        );
        assert_eq!(coefficient(&normalized.objective, 3), None);
        assert_eq!(coefficient(&normalized.constraints[0].terms, 3), None);
        assert_eq!(
            normalized.variable_kinds.get(&3),
            Some(&VariableKind::Original)
        );
    }

    #[test]
    fn keeps_cancelled_terms_out_of_normalized_constraints() {
        let problem = parse_problem("min z = x_1 + x_2\nx_1 - x_1 + x_2 <= 4\n").unwrap();
        let normalized = normalize(&problem).unwrap();

        assert_eq!(coefficient(&normalized.constraints[0].terms, 1), None);
        assert_eq!(coefficient(&normalized.constraints[0].terms, 2), Some(1.0));
        assert_eq!(coefficient(&normalized.constraints[0].terms, 3), Some(1.0));
    }
}

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display};

use regex::Regex;

use crate::problem::{
    Constraint, EPSILON, Problem, Relation, Sense, Term, VariableBound, VariableKind,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: Option<usize>,
    pub message: String,
}

impl ParseError {
    fn at(line: usize, message: impl Into<String>) -> Self {
        Self {
            line: Some(line),
            message: message.into(),
        }
    }

    fn general(message: impl Into<String>) -> Self {
        Self {
            line: None,
            message: message.into(),
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(formatter, "erro na linha {line}: {}", self.message),
            None => formatter.write_str(&self.message),
        }
    }
}

impl Error for ParseError {}

fn relation_from_symbol(symbol: &str) -> Relation {
    match symbol {
        "<=" => Relation::LessOrEqual,
        ">=" => Relation::GreaterOrEqual,
        "=" => Relation::Equal,
        _ => unreachable!(),
    }
}

fn bound_from_relation(relation: Relation) -> VariableBound {
    match relation {
        Relation::GreaterOrEqual => VariableBound::NonNegative,
        Relation::LessOrEqual => VariableBound::NonPositive,
        Relation::Equal => unreachable!(),
    }
}

fn combine_bounds(current: VariableBound, new_bound: VariableBound) -> VariableBound {
    if current == new_bound {
        return current;
    }

    VariableBound::FixedZero
}

fn normalize_parser_input(input: &str) -> String {
    // Parte de identificação e substituição dos caracteres que o parser entende.
    // Depois desta etapa, os regex precisam lidar apenas com sinais ASCII simples.
    input
        .replace(['−', '–', '—'], "-")
        .replace('≤', "<=")
        .replace('≥', ">=")
}

fn consolidate_terms(terms: Vec<Term>) -> Vec<Term> {
    let mut coefficients = BTreeMap::new();

    for term in terms {
        let coefficient = coefficients.entry(term.variable).or_insert(0.0);
        *coefficient += term.coefficient;
    }

    let mut consolidated = Vec::new();
    for (variable, coefficient) in coefficients {
        if coefficient.abs() > EPSILON {
            consolidated.push(Term {
                variable,
                coefficient,
            });
        }
    }
    consolidated
}

fn parse_variable_index(index: &str, line: usize) -> Result<usize, ParseError> {
    let variable = index
        .parse::<usize>()
        .map_err(|_| ParseError::at(line, format!("índice de variável inválido: \"{index}\"")))?;

    if variable == 0 {
        return Err(ParseError::at(
            line,
            "índice de variável inválido: x_0; os índices devem começar em 1",
        ));
    }

    Ok(variable)
}

fn parse_expression(expression: &str, line: usize) -> Result<Vec<Term>, ParseError> {
    let term_regex = Regex::new(r"([+-]?)\s*((?:\d+(?:\.\d*)?|\.\d+)?)\s*x_(\d+)").unwrap();
    // A expressão do regex é:
    // um sinal opcional, seguido de um coeficiente numérico opcional, seguido de "x_" e um índice de variável.
    let mut terms = Vec::new();
    let mut end = 0;

    for captures in term_regex.captures_iter(expression) {
        let whole = captures.get(0).unwrap();
        if !expression[end..whole.start()].trim().is_empty() {
            return Err(ParseError::at(
                line,
                format!("expressão linear inválida: \"{expression}\""),
            ));
        }
        if !terms.is_empty() && captures[1].is_empty() {
            return Err(ParseError::at(
                line,
                format!("termos devem ser separados por '+' ou '-': \"{expression}\""),
            ));
        }

        let sign = if &captures[1] == "-" { -1.0 } else { 1.0 };
        let coefficient = if captures[2].is_empty() {
            1.0
        } else {
            captures[2].parse::<f64>().map_err(|_| {
                ParseError::at(line, format!("coeficiente inválido: \"{}\"", &captures[2]))
            })?
        };
        let variable = parse_variable_index(&captures[3], line)?;
        terms.push(Term {
            variable,
            coefficient: sign * coefficient,
        });
        end = whole.end();
    }

    if terms.is_empty() || !expression[end..].trim().is_empty() {
        return Err(ParseError::at(
            line,
            format!("expressão linear inválida: \"{expression}\""),
        ));
    }
    Ok(consolidate_terms(terms))
}

fn parse_relation_line(text: &str, line: usize) -> Result<(&str, &str, f64), ParseError> {
    let relation_regex =
        Regex::new(r"^(.*?)\s*(<=|>=|=|<|>)\s*(-?(?:\d+(?:\.\d*)?|\.\d+))\s*$").unwrap();
    let captures = relation_regex
        .captures(text)
        .ok_or_else(|| ParseError::at(line, format!("relação inválida: \"{text}\"")))?;
    let lhs = captures.get(1).unwrap().as_str().trim();
    let symbol = captures.get(2).unwrap().as_str();
    if symbol == "<" || symbol == ">" {
        return Err(ParseError::at(
            line,
            "desigualdades estritas não são suportadas; use <= ou >=",
        ));
    }
    let rhs = captures[3].parse::<f64>().map_err(|_| {
        ParseError::at(line, format!("lado direito inválido: \"{}\"", &captures[3]))
    })?;
    Ok((lhs, symbol, rhs))
}

pub fn parse_problem(input: &str) -> Result<Problem, ParseError> {
    let normalized_input = normalize_parser_input(input);

    // Pega o objetivo e o sentido da otimização da primeira.
    // expressão: começa com "max" ou "min" (case-insensitive), seguido de "z =",
    // seguido de uma expressão linear, e nada mais depois.
    let objective_regex = Regex::new(r"(?i)^(max|min)\s+z\s*=\s*(.+?)\s*$").unwrap();

    // Pega os limites do lado esquerdo.
    // expressão: uma ou mais variáveis no formato "x_i", separadas por vírgulas e espaços opcionais, e nada mais depois.
    let bound_lhs_regex = Regex::new(r"^x_\d+(?:\s*,\s*x_\d+)*$").unwrap();

    //Pega os índices das variáveis em uma expressão de limite.
    // expressão: captura o índice de uma variável no formato "x_i".
    let bound_variable_regex = Regex::new(r"x_(\d+)").unwrap();

    //Se o problema é max ou min
    let mut sense = None;
    let mut objective = Vec::new();
    let mut constraints = Vec::new();
    let mut variable_bounds = BTreeMap::new();
    let mut warnings = Vec::new();
    let mut in_bounds = false;

    for (index, raw_line) in normalized_input.lines().enumerate() {
        let line_number = index + 1;
        let text = raw_line.trim();
        if text.is_empty() {
            continue;
        }

        if sense.is_none() {
            let captures = objective_regex.captures(text).ok_or_else(|| {
                ParseError::at(
                    line_number,
                    "a primeira linha deve ser um objetivo no formato 'max|min z = expressão'",
                )
            })?;
            sense = Some(if captures[1].eq_ignore_ascii_case("max") {
                Sense::Max
            } else {
                Sense::Min
            });
            objective = parse_expression(&captures[2], line_number)?;
            continue;
        }

        let (lhs, symbol, rhs) = parse_relation_line(text, line_number)?;
        let relation = relation_from_symbol(symbol);
        let looks_like_bound = lhs.contains(',')
            || (relation != Relation::Equal
                && rhs.abs() <= EPSILON
                && bound_lhs_regex.is_match(lhs));

        if looks_like_bound {
            in_bounds = true;
            if relation == Relation::Equal {
                return Err(ParseError::at(line_number, "um limite não pode usar '='"));
            }
            if rhs.abs() > EPSILON {
                return Err(ParseError::at(
                    line_number,
                    "o lado direito de um limite deve ser 0",
                ));
            }
            if !bound_lhs_regex.is_match(lhs) {
                return Err(ParseError::at(
                    line_number,
                    "lista de variáveis de limite inválida",
                ));
            }
            for captures in bound_variable_regex.captures_iter(lhs) {
                let variable = parse_variable_index(&captures[1], line_number)?;
                let new_bound = bound_from_relation(relation);

                match variable_bounds.get(&variable).copied() {
                    Some(current_bound) => {
                        let combined_bound = combine_bounds(current_bound, new_bound);
                        variable_bounds.insert(variable, combined_bound);

                        if current_bound == new_bound {
                            warnings.push(format!(
                                "Limite repetido para x_{variable}; mantendo apenas uma ocorrência."
                            ));
                        }
                    }
                    None => {
                        variable_bounds.insert(variable, new_bound);
                    }
                }
            }
        } else {
            if in_bounds {
                return Err(ParseError::at(
                    line_number,
                    "uma restrição não pode aparecer depois da seção de limites",
                ));
            }
            constraints.push(Constraint {
                terms: parse_expression(lhs, line_number)?,
                relation,
                rhs,
            });
        }
    }

    let sense =
        sense.ok_or_else(|| ParseError::general("entrada vazia: objetivo não encontrado"))?;
    let used_variables: BTreeSet<_> = objective
        .iter()
        .chain(constraints.iter().flat_map(|constraint| &constraint.terms))
        .map(|term| term.variable)
        .collect();

    if variable_bounds.is_empty() {
        for variable in &used_variables {
            variable_bounds.insert(*variable, VariableBound::NonNegative);
        }
        warnings.push(
            "Nenhum limite declarado; assumindo x_i >= 0 para todas as variáveis originais."
                .to_string(),
        );
    } else {
        for variable in &used_variables {
            if !variable_bounds.contains_key(variable) {
                return Err(ParseError::general(format!(
                    "limites parciais: x_{variable} é usada, mas não possui limite"
                )));
            }
        }
        for variable in variable_bounds.keys() {
            if !used_variables.contains(variable) {
                return Err(ParseError::general(format!(
                    "x_{variable} possui limite, mas não é usada no problema"
                )));
            }
        }
    }

    let mut variable_kinds = BTreeMap::new();
    for variable in used_variables {
        variable_kinds.insert(variable, VariableKind::Original);
    }

    Ok(Problem {
        sense,
        original_sense: sense,
        objective,
        constraints,
        variable_bounds,
        variable_kinds,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use crate::problem::{Relation, Sense, VariableBound};

    use super::parse_problem;

    #[test]
    fn parses_valid_problem_with_flexible_spaces_and_decimals() {
        let problem = parse_problem(
            "MIN z = -x_1 + .5x_2\n\
             2x_1+x_2 >= -3.5\n\
             x_1, x_2 >= 0\n",
        )
        .unwrap();
        assert_eq!(problem.sense, Sense::Min);
        assert_eq!(problem.objective[0].coefficient, -1.0);
        assert_eq!(problem.constraints[0].relation, Relation::GreaterOrEqual);
        assert_eq!(problem.constraints[0].rhs, -3.5);
    }

    #[test]
    fn reports_line_for_invalid_expression() {
        let error = parse_problem("max z = x_1\nx_1 + lixo <= 2\nx_1 <= 0\n").unwrap_err();
        assert_eq!(error.line, Some(2));
        assert!(parse_problem("max z = x_1 x_2\n").is_err());
    }

    #[test]
    fn normalizes_unicode_symbols_before_parsing() {
        let problem = parse_problem(
            "max z = x_1 + x_2\n\
             2x_1 + x_2 ≤ 18\n\
             -x_1 + 2x_2 <= 4\n\
             3x_1 −6x_2 ≥ -12\n",
        )
        .unwrap();

        assert_eq!(problem.constraints[0].relation, Relation::LessOrEqual);
        assert_eq!(problem.constraints[2].relation, Relation::GreaterOrEqual);
        assert_eq!(problem.constraints[2].terms[1].coefficient, -6.0);
    }

    #[test]
    fn combines_repeated_zero_bounds() {
        let repeated = parse_problem("max z = x_1\nx_1 <= 0\nx_1 <= 0\n").unwrap();
        assert_eq!(
            repeated.variable_bounds.get(&1),
            Some(&VariableBound::NonPositive)
        );
        assert_eq!(repeated.warnings.len(), 1);

        let fixed_zero = parse_problem("max z = x_1\nx_1 <= 0\nx_1 >= 0\n").unwrap();
        assert_eq!(
            fixed_zero.variable_bounds.get(&1),
            Some(&VariableBound::FixedZero)
        );

        assert!(parse_problem("max z = x_1 + x_2\nx_1 <= 0\n").is_err());
    }

    #[test]
    fn rejects_variable_zero_where_it_appears() {
        let objective_error = parse_problem("max z = x_0\n").unwrap_err();
        assert_eq!(objective_error.line, Some(1));
        assert!(objective_error.message.contains("x_0"));

        let constraint_error = parse_problem("max z = x_1\nx_0 <= 2\n").unwrap_err();
        assert_eq!(constraint_error.line, Some(2));
        assert!(constraint_error.message.contains("x_0"));

        let bound_error = parse_problem("max z = x_1\nx_0, x_1 >= 0\n").unwrap_err();
        assert_eq!(bound_error.line, Some(2));
        assert!(bound_error.message.contains("x_0"));
    }

    #[test]
    fn assumes_non_negative_bounds_when_none_are_declared() {
        let problem = parse_problem("max z = 3x_1 + 2x_2\nx_1 + x_2 <= 4\n").unwrap();

        assert_eq!(
            problem.variable_bounds.get(&1),
            Some(&VariableBound::NonNegative)
        );
        assert_eq!(
            problem.variable_bounds.get(&2),
            Some(&VariableBound::NonNegative)
        );
        assert_eq!(problem.warnings.len(), 1);
        assert!(problem.warnings[0].contains("x_i >= 0"));
    }

    #[test]
    fn rejects_strict_inequalities() {
        let less_error = parse_problem("max z = x_1\nx_1 < 5\n").unwrap_err();
        let greater_error = parse_problem("max z = x_1\nx_1 > 5\n").unwrap_err();

        assert!(less_error.message.contains("desigualdades estritas"));
        assert!(greater_error.message.contains("desigualdades estritas"));
    }

    #[test]
    fn consolidates_repeated_terms_and_removes_cancellations() {
        let problem = parse_problem(
            "max z = x_1 + 2x_1 - x_2\n\
             x_1 + 3x_1 <= 8\n\
             x_1 - x_1 + x_2 <= 4\n",
        )
        .unwrap();

        assert_eq!(problem.objective.len(), 2);
        assert_eq!(problem.objective[0].variable, 1);
        assert_eq!(problem.objective[0].coefficient, 3.0);
        assert_eq!(problem.constraints[0].terms.len(), 1);
        assert_eq!(problem.constraints[0].terms[0].coefficient, 4.0);
        assert_eq!(
            problem.constraints[1].terms,
            vec![crate::problem::Term {
                variable: 2,
                coefficient: 1.0,
            }]
        );
    }

    #[test]
    fn serializes_current_example() {
        let problem = parse_problem(
            "max z = 5x_1 + 4x_2\n\
             6x_1 + 4x_2 <= 24\n\
             x_1 + x_2 <= 1\n\
             x_2 <= 2\n\
             x_1, x_2 <= 0\n",
        )
        .unwrap();
        assert!(problem.to_string().contains("6x_1 + 4x_2 <= 24"));
        assert!(problem.to_string().ends_with("x_1, x_2 <= 0\n"));
    }
}

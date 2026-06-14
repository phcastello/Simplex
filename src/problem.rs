use std::collections::BTreeMap;
use std::fmt::{self, Display, Write};

pub const EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sense {
    Max,
    Min,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    LessOrEqual,
    GreaterOrEqual,
    Equal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariableKind {
    Original,
    Slack,
    Excess,
}

impl Relation {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::LessOrEqual => "<=",
            Self::GreaterOrEqual => ">=",
            Self::Equal => "=",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Term {
    pub variable: usize,
    pub coefficient: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Constraint {
    pub terms: Vec<Term>,
    pub relation: Relation,
    pub rhs: f64, //Lado direito da restrição
}

#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub sense: Sense,
    pub original_sense: Sense,
    pub objective: Vec<Term>,
    pub constraints: Vec<Constraint>,
    pub variable_bounds: BTreeMap<usize, Relation>,
    pub variable_kinds: BTreeMap<usize, VariableKind>,
    pub warnings: Vec<String>,
}

fn format_number(value: f64) -> String {
    let mut result = format!("{value:.10}");
    while result.contains('.') && result.ends_with('0') {
        result.pop();
    }
    if result.ends_with('.') {
        result.pop();
    }
    result
}

fn write_terms(output: &mut String, terms: &[Term]) -> fmt::Result {
    if terms.is_empty() {
        return output.write_char('0');
    }

    for (index, term) in terms.iter().enumerate() {
        let positive = term.coefficient >= 0.0;
        let absolute = term.coefficient.abs();

        if index == 0 {
            if !positive {
                output.write_char('-')?;
            }
        } else {
            output.write_str(if positive { " + " } else { " - " })?;
        }

        if absolute != 1.0 {
            output.write_str(&format_number(absolute))?;
        }
        write!(output, "x_{}", term.variable)?;
    }
    Ok(())
}

impl Display for Problem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = String::new();
        output.write_str(match self.sense {
            Sense::Max => "max z = ",
            Sense::Min => "min z = ",
        })?;
        write_terms(&mut output, &self.objective)?;
        output.write_char('\n')?;

        for constraint in &self.constraints {
            output.write_str("    ")?;
            write_terms(&mut output, &constraint.terms)?;
            writeln!(
                output,
                " {} {}",
                constraint.relation.symbol(),
                format_number(constraint.rhs)
            )?;
        }

        for relation in [Relation::LessOrEqual, Relation::GreaterOrEqual] {
            let variables: Vec<_> = self
                .variable_bounds
                .iter()
                .filter_map(|(variable, bound)| (*bound == relation).then_some(*variable))
                .collect();
            if !variables.is_empty() {
                output.write_str("    ")?;
                for (index, variable) in variables.iter().enumerate() {
                    if index > 0 {
                        output.write_str(", ")?;
                    }
                    write!(output, "x_{variable}")?;
                }
                writeln!(output, " {} 0", relation.symbol())?;
            }
        }

        formatter.write_str(&output)
    }
}

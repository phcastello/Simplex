use std::io::{self, Write};

use simplex::branch_and_bound::{
    BranchAndBoundResult, BranchAndBoundStatus, BranchConstraint, NodeFinalAction,
    solve_all_integer_variables,
};
use simplex::matrix::Matrix;
use simplex::matrix_form::{MatrixForm, SimplexResult, SimplexStatus};
use simplex::normalizer::normalize;
use simplex::problem::{EPSILON, Problem, Relation, VariableKind};
use simplex::problem_io::read_problem;

pub const READ_PATH: &str = "data/read.txt";

fn print_menu() {
    println!("=================================");
    println!("Escolha uma opção abaixo:");
    println!("0 - Sair");
    println!("1 - Resolver simplex");
    println!("2 - Exibir análise do simplex");
    println!("3 - Resolver Branch and Bound");
    println!("4 - Exibir análise do Branch and Bound");
    print!("Opção: ");
    io::stdout().flush().unwrap();
}

fn solve_simplex() {
    let problem = match read_problem(READ_PATH) {
        Ok(problem) => problem,
        Err(error) => {
            eprintln!("Erro: {error}");
            return;
        }
    };

    for warning in &problem.warnings {
        eprintln!("Aviso: {warning}");
    }

    let normalized = match normalize(&problem) {
        Ok(normalized) => normalized,
        Err(error) => {
            eprintln!("Erro ao normalizar o problema: {error}");
            return;
        }
    };

    let form = match MatrixForm::from_problem(&normalized) {
        Ok(form) => form,
        Err(error) => {
            eprintln!("Erro ao converter o problema em matriz: {error}");
            return;
        }
    };
    let result = match form.solve_simplex() {
        Ok(result) => result,
        Err(error) => {
            eprintln!("Erro durante o simplex: {error}");
            return;
        }
    };

    println!("\nResultado do simplex");
    println!("Iterações realizadas: {}", result.iterations);
    println!("Iterações na Fase I: {}", result.phase_one_iterations);
    println!("Iterações na Fase II: {}", result.phase_two_iterations);
    if let Some(value) = result.phase_one_objective {
        println!("Valor final da Fase I (w): {}", format_number(value));
    }

    match result.status {
        SimplexStatus::Optimal => {
            let mut basic_variables = Vec::new();
            for column in &result.state.basic_columns {
                if result.form.variable_kinds[*column] == VariableKind::Artificial {
                    continue;
                }
                let variable = result.form.variables[*column];
                basic_variables.push(format!("x_{variable}"));
            }

            let mut non_basic_variables = Vec::new();
            for column in &result.state.non_basic_columns {
                if result.form.variable_kinds[*column] == VariableKind::Artificial {
                    continue;
                }
                let variable = result.form.variables[*column];
                non_basic_variables.push(format!("x_{variable}"));
            }

            println!("Solução ótima encontrada.");
            println!("Variáveis básicas: {}", basic_variables.join(", "));
            println!("Variáveis não básicas: {}", non_basic_variables.join(", "));

            match result.state.solution(&result.form) {
                Ok(solution) => {
                    println!("\nx (solução final)");
                    for row in 0..result.form.variables.len() {
                        if result.form.variable_kinds[row] == VariableKind::Artificial {
                            continue;
                        }
                        let variable = result.form.variables[row];
                        println!("x_{variable} = {}", format_number(solution.get(row, 0)));
                    }
                    for variable in &result.form.fixed_zero_variables {
                        println!("x_{variable} = 0");
                    }
                }
                Err(error) => eprintln!("Não foi possível calcular a solução final: {error}"),
            }

            match result.form.objective_value(&result.state) {
                Ok(value) => println!("\nz = {}", format_number(value)),
                Err(error) => eprintln!("Não foi possível calcular o valor objetivo: {error}"),
            }
        }
        SimplexStatus::Unbounded => println!("O problema é ilimitado."),
        SimplexStatus::Infeasible => println!("O problema é inviável."),
        SimplexStatus::IterationLimit => {
            println!("O limite de iterações foi atingido antes da conclusão.")
        }
    }

    println!();
}

fn format_number(value: f64) -> String {
    let value = if value.abs() < EPSILON { 0.0 } else { value };
    let mut result = format!("{value:.4}");
    while result.contains('.') && result.ends_with('0') {
        result.pop();
    }
    if result.ends_with('.') {
        result.pop();
    }
    result
}

fn print_matrix(name: &str, matrix: &Matrix, row_labels: &[String], column_labels: &[String]) {
    let values: Vec<Vec<String>> = (0..matrix.rows())
        .map(|row| {
            (0..matrix.columns())
                .map(|column| format_number(matrix.get(row, column)))
                .collect()
        })
        .collect();

    let label_width = row_labels.iter().map(String::len).max().unwrap_or(0);
    let column_widths: Vec<usize> = (0..matrix.columns())
        .map(|column| {
            let value_width = values
                .iter()
                .map(|row| row[column].len())
                .max()
                .unwrap_or(0);
            value_width.max(column_labels[column].len())
        })
        .collect();
    let matrix_width = column_widths.iter().sum::<usize>() + 2 * matrix.columns();

    println!("\n{name} ({} x {})", matrix.rows(), matrix.columns());
    print!("{:label_width$}  ", "");
    for (label, width) in column_labels.iter().zip(&column_widths) {
        print!(" {:>width$} ", label);
    }
    println!();
    println!("{:label_width$} +{}+", "", "-".repeat(matrix_width));

    for (label, row) in row_labels.iter().zip(values) {
        print!("{label:>label_width$} |");
        for (value, width) in row.iter().zip(&column_widths) {
            print!(" {value:>width$} ");
        }
        println!("|");
    }

    println!("{:label_width$} +{}+", "", "-".repeat(matrix_width));
}

fn value_of_variable(result: &SimplexResult, variable: usize) -> Option<f64> {
    if result.form.fixed_zero_variables.contains(&variable) {
        return Some(0.0);
    }

    let solution = result.state.solution(&result.form).ok()?;

    for column in 0..result.form.variables.len() {
        if result.form.variables[column] == variable {
            return Some(solution.get(column, 0));
        }
    }

    None
}

fn solution_satisfies_original_constraints(problem: &Problem, result: &SimplexResult) -> bool {
    for constraint in &problem.constraints {
        let mut value = 0.0;

        for term in &constraint.terms {
            let variable_value = match value_of_variable(result, term.variable) {
                Some(value) => value,
                None => return false,
            };
            value += term.coefficient * variable_value;
        }

        let satisfied = match constraint.relation {
            Relation::LessOrEqual => value <= constraint.rhs + EPSILON,
            Relation::GreaterOrEqual => value >= constraint.rhs - EPSILON,
            Relation::Equal => (value - constraint.rhs).abs() <= EPSILON,
        };

        if !satisfied {
            return false;
        }
    }

    true
}

fn simplex_status_label(status: SimplexStatus) -> &'static str {
    match status {
        SimplexStatus::Optimal => "ótimo",
        SimplexStatus::Unbounded => "ilimitado",
        SimplexStatus::Infeasible => "inviável",
        SimplexStatus::IterationLimit => "limite de iterações atingido",
    }
}

fn branch_and_bound_status_label(status: BranchAndBoundStatus) -> &'static str {
    match status {
        BranchAndBoundStatus::Optimal => "ótimo",
        BranchAndBoundStatus::Infeasible => "inteiro inviável",
        BranchAndBoundStatus::NodeLimit => "limite de nós atingido",
        BranchAndBoundStatus::SimplexIterationLimit => "limite de iterações do simplex atingido",
        BranchAndBoundStatus::RelaxationUnbounded => "relaxação linear ilimitada",
    }
}

fn node_action_label(action: NodeFinalAction) -> &'static str {
    match action {
        NodeFinalAction::Branched => "ramificado",
        NodeFinalAction::PrunedInfeasible => "podado por inviabilidade",
        NodeFinalAction::PrunedByBound => "podado por limitante",
        NodeFinalAction::IntegerSolution => "solução inteira",
        NodeFinalAction::IterationLimit => "limite de iterações",
        NodeFinalAction::RelaxationUnbounded => "relaxação ilimitada",
    }
}

fn branch_constraint_label(constraint: BranchConstraint) -> String {
    format!(
        "x_{} {} {}",
        constraint.variable,
        constraint.relation.symbol(),
        format_number(constraint.rhs)
    )
}

fn print_branch_and_bound_summary(result: &BranchAndBoundResult) {
    println!("\nResultado do Branch and Bound");
    println!("Status: {}", branch_and_bound_status_label(result.status));

    match &result.solution {
        Some(solution) => {
            if result.status == BranchAndBoundStatus::NodeLimit {
                println!("Incumbente encontrada, mas a otimalidade não foi provada.");
            }
            println!("z = {}", format_number(solution.objective_value));
            println!("Valores das variáveis originais:");
            for (variable, value) in &solution.variable_values {
                println!("x_{variable} = {}", format_number(*value));
            }
        }
        None => {
            println!("Nenhuma solução inteira foi encontrada.");
        }
    }

    if result.status == BranchAndBoundStatus::RelaxationUnbounded {
        println!(
            "Uma relaxação linear foi ilimitada; esta implementação simples não conclui o problema inteiro nesse caso."
        );
    }

    println!("Nós explorados: {}", result.explored_nodes);
    println!("Nós criados: {}", result.created_nodes);
    println!("Podas por inviabilidade: {}", result.pruned_infeasible);
    println!("Podas por limitante: {}", result.pruned_by_bound);
    println!("Nós com solução inteira: {}", result.integer_solution_nodes);
    println!(
        "Total de iterações do Simplex: {}",
        result.total_simplex_iterations
    );
    println!();
}

fn solve_branch_and_bound_from_file() {
    let problem = match read_problem(READ_PATH) {
        Ok(problem) => problem,
        Err(error) => {
            eprintln!("Erro: {error}");
            return;
        }
    };

    for warning in &problem.warnings {
        eprintln!("Aviso: {warning}");
    }

    println!("Branch and Bound: considerando todas as variáveis originais como inteiras.");

    match solve_all_integer_variables(&problem, 10_000) {
        Ok(result) => print_branch_and_bound_summary(&result),
        Err(error) => eprintln!("Erro durante o Branch and Bound: {error}"),
    }
}

fn print_branch_and_bound_analysis() {
    let problem = match read_problem(READ_PATH) {
        Ok(problem) => problem,
        Err(error) => {
            eprintln!("Erro: {error}");
            return;
        }
    };

    for warning in &problem.warnings {
        eprintln!("Aviso: {warning}");
    }

    println!("\nAnálise do Branch and Bound");
    println!("Todas as variáveis originais foram consideradas inteiras.");

    let result = match solve_all_integer_variables(&problem, 10_000) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("Erro durante o Branch and Bound: {error}");
            return;
        }
    };

    for report in &result.node_reports {
        let indent = "  ".repeat(report.depth);
        match report.branch_constraint {
            Some(constraint) => println!(
                "{}P{} [{}]",
                indent,
                report.id,
                branch_constraint_label(constraint)
            ),
            None => println!("{}P{}", indent, report.id),
        }

        println!(
            "{}  relaxação: {}",
            indent,
            simplex_status_label(report.relaxation_status)
        );
        if let Some(value) = report.relaxation_objective {
            println!("{}  z = {}", indent, format_number(value));
        }

        if !report.original_variable_values.is_empty() {
            for (variable, value) in &report.original_variable_values {
                println!("{}  x_{} = {}", indent, variable, format_number(*value));
            }
        }

        if let Some(variable) = report.branched_variable {
            let left = report.left_bound.unwrap_or(0.0);
            let right = report.right_bound.unwrap_or(0.0);
            println!(
                "{}  ramificação: x_{} <= {} ou x_{} >= {}",
                indent,
                variable,
                format_number(left),
                variable,
                format_number(right)
            );
        }

        println!(
            "{}  ação final: {}",
            indent,
            node_action_label(report.final_action)
        );
        println!();
    }

    print_branch_and_bound_summary(&result);
}

fn print_final_analysis(problem: &Problem, result: &SimplexResult) {
    println!("\nResumo final da execução");
    println!("Status final: {}", simplex_status_label(result.status));
    println!("Iterações realizadas: {}", result.iterations);
    println!("Iterações na Fase I: {}", result.phase_one_iterations);
    println!("Iterações na Fase II: {}", result.phase_two_iterations);
    if let Some(value) = result.phase_one_objective {
        println!("Valor final da Fase I (w): {}", format_number(value));
    }

    if result.status == SimplexStatus::Optimal {
        println!("Valores das variáveis originais:");
        for column in 0..result.form.variables.len() {
            if result.form.variable_kinds[column] == VariableKind::Original {
                let variable = result.form.variables[column];
                match value_of_variable(result, variable) {
                    Some(value) => println!("x_{variable} = {}", format_number(value)),
                    None => println!("x_{variable} = indisponível"),
                }
            }
        }
        for variable in &result.form.fixed_zero_variables {
            println!("x_{variable} = 0");
        }

        match result.form.objective_value(&result.state) {
            Ok(value) => println!("z = {}", format_number(value)),
            Err(error) => eprintln!("Não foi possível calcular o valor objetivo: {error}"),
        }

        if solution_satisfies_original_constraints(problem, result) {
            println!("A solução satisfaz as restrições originais dentro de EPSILON.");
        } else {
            println!("A solução não satisfaz todas as restrições originais dentro de EPSILON.");
        }
    }
}

fn print_simplex_analysis() {
    match read_problem(READ_PATH) {
        Ok(problem) => {
            for warning in &problem.warnings {
                eprintln!("Aviso: {warning}");
            }

            match normalize(&problem) {
                Ok(normalized) => match MatrixForm::from_problem(&normalized) {
                    Ok(form) => {
                        let natural_basis = form.natural_slack_basis();
                        let mut missing_rows = Vec::new();

                        println!("\nBase natural encontrada:");
                        for (row, basis_column) in natural_basis.iter().enumerate() {
                            match basis_column {
                                Some(column) => {
                                    let variable = form.variables[*column];
                                    println!("linha {} -> x_{}", row + 1, variable);
                                }
                                None => {
                                    println!("linha {} -> sem base", row + 1);
                                    missing_rows.push(row + 1);
                                }
                            }
                        }

                        if !missing_rows.is_empty() {
                            println!(
                                "Linhas que exigem artificiais: {}",
                                missing_rows
                                    .iter()
                                    .map(|row| row.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            );
                            println!(
                                "Objetivo artificial: min w = soma das variáveis artificiais."
                            );
                        }

                        let state = match form.prepare_current_phase_two_state() {
                            Ok(state) => state,
                            Err(error) => {
                                eprintln!("Erro ao preparar a base inicial: {error}");
                                match form.solve_simplex() {
                                    Ok(result) => {
                                        print_final_analysis(&problem, &result);
                                        if result.phase_one_objective.is_some() {
                                            let has_artificial = result
                                                .form
                                                .variable_kinds
                                                .contains(&VariableKind::Artificial);
                                            if has_artificial {
                                                println!(
                                                    "Artificiais removidas: não, problema encerrado antes da Fase II."
                                                );
                                            } else {
                                                println!(
                                                    "Artificiais removidas: sim; base final passada para a Fase II."
                                                );
                                            }
                                        }
                                    }
                                    Err(error) => {
                                        eprintln!("Não foi possível resolver o simplex: {error}");
                                    }
                                }
                                return;
                            }
                        };
                        let variables: Vec<String> = form
                            .variables
                            .iter()
                            .map(|variable| format!("x_{variable}"))
                            .collect();
                        let constraints: Vec<String> = (1..=form.a.rows())
                            .map(|constraint| format!("r_{constraint}"))
                            .collect();
                        let mut basic_variables = Vec::new();
                        for column in &state.basic_columns {
                            let variable = form.variables[*column];
                            basic_variables.push(format!("x_{variable}"));
                        }
                        let mut non_basic_variables = Vec::new();
                        for column in &state.non_basic_columns {
                            let variable = form.variables[*column];
                            non_basic_variables.push(format!("x_{variable}"));
                        }

                        println!("\nAnálise do simplex");
                        println!("Variáveis básicas: {}", basic_variables.join(", "));
                        println!("Variáveis não básicas: {}", non_basic_variables.join(", "));
                        println!("\nForma matricial: Ax = b, com objetivo c^T x");
                        print_matrix("A", &form.a, &constraints, &variables);
                        print_matrix(
                            "B (matriz básica)",
                            &state.basic_matrix(&form),
                            &constraints,
                            &basic_variables,
                        );
                        print_matrix("b", &form.b, &constraints, &[String::from("RHS")]);
                        print_matrix("c", &form.c, &variables, &[String::from("coef.")]);
                        let basic_costs = state.basic_costs();
                        print_matrix(
                            "c_B (custos básicos)",
                            &basic_costs,
                            &basic_variables,
                            &[String::from("custo")],
                        );
                        match state.basic_solution(&form) {
                            Ok(basic_solution) => {
                                print_matrix(
                                    "x_B (solução básica)",
                                    &basic_solution,
                                    &basic_variables,
                                    &[String::from("valor")],
                                );

                                match state.is_basic_solution_feasible(&form) {
                                    Ok(true) => println!(
                                        "\nA base é factível: todos os valores de x_B são não negativos."
                                    ),
                                    Ok(false) => println!(
                                        "\nA base não é factível: existe um valor negativo em x_B."
                                    ),
                                    Err(error) => {
                                        eprintln!("Não foi possível verificar a base: {error}");
                                    }
                                }
                            }
                            Err(error) => {
                                eprintln!("Não foi possível resolver B x_B = b: {error}");
                            }
                        }

                        match state.lambda(&form) {
                            Ok(lambda) => {
                                print_matrix(
                                    "lambda",
                                    &lambda,
                                    &constraints,
                                    &[String::from("valor")],
                                );
                            }
                            Err(error) => {
                                eprintln!("Não foi possível calcular lambda: {error}");
                            }
                        }

                        match state.reduced_costs(&form) {
                            Ok(reduced_costs) => {
                                println!("\nCustos reduzidos das variáveis não básicas:");
                                for reduced_cost in reduced_costs {
                                    let variable = reduced_cost.variable;
                                    let value = format_number(reduced_cost.value);

                                    if reduced_cost.improves_objective {
                                        println!("x_{variable}: {value} (melhora o objetivo)");
                                    } else {
                                        println!("x_{variable}: {value} (não melhora o objetivo)");
                                    }
                                }
                            }
                            Err(error) => {
                                eprintln!("Não foi possível calcular os custos reduzidos: {error}");
                            }
                        }

                        match state.direction(&form) {
                            Ok(Some(direction)) => {
                                let entering_variable = direction.entering_variable;
                                let reduced_cost = format_number(direction.reduced_cost);

                                println!(
                                    "\nVariável entrante escolhida: x_{entering_variable}, com custo reduzido {reduced_cost}."
                                );
                                print_matrix(
                                    "a_k (coluna da variável entrante)",
                                    &direction.entering_column,
                                    &constraints,
                                    &[format!("x_{entering_variable}")],
                                );
                                print_matrix(
                                    "y (direção), solução de B y = a_k",
                                    &direction.y,
                                    &basic_variables,
                                    &[String::from("valor")],
                                );
                                println!("\nRelação: x_Bnovo = x_B - y theta");

                                match state.ratio_test(&form, &direction) {
                                    Ok(ratio_test) => {
                                        if ratio_test.is_unbounded {
                                            println!(
                                                "\nO problema é ilimitado: nenhum valor de y é positivo."
                                            );
                                        } else {
                                            println!("\nRazões válidas x_Bi / y_i:");

                                            for ratio in &ratio_test.ratios {
                                                let basic_variable = ratio.basic_variable;
                                                let basic_value = format_number(ratio.basic_value);
                                                let direction_value =
                                                    format_number(ratio.direction_value);
                                                let value = format_number(ratio.value);

                                                println!(
                                                    "x_{basic_variable}: {basic_value} / {direction_value} = {value}"
                                                );
                                            }

                                            match state.leaving_variable(&ratio_test) {
                                                Some(leaving_variable) => {
                                                    let variable = leaving_variable.variable;
                                                    let theta =
                                                        format_number(leaving_variable.theta);
                                                    println!(
                                                        "\nVariável que sai: x_{variable}, com theta = {theta}."
                                                    );
                                                }
                                                None => {
                                                    println!(
                                                        "\nNão foi possível escolher uma variável para sair."
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    Err(error) => {
                                        eprintln!("Não foi possível calcular as razões: {error}");
                                    }
                                }
                            }
                            Ok(None) => {
                                println!(
                                    "\nNenhuma variável não básica melhora o objetivo; não há direção entrante."
                                );
                            }
                            Err(error) => {
                                eprintln!("Não foi possível calcular a direção: {error}");
                            }
                        }

                        match form.solve_simplex() {
                            Ok(result) => print_final_analysis(&problem, &result),
                            Err(error) => eprintln!("Não foi possível resolver o simplex: {error}"),
                        }

                        println!();
                    }
                    Err(error) => eprintln!("Erro ao converter o problema em matriz: {error}"),
                },
                Err(error) => eprintln!("Erro ao normalizar o problema: {error}"),
            }
        }
        Err(error) => eprintln!("Erro: {error}"),
    }
}

fn main() {
    loop {
        print_menu();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "0" => break,
            "1" => solve_simplex(),
            "2" => print_simplex_analysis(),
            "3" => solve_branch_and_bound_from_file(),
            "4" => print_branch_and_bound_analysis(),
            _ => println!("Opção inválida."),
        }
    }
}

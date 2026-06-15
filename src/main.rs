use std::io::{self, Write};

use simplex::matrix::Matrix;
use simplex::matrix_form::{MatrixForm, SimplexStatus};
use simplex::normalizer::normalize;
use simplex::problem_io::read_problem;

pub const READ_PATH: &str = "data/read.txt";

fn print_menu() {
    println!("=================================");
    println!("Escolha uma opção abaixo:");
    println!("0 - Sair");
    println!("1 - Resolver simplex");
    println!("2 - Exibir análise do simplex");
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

    match result.status {
        SimplexStatus::Optimal => {
            let mut basic_variables = Vec::new();
            for column in &result.form.basic_columns {
                let variable = result.form.variables[*column];
                basic_variables.push(format!("x_{variable}"));
            }

            let mut non_basic_variables = Vec::new();
            for column in &result.form.non_basic_columns {
                let variable = result.form.variables[*column];
                non_basic_variables.push(format!("x_{variable}"));
            }

            println!("Solução ótima encontrada.");
            println!("Variáveis básicas: {}", basic_variables.join(", "));
            println!("Variáveis não básicas: {}", non_basic_variables.join(", "));

            let mut variables = Vec::new();
            for variable in &result.form.variables {
                variables.push(format!("x_{variable}"));
            }

            match result.form.solution() {
                Ok(solution) => {
                    print_matrix(
                        "x (solução final)",
                        &solution,
                        &variables,
                        &[String::from("valor")],
                    );
                }
                Err(error) => eprintln!("Não foi possível calcular a solução final: {error}"),
            }

            match result.form.objective_value() {
                Ok(value) => println!("\nz = {}", format_number(value)),
                Err(error) => eprintln!("Não foi possível calcular o valor objetivo: {error}"),
            }
        }
        SimplexStatus::Unbounded => println!("O problema é ilimitado."),
        SimplexStatus::InfeasibleInitialBase => {
            println!("A base inicial não é factível. O simplex primal não pode começar.")
        }
        SimplexStatus::IterationLimit => {
            println!("O limite de iterações foi atingido antes da conclusão.")
        }
    }

    println!();
}

fn format_number(value: f64) -> String {
    let value = if value.abs() < 1e-9 { 0.0 } else { value };
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

fn print_simplex_analysis() {
    match read_problem(READ_PATH) {
        Ok(problem) => {
            for warning in &problem.warnings {
                eprintln!("Aviso: {warning}");
            }

            match normalize(&problem) {
                Ok(normalized) => match MatrixForm::from_problem(&normalized) {
                    Ok(form) => {
                        let variables: Vec<String> = form
                            .variables
                            .iter()
                            .map(|variable| format!("x_{variable}"))
                            .collect();
                        let constraints: Vec<String> = (1..=form.a.rows())
                            .map(|constraint| format!("r_{constraint}"))
                            .collect();
                        let mut basic_variables = Vec::new();
                        for column in &form.basic_columns {
                            let variable = form.variables[*column];
                            basic_variables.push(format!("x_{variable}"));
                        }
                        let mut non_basic_variables = Vec::new();
                        for column in &form.non_basic_columns {
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
                            &form.basic_matrix,
                            &constraints,
                            &basic_variables,
                        );
                        print_matrix("b", &form.b, &constraints, &[String::from("RHS")]);
                        print_matrix("c", &form.c, &variables, &[String::from("coef.")]);
                        let basic_costs = form.basic_costs();
                        print_matrix(
                            "c_B (custos básicos)",
                            &basic_costs,
                            &basic_variables,
                            &[String::from("custo")],
                        );
                        match form.basic_solution() {
                            Ok(basic_solution) => {
                                print_matrix(
                                    "x_B (solução básica)",
                                    &basic_solution,
                                    &basic_variables,
                                    &[String::from("valor")],
                                );

                                match form.is_basic_solution_feasible() {
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

                        match form.lambda() {
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

                        match form.reduced_costs() {
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

                        match form.direction() {
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

                                match form.ratio_test(&direction) {
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

                                            match form.leaving_variable(&ratio_test) {
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
            _ => println!("Opção inválida."),
        }
    }
}

use std::io::{self, Write};

use simplex::matrix::Matrix;
use simplex::matrix_form::MatrixForm;
use simplex::normalizer::normalize;
use simplex::problem_io::{read_problem, write_problem};

pub const READ_PATH: &str = "data/read.txt";
pub const WRITE_PATH: &str = "data/write.txt";

fn print_menu() {
    println!("=================================");
    println!("Escolha uma opção abaixo:");
    println!("0 - Sair");
    println!("1 - Normalizar problema");
    println!("2 - Resolver simplex");
    println!("3 - Calcular determinante");
    println!("4 - Calcular inversa");
    println!("5 - Aleatorizar matriz");
    println!("6 - Imprimir A, B, b e c");
    print!("Opção: ");
    io::stdout().flush().unwrap();
}

fn normalize_problem() {
    match read_problem(READ_PATH) {
        Ok(problem) => {
            for warning in &problem.warnings {
                eprintln!("Aviso: {warning}");
            }

            match normalize(&problem) {
                Ok(normalized) => {
                    print!("{normalized}");
                    match write_problem(WRITE_PATH, &normalized) {
                        Ok(()) => println!("Resultado gravado em {WRITE_PATH}"),
                        Err(error) => eprintln!("Erro: {error}"),
                    }
                }
                Err(error) => eprintln!("Erro ao normalizar o problema: {error}"),
            }
        }
        Err(error) => eprintln!("Erro: {error}"),
    }
}

fn format_number(value: f64) -> String {
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

fn print_matrix_form() {
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
                        for index in 0..form.variables.len() {
                            if form.variable_kinds[index]
                                != simplex::problem::VariableKind::Original
                            {
                                basic_variables.push(format!("x_{}", form.variables[index]));
                            }
                        }

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
            "1" => normalize_problem(),
            "2" => println!("O solver simplex ainda não foi implementado."),
            "3" => println!("O determinante depende da futura conversão do problema em matriz."),
            "4" => println!("A inversa depende da futura conversão do problema em matriz."),
            "5" => println!("O aleatorizador de matrizes ainda não foi implementado."),
            "6" => print_matrix_form(),
            _ => println!("Opção inválida."),
        }
    }
}

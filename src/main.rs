ub mod autocorrect;
pub mod compiler;
pub mod table;
pub mod closure;
use clap::{command, crate_version, value_parser, Arg, Command};
use compiler::*;
use eyre::{bail, Result};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

macro_rules! input_output {
    ($command_name:literal) => {
        Command::new($command_name)
            .about(concat!("Compiles code to ", $command_name))
            .arg(
                Arg::new("input")
                    .short('i')
                    .long("input")
                    .value_name("INPUT_FILE")
                    .required(true)
                    .value_parser(value_parser!(PathBuf))
                    .help("Input file to compile"),
            )
            .arg(
                Arg::new("output")
                    .short('o')
                    .long("output")
                    .value_name("OUTPUT_FILE")
                    .required(false)
                    .value_parser(value_parser!(PathBuf))
                    .default_value(concat!("output.", $command_name))
                    .help("Output file"),
            )
    };
}

macro_rules! exec_output {
    ($matches:ident, $output:expr) => {
        if let Some($matches) = $matches.subcommand_matches($output.to_string().as_str()) {
            let input: &PathBuf = $matches.get_one("input").unwrap();
            let output: &PathBuf = $matches.get_one("output").unwrap();
            let file = File::open(input)?;
            let reader = BufReader::new(file);
            let mut interpreter = VM::new(String::new());
            for (index, line) in reader.lines().enumerate() {
                let line = line?;
                interpreter.code_text = line;
                interpreter.pointer = 0;
                let result = interpreter.interpret_next_line();
                match result {
                    Ok(_) => {}
                    Err(e) => {
                        bail!(
                            "Compilation failed due to error in line {}\n\n\x1b[91m{}\x1b[0m",
                            index + 1,
                            e
                        );
                    }
                }
            }
            std::fs::write(output, interpreter.save_to_string($output)?)?;
            return Ok(());
        }
    };
}

fn main() -> Result<()> {
    let matches = command!()
        .subcommand(input_output!("csv"))
        .subcommand(input_output!("html"))
        .get_matches();

    exec_output!(matches, OutputType::Csv);
    exec_output!(matches, OutputType::Html);

    println!("Spreadsheet for Dummies");
    println!("Repl v{}", crate_version!());
    println!("Use 'Ctrl+C' or 'Ctrl-D' to quit\n");

    let mut rl = DefaultEditor::new()?;
    let _ = rl.load_history("history.txt");

    let mut interpreter = VM::new(String::new());
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                if line.starts_with("help") {
                    continue;
                }
                interpreter.code_text = line;
                interpreter.pointer = 0;
                if let Err(e) = interpreter.interpret() {
                    eprintln!("\x1b[91m{}\x1b[0m", e);
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(e) => bail!(e),
        }
    }
    let _ = rl.save_history("history.txt");

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::compiler::*;

    #[test]
    pub fn create_table() {
        let mut t = Table::new("t".to_string());
        t.assign_title("Table".to_string());
        t.extend_headers(vec![
            Cell::String("Header 1".to_string()),
            Cell::String("Header 2".to_string()),
            Cell::String("Header 3".to_string()),
        ]);
        t.append_row("Row 1 Col 1,Row 1 Col 2,Row 1 Col 3,Row 1 Col 4".to_string());

        assert_eq!(t.title(), "Table");
        assert_eq!(t.rows[0].len(), 3);
        assert_eq!(t.dimensions(), (1, 3));
    }

    #[test]
    pub fn create_table_from_code() {
        let code = r#"table1 create with title "Table 1"
            table1 create with title "overwritten"
            table1 create with title "overwritten once" with title "overwritten twice"
            "#;
        let mut vm = VM::new(code.to_string());
        assert!(vm.tables.len() == 0);
        vm.interpret_next_line().unwrap();
        assert!(vm.tables.len() == 1);
        assert_eq!(vm.tables[0].title(), "Table 1");

        vm.interpret_next_line().unwrap();
        assert!(vm.tables.len() == 1);
        assert_eq!(vm.tables[0].title(), "overwritten");

        vm.interpret_next_line().unwrap();
        assert!(vm.tables.len() == 1);
        assert_eq!(vm.tables[0].title(), "overwritten twice");
    }

    #[test]
    pub fn header_and_row() {
        let code = r#"t create_table with title "Table 1"
            t headers "Header 1" "Header 2" "Header 3"
            t add_row "Row 1 Col 1" "Row 1 Col 2" "Row 1 Col 3"
            t add_row
            t add_row "Row 3 Col 1" "87" "47.63" "99.1"
            t add_row "Row 4 Col 1" "32"
            "#;
        let mut vm = VM::new(code.to_string());

        vm.interpret_next_line().unwrap();
        vm.interpret_next_line().unwrap();
        assert!(vm.tables[0].rows.len() == 0);

        vm.interpret_next_line().unwrap();
        // assert_eq!(vm.tables[0].dimensions());
        assert!(vm.tables[0].rows.len() == 1);

        vm.interpret_next_line().unwrap();
        assert!(vm.tables[0].rows.len() == 2);

        vm.interpret_next_line().unwrap();
        assert!(vm.tables[0].rows.len() == 3);
        assert_eq!(
            vm.tables[0].rows[1],
            Row {
                cells: vec![Cell::Empty, Cell::Empty, Cell::Empty]
            }
        );
        assert_eq!(
            vm.tables[0].rows[2],
            Row {
                cells: vec![
                    Cell::String("Row 3 Col 1".to_string()),
                    Cell::Number(87.0),
                    Cell::Number(47.63),
                ]
            }
        );

        vm.interpret_next_line().unwrap();
        assert!(vm.tables[0].rows.len() == 4);
        assert_eq!(
            *vm.tables[0].rows.last().unwrap(),
            Row {
                cells: vec![
                    Cell::String("Row 4 Col 1".to_string()),
                    Cell::Number(32.0),
                    Cell::Empty
                ]
            }
        );
    }
}

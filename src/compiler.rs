pub use crate::table::*;
use crate::{autocorrect::keyboard_distance_matcher, closure::TableClosure, suggestion, warn};
use chatgpt::types::CompletionResponse;
use eyre::{bail, eyre, Result};
use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    str::FromStr,
};

pub enum ResponsePlotType {
    Bargraph((usize, usize)),
    Histogram(usize),
    Piechart(usize),
    Scatterplot((usize, usize)),
    DataInsufficient,
    None,
}
impl FromStr for ResponsePlotType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut text = s.lines().map(|s| s.trim());
        if let Some(plot_type) = text.next() {
            let mut read_tuple = || -> Option<(usize, usize)> {
                let x = text.next()?.split_whitespace().last()?;
                let y = text.next()?.split_whitespace().last()?;
                Some((x.parse::<usize>().ok()? - 1, y.parse::<usize>().ok()? - 1))
            };
            match plot_type.to_lowercase().as_str() {
                "bargraph" | "bar" => {
                    let tuple = read_tuple();
                    if let Some(tuple) = tuple {
                        Ok(ResponsePlotType::Bargraph(tuple))
                    } else {
                        Ok(ResponsePlotType::DataInsufficient)
                    }
                }
                "scatterplot" | "scatter" => {
                    let tuple = read_tuple();
                    if let Some(tuple) = tuple {
                        Ok(ResponsePlotType::Bargraph(tuple))
                    } else {
                        Ok(ResponsePlotType::DataInsufficient)
                    }
                }
                "histogram" | "hist" => {
                    let token = text.next().ok_or(())?.split_whitespace().last().ok_or(())?;
                    if let Some(idx) = token.parse::<usize>().ok() {
                        Ok(ResponsePlotType::Histogram(idx))
                    } else {
                        Ok(ResponsePlotType::DataInsufficient)
                    }
                }
                "piechart" | "pie" => {
                    let token = text.next().ok_or(())?.split_whitespace().last().ok_or(())?;
                    if let Some(idx) = token.parse::<usize>().ok() {
                        Ok(ResponsePlotType::Piechart(idx))
                    } else {
                        Ok(ResponsePlotType::DataInsufficient)
                    }
                }
                "data_insufficient" => Ok(ResponsePlotType::DataInsufficient),
                "none" => Ok(ResponsePlotType::None),
                _ => Err(()),
            }
        } else {
            Err(())
        }
    }
}

pub enum Operator {
    Assignment,
    AddRow,
    ExtendHeaders,
    Apply,
    DefineClosure,
    View,
    Prompt,
}

pub(crate) enum Token<'a> {
    Operator,
    Modifier,
    DataModifier,
    Table(&'a HashMap<String, usize>),
}

pub enum Modifier {
    WithHeader,
    WithTitle,
}

impl Modifier {
    pub fn is_modifier(token: &str) -> bool {
        matches!(token, "with" | "and")
    }
}
impl FromStr for Modifier {
    type Err = eyre::Report;
    fn from_str(s: &str) -> Result<Modifier> {
        match s {
            "header" | "headers" => Ok(Modifier::WithHeader),
            "title" => Ok(Modifier::WithTitle),
            unknown => {
                let correction = keyboard_distance_matcher(s, Token::DataModifier);
                suggestion!("Did you mean to use \"{correction}\"?");
                bail!("\"{unknown}\" is not a valid modifier")
            }
        }
    }
}

impl Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operator::Assignment => write!(f, "create"),
            Operator::AddRow => write!(f, "add_row"),
            Operator::ExtendHeaders => write!(f, "headers"),
            Operator::Apply => write!(f, "apply"),
            Operator::View => write!(f, "view"),
            Operator::DefineClosure => write!(f, "define"),
            Operator::Prompt => write!(f, "prompt"),
        }
    }
}

impl FromStr for Operator {
    type Err = eyre::Report;
    fn from_str(s: &str) -> Result<Operator> {
        Ok(match s {
            "create" | "create_table" | "=" => Operator::Assignment,
            "add_row" => Operator::AddRow,
            "headers" => Operator::ExtendHeaders,
            "apply" => Operator::Apply,
            "view" => Operator::View,
            "define" => Operator::DefineClosure,
            "prompt" => Operator::Prompt,
            unknown => {
                let correction = keyboard_distance_matcher(s, Token::Operator);
                suggestion!("Did you mean \x1b[96m{}\x1b[0m?", correction);
                bail!("\"{unknown}\" is not a valid operator")
            }
        })
    }
}

pub struct VM {
    pub(crate) code_text: String,
    pub(crate) tables: Vec<Table>,
    pub(crate) tables_idx: HashMap<String, usize>,
    pub(crate) pointer: usize,
    pub(crate) closure_idx: HashMap<String, usize>,
    pub(crate) closures: Vec<TableClosure>,
}

impl VM {
    pub fn new(code: String) -> VM {
        VM {
            code_text: code,
            tables: Vec::new(),
            tables_idx: HashMap::new(),
            pointer: 0,
            closure_idx: HashMap::new(),
            closures: Vec::new(),
        }
    }

    fn parse_double_quote(token: &mut std::str::SplitWhitespace<'_>) -> Result<String> {
        let token_start = token
            .next()
            .ok_or_else(|| eyre!("String literal not found."))?;
        if token_start.is_empty() || !token_start.starts_with('"') {
            suggestion!("String literals must be enclosed in double quotes (\").");
            return Err(eyre!(
                "String literal not found, found \"{token_start}\" instead."
            ));
        }
        if token_start.ends_with('"') && token_start.len() != 1 {
            return Ok(String::from(&token_start[1..token_start.len() - 1]));
        }
        let mut string = String::from(&token_start[1..]);

        while let Some(token) = token.next() {
            string.push(' ');
            if token.ends_with('"') {
                string.push_str(&token[..token.len() - 1]);
                return Ok(string);
            } else {
                string.push_str(token);
            }
        }
        warn!("String literal was not closed properly.");
        suggestion!("Try adding a double quote (\") at the end of the string literal.");
        Ok(string)
    }

    pub fn interpret(&mut self) -> Result<()> {
        while self.pointer < self.code_text.len() {
            self.interpret_next_line()?
        }
        return Ok(());
    }

    pub fn interpret_next_line(&mut self) -> Result<()> {
        let code_line_end = self.code_text[self.pointer..]
            .find('\n')
            .unwrap_or(self.code_text.len() - self.pointer);

        let code_line = self.code_text[self.pointer..self.pointer + code_line_end].trim();
        self.pointer += code_line_end + 1;
        if code_line.is_empty() {
            return Ok(());
        }

        let mut read_pointer = 0;
        let mut token = code_line.split_whitespace();
        let table_name_token = token
            .next()
            .ok_or_else(|| eyre!("{code_line}\n^\nNo token found referencing to table"))?;
        if table_name_token == "render" {
            // compiler intrinsic
            let mut file = File::create("table.html")?;
            file.write(b"<!DOCTYPE html><html><body><style>table,th,td{border:1px solid black;padding:3px;margin:2px;}</style>")?;
            file.write_all(&self.save(OutputType::Html)?)?;
            file.write(b"</body></html>")?;
            println!("Render successful.");
            return Ok(());
        }
        read_pointer += table_name_token.len() + 1;

        let operator = token.next().ok_or_else(|| {
            let table_exists = self.tables_idx.get(table_name_token);
            if let Some(&idx) = table_exists {
                suggestion!(
                    "Table {table_name} can be displayed with \x1b[96m{table_name_token} view\x1b[0m.",
                    table_name = self.tables[idx].title()
                )
            } else {
                suggestion!("Use \x1b[96m{table_name_token} create\x1b[0m to create a new table.");
                 if !self.tables.is_empty() {
                    let correction =
                        keyboard_distance_matcher(table_name_token, Token::Table(&self.tables_idx));
                    suggestion!("Table with name {correction} also exists.");
                }
            }
            eyre!(
                "{code_line}\n{carrot}\nNo token found for any operation",
                carrot = String::from_iter(
                    std::iter::repeat(' ')
                        .take(read_pointer)
                        .chain(std::iter::once('^'))
                ),
            )
        })?;
        let mut temp_read_pointer = read_pointer;
        read_pointer += operator.len() + 1;
        let operator = Operator::from_str(operator).map_err(|e| {
            eyre!(
                "{code_line}\n{carrot}\n{error}",
                carrot = String::from_iter(
                    std::iter::repeat(' ')
                        .take(temp_read_pointer)
                        .chain(std::iter::repeat('^').take(operator.len()))
                ),
                error = e,
            )
        })?;
        let table_idx = match operator {
            Operator::DefineClosure => 0_usize,
            Operator::Assignment => *self
                .tables_idx
                .entry(table_name_token.to_string())
                .and_modify(|&mut old_table| {
                    warn!(
                        "Table {table_name_token} ({title}) already existed, overwriting it.",
                        table_name_token = self.tables[old_table].identifier(),
                        title = self.tables[old_table].title()
                    );
                    self.tables[old_table] = Table::new(table_name_token.to_string());
                })
                .or_insert_with(|| {
                    self.tables.push(Table::new(table_name_token.to_string()));
                    self.tables.len() - 1
                }),
            _ => *self.tables_idx.get(table_name_token).ok_or_else(|| {
                if !self.tables.is_empty() {
                    let correction =
                        keyboard_distance_matcher(table_name_token, Token::Table(&self.tables_idx));
                    suggestion!("Did you mean to refer \"{correction}\" ?");
                }
                eyre!(
                    "{code_line}\n{carrot}\nNo table found with name \"{table_name_token}\"",
                    carrot = String::from_iter(std::iter::repeat('^').take(table_name_token.len())),
                )
            })?,
        }
        .clone();
        match operator {
            Operator::Assignment => {
                // Table was already created, now dealing with modifiers
                let table = &mut self.tables[table_idx];

                while let Some(modifier_token) = token.next() {
                    temp_read_pointer = read_pointer;
                    read_pointer += modifier_token.len() + 1;
                    if Modifier::is_modifier(modifier_token) {
                        let modifier_token = token.next().ok_or_else(|| {
                            eyre!(
                            "{code_line}\n{carrot}\nNo token found for modifier {modifier_token}.",
                            carrot = String::from_iter(
                                std::iter::repeat(' ')
                                    .take(temp_read_pointer)
                                    .chain(std::iter::repeat('^').take(modifier_token.len()))
                            ),
                        )
                        })?;

                        temp_read_pointer = read_pointer;
                        read_pointer += modifier_token.len() + 1;
                        let modifier = Modifier::from_str(modifier_token).map_err(|e| {
                            eyre!(
                                "{code_line}\n{carrot}\n{error}",
                                carrot = String::from_iter(
                                    std::iter::repeat(' ')
                                        .take(temp_read_pointer)
                                        .chain(std::iter::repeat('^').take(modifier_token.len()))
                                ),
                                error = e,
                            )
                        })?;

                        let mut cell_data = Vec::new();
                        while let Some(next_token) = token.clone().next() {
                            if !next_token.starts_with('"') {
                                break;
                            }
                            let cell_element =
                                Self::parse_double_quote(&mut token).map_err(|e| {
                                    eyre!(
                                        "{code_line}\n{carrot}\n{error}",
                                        carrot = String::from_iter(
                                            std::iter::repeat(' ').take(read_pointer).chain(
                                                std::iter::repeat('^').take(next_token.len())
                                            )
                                        ),
                                        error = e,
                                    )
                                })?;
                            read_pointer += cell_element.len() + 3;
                            cell_data.push(cell_element);
                        }

                        if cell_data.is_empty() {
                            bail!("{code_line}\n{carrot}\nNo data found for modifier {modifier_token}" ,carrot = String::from_iter(
                                    std::iter::repeat(' ')
                                        .take(temp_read_pointer)
                                        .chain(std::iter::repeat('^').take(modifier_token.len()))
                                ),);
                        }
                        match modifier {
                            Modifier::WithHeader => table.extend_headers(
                                cell_data
                                    .into_iter()
                                    .map(|s| Cell::from_str(&s))
                                    .collect::<Result<Vec<_>>>()?,
                            ),
                            Modifier::WithTitle => table.assign_title(cell_data.swap_remove(0)),
                        }
                    } else {
                        let correction = keyboard_distance_matcher(modifier_token, Token::Modifier);
                        suggestion!("You may want to use \x1b[96m{correction}\x1b[0m instead.");
                        warn!(
                        "Expected modifier but found \"{modifier_token}\", ignoring token\n{code_line}\n{carrot}",
                        carrot = String::from_iter(
                            std::iter::repeat(' ')
                                .take(temp_read_pointer)
                                .chain(std::iter::repeat('^').take(modifier_token.len()))
                        ),
                    );
                    }
                }
            }
            Operator::AddRow => {
                let mut cell_data = Vec::new();
                while let Some(next_token) = token.clone().next() {
                    let cell_element = Self::parse_double_quote(&mut token).map_err(|e| {
                        eyre!(
                            "{code_line}\n{carrot}\n{error}",
                            carrot = String::from_iter(
                                std::iter::repeat(' ')
                                    .take(read_pointer)
                                    .chain(std::iter::repeat('^').take(next_token.len()))
                            ),
                            error = e,
                        )
                    })?;
                    read_pointer += cell_element.len() + 3;
                    cell_data.push(cell_element);
                }

                let cell_data = cell_data.iter().map(AsRef::as_ref).collect();
                self.tables[table_idx].append_row_from(cell_data);
            }
            Operator::ExtendHeaders => {
                let mut cell_data = Vec::new();
                while let Some(next_token) = token.clone().next() {
                    let cell_element = Self::parse_double_quote(&mut token).map_err(|e| {
                        eyre!(
                            "{code_line}\n{carrot}\n{error}",
                            carrot = String::from_iter(
                                std::iter::repeat(' ')
                                    .take(read_pointer)
                                    .chain(std::iter::repeat('^').take(next_token.len()))
                            ),
                            error = e,
                        )
                    })?;
                    read_pointer += cell_element.len() + 3;
                    cell_data.push(cell_element);
                }

                let cell_data = cell_data
                    .into_iter()
                    .map(|x| Cell::from_str(&x).unwrap_or_default())
                    .collect();
                self.tables[table_idx].extend_headers(cell_data);
            }
            Operator::View => {
                let table = &self.tables[table_idx];
                println!("{}", table.save_to_string(OutputType::Csv)?);
                table.table_view()?;
            }
            Operator::Apply => {
                // this will be highly experimental code
                warn!("This feature is in alpha stage, it may not work as expected.");
                let closure_name_token = token.next().ok_or_else(|| {
                    eyre!(
                        "{code_line}\n{carrot}\nNo token found for closure name.",
                        carrot = String::from_iter(
                            std::iter::repeat(' ')
                                .take(read_pointer)
                                .chain(std::iter::repeat('^').take(1))
                        ),
                    )
                })?;
                read_pointer += closure_name_token.len() + 1;
                if let Some(&closure) = self.closure_idx.get(closure_name_token) {
                    let closure = &self.closures[closure];
                    let table = &mut self.tables[table_idx];
                    let mut get_coord = || -> Result<(usize, usize)> {
                        let start_i = token.next().ok_or_else(|| {
                            eyre!(
                                "{code_line}\n{carrot}\nNo token found for row-index.",
                                carrot = String::from_iter(
                                    std::iter::repeat(' ')
                                        .take(read_pointer)
                                        .chain(std::iter::repeat('^').take(1))
                                ),
                            )
                        })?;
                        read_pointer += start_i.len() + 1;
                        let start_j = token.next().ok_or_else(|| {
                            eyre!(
                                "{code_line}\n{carrot}\nNo token found for column-index.",
                                carrot = String::from_iter(
                                    std::iter::repeat(' ')
                                        .take(read_pointer)
                                        .chain(std::iter::repeat('^').take(1))
                                ),
                            )
                        })?;
                        let start = (start_i.parse::<usize>()?, start_j.parse::<usize>()?);
                        Ok(start)
                    };
                    let start = get_coord()?;
                    let end = get_coord().unwrap_or_else(|e| {
                        warn!("Defaulting end to start cell address. {e}");
                        start
                    });
                    closure.apply(table, start, end, &self.closures)?;
                } else {
                    bail!(
                        "{code_line}\n{carrot}\nNo closure found with name {closure_name_token}.",
                        carrot = String::from_iter(
                            std::iter::repeat(' ')
                                .take(read_pointer)
                                .chain(std::iter::repeat('^').take(closure_name_token.len()))
                        ),
                    );
                }
            }
            Operator::DefineClosure => {
                // as it contains invalid data, so to prevent accidental use
                drop(table_idx);
                warn!("This feature is in alpha stage, it may not work as expected.");
                let defination = token.collect::<Vec<&str>>().join(" ");
                let defination = TableClosure::new(defination)?;
                let closure_idx = *self
                    .closure_idx
                    .entry(table_name_token.to_string())
                    .and_modify(|_| {
                        warn!("Closure {table_name_token} already existed, overwriting it.",);
                    })
                    .or_insert_with(|| self.closures.len());
                if let Some(v) = self.closures.get_mut(closure_idx) {
                    *v = defination;
                } else {
                    self.closures.push(defination);
                }
            }
            Operator::Prompt => {
                use chatgpt::prelude::*;

                let prompt = token.collect::<Vec<&str>>().join(" ");
                let prompt = prompt.trim_matches('"');
                let table = &self.tables[table_idx];
                if self.tables.len() > 1 {
                    warn!("You can only reference one table in prompt for now.");
                }
                let mut prompt_text = format!(
                    "I am working with CSV files. My table is as follows:\nTable name:{table}\nI want you to answer my next following question. You are expected to keep the answer as short as possible.\n",
                    table = table.to_csv()?
                );
                prompt_text.push_str(prompt);
                // println!("Prompt: {}", prompt_text);
                let client = ChatGPT::new(env!(
                    "OPENAI_KEY",
                    "OpenAI key not provided! Required to use prompt."
                ))?;
                use tokio::runtime::Runtime;
                let rt = Runtime::new().unwrap();

                let prompt_success: Result<()> = rt.block_on(async {
                    let response = client.send_message(prompt_text).await?;
                    println!("Response: {}", response.message().content);
                    Ok(())
                });
                prompt_success?;
            }
        }
        Ok(())
    }
}

impl Save for VM {
    fn to_csv(&self) -> Result<String> {
        self.tables
            .iter()
            .map(|table| table.to_csv())
            .collect::<Result<Vec<_>>>()
            .map(|tables| tables.join("\n"))
    }

    fn to_html(&self) -> Result<String> {
        self.tables
            .iter()
            .map(|table| table.to_html())
            .collect::<Result<Vec<_>>>()
            .map(|tables| tables.join("\n"))
    }
}

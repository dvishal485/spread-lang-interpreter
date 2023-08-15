use eyre::{bail, Result};
use serde::Serialize;
use std::{
    fmt::Display,
    fs::File,
    io::Write,
    ops::{Add, AddAssign},
    path::PathBuf,
    str::FromStr,
};

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ({
        use std::io::Write;
        writeln!(&mut std::io::stderr(), "\x1b[93m\x1b[1mWARNING:\x1b[0m {}", format!($($arg)*)).expect("Could not write to stderr");
    })
}

#[macro_export]
macro_rules! suggestion {
    ($($arg:tt)*) => ({
        use std::io::Write;
        writeln!(&mut std::io::stdout(), "\x1b[96m\x1b[1mSuggestion:\x1b[0m {}", format!($($arg)*)).expect("Could not write to stdout");
    })
}

pub trait Save {
    fn to_csv(&self) -> Result<String>;
    fn to_html(&self) -> Result<String>;
    fn save_to_string(&self, output_type: OutputType) -> Result<String> {
        match output_type {
            OutputType::Html => self.to_html(),
            OutputType::Csv => self.to_csv(),
        }
    }
    fn save_to_file(&self, output_type: OutputType, path: PathBuf) -> Result<()> {
        let save_str = self.save_to_string(output_type)?;
        std::fs::write(path, save_str)?;
        Ok(())
    }
    fn save(&self, output_type: OutputType) -> Result<Vec<u8>> {
        Ok(self.save_to_string(output_type)?.into_bytes())
    }
}

pub struct Table {
    title: String,
    ident: String,
    headers: Row,
    pub(crate) rows: Vec<Row>,
}
#[derive(Debug, PartialEq)]
pub struct Row {
    pub(crate) cells: Vec<Cell>,
}
impl Row {
    pub fn new(row_size: usize) -> Row {
        Row {
            cells: Vec::from_iter(std::iter::repeat(Cell::Empty).take(row_size)),
        }
    }
    pub fn len(&self) -> usize {
        self.cells.len()
    }
}
#[derive(Default, Clone, Debug, PartialEq, serde::Deserialize)]
pub enum Cell {
    String(String),
    Number(f64),
    Boolean(bool),
    #[default]
    Empty,
}

impl Serialize for Cell {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Cell::String(s) => serializer.serialize_str(s),
            Cell::Number(n) => serializer.serialize_f64(*n),
            Cell::Boolean(b) => serializer.serialize_bool(*b),
            Cell::Empty => serializer.serialize_none(),
        }
    }
}

impl Add for Cell {
    type Output = Cell;

    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Cell::String(s) => Cell::String(format!("{}{}", s, rhs)),
            Cell::Number(n) => match rhs {
                Cell::String(s) => {
                    if let Ok(n2) = s.parse::<f64>() {
                        Cell::Number(n + n2)
                    } else {
                        Cell::String(format!("{}{}", n, s))
                    }
                }
                Cell::Number(n2) => Cell::Number(n + n2),
                Cell::Boolean(b) => Cell::Number(n + b as u8 as f64),
                Cell::Empty => Cell::Number(n),
            },
            Cell::Boolean(b) => match rhs {
                Cell::String(s) => {
                    if let Ok(n) = s.parse::<f64>() {
                        Cell::Number(b as u8 as f64 + n)
                    } else {
                        Cell::String(format!("{}{}", b, s))
                    }
                }
                Cell::Number(n) => Cell::Number(b as u8 as f64 + n),
                Cell::Boolean(b2) => Cell::Boolean(b || b2),
                Cell::Empty => Cell::Boolean(b),
            },
            Cell::Empty => match rhs {
                Cell::Empty => Cell::Empty,
                _ => rhs,
            },
        }
    }
}
impl AddAssign for Cell {
    fn add_assign(&mut self, rhs: Self) {
        match self {
            Cell::String(s) => s.push_str(&rhs.to_string()),
            Cell::Number(n) => match rhs {
                Cell::Empty => {}
                Cell::Number(n2) => *n += n2,
                Cell::Boolean(b) => *n += b as u8 as f64,
                Cell::String(s) => {
                    if let Ok(n2) = s.parse::<f64>() {
                        *n += n2;
                    } else {
                        *self = Cell::String(format!("{}{}", n, s));
                    }
                }
            },
            Cell::Boolean(b) => match rhs {
                Cell::String(s) => {
                    if let Ok(n) = s.parse::<f64>() {
                        *self = Cell::Number(*b as u8 as f64 + n);
                    } else {
                        *self = Cell::String(format!("{}{}", b, s));
                    }
                }
                Cell::Number(n) => *self = Cell::Number(*b as u8 as f64 + n),
                Cell::Boolean(b2) => *b = *b || b2,
                Cell::Empty => {}
            },
            Cell::Empty => match rhs {
                Cell::Empty => {}
                _ => *self = rhs,
            },
        }
    }
}

impl std::ops::Sub for Cell {
    type Output = Cell;

    fn sub(self, rhs: Self) -> Self::Output {
        match self {
            Cell::Number(n1) => match rhs {
                Cell::String(s) => {
                    if let Ok(n2) = s.parse::<f64>() {
                        Cell::Number(n1 - n2)
                    } else {
                        Cell::String(format!("{}{}", n1, s))
                    }
                }
                Cell::Number(n2) => Cell::Number(n1 - n2),
                Cell::Boolean(b) => Cell::Number(n1 - b as u8 as f64),
                Cell::Empty => self,
            },
            Cell::Boolean(b) => match rhs {
                Cell::String(s) => {
                    if let Ok(n) = s.parse::<f64>() {
                        Cell::Number(b as u8 as f64 - n)
                    } else {
                        Cell::String(format!("{}{}", b, s))
                    }
                }
                Cell::Number(n) => Cell::Number(b as u8 as f64 - n),
                Cell::Boolean(b2) => Cell::Boolean(b ^ b2),
                Cell::Empty => self,
            },
            Cell::String(s) => match rhs {
                Cell::String(s2) => Cell::String(s.replace(&s2, "")),
                Cell::Number(n) => {
                    if let Ok(n2) = s.parse::<f64>() {
                        Cell::Number(n2 - n)
                    } else {
                        Cell::String(s) - Cell::String(n.to_string())
                    }
                }
                Cell::Boolean(b) => {
                    if let Ok(n) = s.parse::<f64>() {
                        Cell::Number(n - b as u8 as f64)
                    } else {
                        Cell::String(s) - Cell::String(b.to_string())
                    }
                }
                Cell::Empty => Cell::String(s),
            },
            Cell::Empty => match rhs {
                Cell::Empty => Cell::Empty,
                Cell::Number(n) => Cell::Number(-n),
                Cell::String(_) => Cell::Empty,
                Cell::Boolean(b) => Cell::Boolean(!b),
            },
        }
    }
}

impl std::ops::Mul for Cell {
    type Output = Result<Cell>;

    fn mul(self, rhs: Self) -> Self::Output {
        let lhs = match self {
            Cell::Number(n) => n,
            Cell::Boolean(b) => b as u8 as f64,
            Cell::Empty => return Ok(rhs),
            _ => bail!("Cell of type string cannot be multiplied."),
        };
        let rhs = match rhs {
            Cell::Number(n) => n,
            Cell::Boolean(b) => b as u8 as f64,
            Cell::Empty => return Ok(self),
            _ => bail!("Cell of type string cannot be multiplied."),
        };

        Ok(Cell::Number(lhs * rhs))
    }
}
impl std::ops::Div for Cell {
    type Output = Result<Cell>;

    fn div(self, rhs: Self) -> Self::Output {
        let lhs = match self {
            Cell::Number(n) => n,
            Cell::Boolean(b) => b as u8 as f64,
            Cell::Empty => return Ok(rhs),
            _ => bail!("Cell of type a string cannot be divided."),
        };
        let rhs = match rhs {
            Cell::Number(n) if n == 0_f64 => bail!("Cannot divide by zero!"),
            Cell::Boolean(false) => bail!("Cannot divide by zero!"),
            Cell::Number(n) if n != 0_f64 => n,
            Cell::Boolean(true) => 1_f64,
            Cell::Empty => return Ok(self),
            _ => bail!("Cell cannot be divided by a string."),
        };

        Ok(Cell::Number(lhs / rhs))
    }
}

impl std::ops::Rem for Cell {
    type Output = Result<Cell>;

    fn rem(self, rhs: Self) -> Self::Output {
        let lhs = match self {
            Cell::Number(n) => n,
            Cell::Boolean(b) => b as u8 as f64,
            _ => bail!("Cell of type a string or empty cell cannot be divided."),
        };
        let rhs = match rhs {
            Cell::Number(n) if n == 0_f64 => bail!("Cannot divide by zero!"),
            Cell::Boolean(false) => bail!("Cannot divide by zero!"),
            Cell::Number(n) if n != 0_f64 => n,
            Cell::Boolean(true) => 1_f64,
            _ => bail!("Cell cannot be divided by a string or empty cell."),
        };

        Ok(Cell::Number(lhs % rhs))
    }
}

impl FromStr for Cell {
    type Err = eyre::Report;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Cell::Empty)
        } else if s.parse::<f64>().is_ok() {
            Ok(Cell::Number(s.parse::<f64>().unwrap()))
        } else if s.parse::<bool>().is_ok() {
            Ok(Cell::Boolean(s.parse::<bool>().unwrap()))
        } else {
            Ok(Cell::String(s.to_string()))
        }
    }
}

impl std::fmt::Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cell::String(s) => write!(f, "{}", s),
            Cell::Number(n) => write!(f, "{}", n),
            Cell::Boolean(b) => write!(f, "{}", b),
            Cell::Empty => write!(f, ""),
        }
    }
}
pub enum OutputType {
    Html,
    Csv,
}

impl Display for OutputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputType::Html => write!(f, "html"),
            OutputType::Csv => write!(f, "csv"),
        }
    }
}

impl Display for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let output = self.save_to_string(OutputType::Csv).unwrap();
        write!(f, "{}", output)
    }
}
impl Table {
    pub fn new(ident: String) -> Table {
        Table {
            title: ident.clone(),
            ident,
            headers: Row::new(0),
            rows: Vec::new(),
        }
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn identifier(&self) -> &str {
        &self.ident
    }
    pub fn assign_title(&mut self, title: String) {
        self.title = title;
    }
    pub fn dimensions(&self) -> (usize, usize) {
        (self.rows.len(), self.headers.len())
    }
    pub fn extend_headers(&mut self, headers: Vec<Cell>) {
        self.headers.cells.extend(headers);
    }
    pub(crate) fn create_empty_row(&mut self) {
        let row = Row::new(self.headers.len());
        self.rows.push(row);
    }
    fn parse_row_splitted(&self, cell_tokens: Vec<&str>) -> Option<Row> {
        if self.headers.len() == 0 {
            warn!("Headers are not set, adding data before setting headers is not allowed.");
            suggestion!(
                "Headers can be added using\n$ \x1b[96m{table_name} headers {header_text}\x1b[0m",
                table_name = self.ident,
                header_text =
                    String::from_iter((1..=cell_tokens.len()).map(|i| format!("\"Header {i}\" ")))
            );
            return None;
        }
        if cell_tokens.len() > self.headers.len() {
            warn!(
                "Row size is greater than the header size ({row_len} > {header_len}), truncating Row size to match header size",
                row_len = cell_tokens.len(),
                header_len = self.headers.len()
            );
        }
        Some(Row {
            cells: cell_tokens
                .into_iter()
                .map(|cell| Cell::from_str(cell).unwrap_or_default())
                .chain(std::iter::repeat(Cell::Empty))
                .take(self.headers.len())
                .collect(),
        })
    }
    fn parse_row(&self, row: String) -> Option<Row> {
        let cell_tokens: Vec<_> = row.split(',').collect();
        self.parse_row_splitted(cell_tokens)
    }
    pub fn append_row(&mut self, row: String) {
        let row = self.parse_row(row);
        if let Some(row) = row {
            self.rows.push(row);
        }
    }
    pub fn append_row_from(&mut self, row: Vec<&str>) {
        let row = self.parse_row_splitted(row);
        if let Some(row) = row {
            self.rows.push(row);
        }
    }
    pub fn get_cell(&self, row: usize, col: usize) -> Result<&Cell> {
        println!("dimen = {:?}; ({},{})", self.dimensions(), row, col);
        self.rows
            .get(row)
            .ok_or(eyre::eyre!("Row index out of bound."))?
            .cells
            .get(col)
            .ok_or(eyre::eyre!("Column index out of bound."))
    }
    pub fn get_cell_mut(&mut self, row: usize, col: usize) -> Result<&mut Cell> {
        self.rows
            .get_mut(row)
            .ok_or(eyre::eyre!("Row index out of bound."))?
            .cells
            .get_mut(col)
            .ok_or(eyre::eyre!("Column index out of bound."))
    }
    pub fn get_column(&self, col: usize) -> Result<Vec<&Cell>> {
        self.rows
            .iter()
            .map(|row| {
                row.cells
                    .get(col)
                    .ok_or(eyre::eyre!("Column index out of bound."))
            })
            .collect::<Result<Vec<_>>>()
    }
    pub fn get_row(&self, row: usize) -> Result<&Row> {
        self.rows
            .get(row)
            .ok_or(eyre::eyre!("Row index out of bound."))
    }
    pub fn table_view(&self) -> Result<()> {
        use plotly::{Plot, Scatter};
        let mut plot = Plot::new();
        let c1 = self.get_column(0)?.into_iter().map(|x| x.clone()).collect();
        let c2 = self.get_column(1)?.into_iter().map(|x| x.clone()).collect();
        let trace = Scatter::new(c1, c2)
            .name(self.title())
            .x_axis(self.headers.cells[0].to_string())
            .y_axis(self.headers.cells[1].to_string());
        plot.add_trace(trace);
        plot.write_html("plot.html");
        let html_table = self.to_html()?;
        // write table to table.html
        let mut file = File::create("table.html")?;
        file.write(b"<!DOCTYPE html><html><body><style>table,th,td{border:1px solid black;padding:3px;margin:2px;}</style>")?;
        file.write_all(html_table.as_bytes())?;
        file.write(b"</body></html>")?;
        Ok(())
    }
}

impl Save for Table {
    fn to_csv(&self) -> Result<String> {
        let mut csv = String::new();
        csv.push_str(&self.title);
        csv.push_str("\n");
        for cell in &self.headers.cells {
            csv.push_str(cell.to_string().as_str());
            csv.push(',');
        }
        csv.push_str("\n");
        for row in &self.rows {
            for cell in &row.cells {
                csv.push_str(cell.to_string().as_str());
                csv.push(',');
            }
            csv.push('\n');
        }
        Ok(csv)
    }

    fn to_html(&self) -> Result<String> {
        let mut html = String::new();
        html.push_str(&self.title);
        html.push_str("<br><table><tr>");
        for cell in &self.headers.cells {
            html.push_str("<th>");
            html.push_str(cell.to_string().as_str());
            html.push_str("</th>");
        }
        html.push_str("</tr>");
        for row in &self.rows {
            html.push_str("<tr>");
            for cell in &row.cells {
                html.push_str("<td>");
                html.push_str(cell.to_string().as_str());
                html.push_str("</td>");
            }
            html.push_str("</tr>");
        }
        html.push_str("</table><br/>");
        Ok(html)
    }
}

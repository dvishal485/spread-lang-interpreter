use std::str::FromStr;

use crate::{
    compiler::{Cell, Table, VM},
    suggestion,
};
use eyre::{bail, Result};
pub(crate) enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
    Mod,
    And,
    Or,
    Xor,
}

pub(crate) enum StorageType {
    Accumulator(usize, usize),
    Register(usize),
}

impl Operation {
    pub(crate) fn apply(&self, a: Cell, b: Cell) -> Result<Cell> {
        match self {
            Self::Add => Ok(a + b),
            Self::Subtract => Ok(a - b),
            Self::Multiply => a * b,
            Self::Divide => a / b,
            Self::Mod => a % b,
            _ => todo!(),
            /* Self::And => (a as u64 & b as u64) as f64,
            Self::Or => (a as u64 | b as u64) as f64,
            Self::Xor => (a as u64 ^ b as u64) as f64, */
        }
    }
}
impl FromStr for Operation {
    type Err = eyre::Report;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "+" => Ok(Self::Add),
            "-" => Ok(Self::Subtract),
            "*" => Ok(Self::Multiply),
            "/" => Ok(Self::Divide),
            "%" => Ok(Self::Mod),
            "&" => Ok(Self::And),
            "|" => Ok(Self::Or),
            "^" => Ok(Self::Xor),
            _ => bail!("Invalid operation."),
        }
    }
}

pub(crate) struct FunctionExecutor<'a> {
    closure: &'a TableClosure,
    pub(crate) registers: Vec<Cell>,
    pub(crate) accumulator: Option<(usize, usize)>,
}
impl TableClosure {
    pub(crate) fn new(mut defination: String) -> Result<Self> {
        let components = defination.split(';').collect::<Vec<_>>();
        let postscript = components[2].to_string();
        let script = components[1].to_string();
        defination.truncate(components[0].len());
        Ok(Self {
            prescript: defination,
            script,
            postscript,
        })
    }
    pub fn apply(
        &self,
        table: &mut Table,
        cell_start: (usize, usize),
        cell_end: (usize, usize),
        table_closures: &Vec<TableClosure>,
    ) -> Result<()> {
        FunctionExecutor {
            closure: self,
            registers: Vec::new(),
            accumulator: None,
        }
        .apply(table, cell_start, cell_end, table_closures)
    }
}

#[derive(Debug)]
pub(crate) struct TableClosure {
    pub(crate) prescript: String,
    pub(crate) script: String,
    pub(crate) postscript: String,
}
impl FunctionExecutor<'_> {
    pub(crate) fn do_artihmetic(
        &self,
        raw_input: &str,
        storage_cell: &Cell,
        (curr_i, curr_j): (usize, usize),
        table: &Table,
        vm: &Vec<TableClosure>,
    ) -> Result<Cell> {
        let mut stack: Vec<Cell> = Vec::new();
        let raw_input = raw_input.trim();
        // this will be in postfix notation
        let mut raw_input = raw_input.split_whitespace();
        while let Some(token) = raw_input.next() {
            // todo: allow to do arthmetic on cells indexes
            // by recursively calling do_arthmetic
            // find a way to parse this
            // suggestion: use a closed bracket detection
            if token.starts_with("~cell(") {
                let token = token
                    .strip_prefix("~cell(")
                    .unwrap()
                    .strip_suffix(')')
                    .ok_or_else(|| eyre::eyre!("Invalid cell reference."))?;
                let (i, j) = token.split_once(',').ok_or_else(|| {
                    eyre::eyre!("Invalid cell reference. Must be in the form '~cell(i,j)'")
                })?;
                let i = i.parse::<usize>()?;
                let j = j.parse::<usize>()?;
                stack.push(table.get_cell(i, j)?.clone());
                continue;
            }
            match token {
                "~op" => stack.push(storage_cell.clone()),
                "~cell" => stack.push(table.get_cell(curr_i, curr_j)?.clone()),
                "~cell.x" => stack.push(Cell::Number(curr_i as f64)),
                "~cell.y" => stack.push(Cell::Number(curr_j as f64)),
                "~fn" => {
                    let fn_token = raw_input.next();
                }
                _ => {
                    if let Ok(op) = token.parse::<Operation>() {
                        let b = stack.pop().ok_or_else(|| {
                            eyre::eyre!("Invalid postfix expression. Not enough operands.")
                        })?;
                        let a = stack.pop().ok_or_else(|| {
                            eyre::eyre!("Invalid postfix expression. Not enough operands.")
                        })?;
                        stack.push(op.apply(a, b)?);
                    } else {
                        stack.push(Cell::from_str(token)?);
                    }
                }
            };
        }
        if stack.len() > 1 {
            bail!("Invalid prefix expression. Too many values.");
        }
        Ok(stack.pop().unwrap_or(Cell::Empty))
    }
    pub(crate) fn apply(
        &mut self,
        table: &mut Table,
        cell_start: (usize, usize),
        cell_end: (usize, usize),
        table_closures: &Vec<TableClosure>,
    ) -> Result<()> {
        let dimensions = table.dimensions();
        if dimensions < cell_start || dimensions < cell_end {
            bail!("Cell index out of range. Dimensions: {dimensions:?}, Cell start: {cell_start:?}, Cell end: {cell_end:?}");
        }
        self.apply_prescript(table, cell_start, cell_end, table_closures)?;
        let script = &self.closure.script;
        let mut reader = script.split('!');
        while let Some(token) = reader.next() {
            let token = token.trim();
            let mut get_storage_cell = || -> Result<Cell> {
                let storage = reader.next().ok_or_else(|| {
                    suggestion!("Use 'acc' or 'reg' to specify a storage location.");
                    eyre::eyre!("Storage location not specified.")
                })?;
                let storage = storage.trim();
                // right now storage type is redundant
                let storage_type: StorageType;
                Ok(match storage {
                    "acc" => {
                        let Some(acc) = self.accumulator else {
                             bail!("Accumulator was not defined in the prescript.");
                            };
                        storage_type = StorageType::Accumulator(acc.0, acc.1);
                        let storage_cell = std::mem::take(table.get_cell_mut(acc.0, acc.1)?);
                        storage_cell
                    }
                    "reg" => {
                        let reg_id = reader
                            .next()
                            .ok_or_else(|| {
                                suggestion!("Use a number to specify a register.");
                                eyre::eyre!("Register ID not specified.")
                            })?
                            .parse::<usize>()?;
                        storage_type = StorageType::Register(reg_id);
                        let reg_len = self.registers.len();
                        std::mem::take(self.registers.get_mut(reg_id).ok_or_else(|| {
                            eyre::eyre!(
                                "Register ID out of range. (index is {} but length is {})",
                                reg_id,
                                reg_len
                            )
                        })?)
                    }
                    _ => {
                        suggestion!("Use 'acc' or 'reg' to specify a storage location.");
                        bail!("Invalid storage location.");
                    }
                })
            };
            match token {
                "each" => {
                    let mut storage_cell = get_storage_cell()?;
                    let closure = reader
                        .next()
                        .ok_or_else(|| eyre::eyre!("Invalid closure. No closure specified."))?;
                    for cell_i in (cell_start.0)..=cell_end.0 {
                        for cell_j in (cell_start.1)..=cell_end.1 {
                            storage_cell = self.do_artihmetic(
                                closure,
                                &storage_cell,
                                (cell_i, cell_j),
                                table,
                                table_closures,
                            )?;
                        }
                    }
                    /* match storage_type {
                        StorageType::Accumulator(x, y) => {
                            std::mem::replace(table.get_cell_mut(x, y)?, storage_cell);
                        }
                        StorageType::Register(reg_id) => {
                            self.registers[reg_id] = storage_cell;
                        }
                    } */
                    if let Some((x, y)) = self.accumulator {
                        *table.get_cell_mut(x, y)? = storage_cell;
                    }
                    return self.apply_postscript(table, cell_start, cell_end);
                }
                "raw" => {
                    let mut storage_cell = get_storage_cell()?;
                    let closure = reader
                        .next()
                        .ok_or_else(|| eyre::eyre!("Invalid closure. No closure specified."))?;
                    storage_cell = self.do_artihmetic(
                        closure,
                        &storage_cell,
                        (cell_start.0, cell_start.0),
                        table,
                        table_closures,
                    )?;
                    if let Some((x, y)) = self.accumulator {
                        *table.get_cell_mut(x, y)? = storage_cell;
                    }
                    return self.apply_postscript(table, cell_start, cell_end);
                }
                /* "acc" => {}
                "reg" => {}
                "mem" => {}
                "if" => {} */
                _ => bail!("Invalid token in closure."),
            }
        }

        Ok(())
    }
    pub(crate) fn apply_postscript(
        &mut self,
        table: &mut Table,
        cell_start: (usize, usize),
        cell_end: (usize, usize),
    ) -> Result<()> {
        self.destroy();
        Ok(())
    }
    pub(crate) fn apply_prescript(
        &mut self,
        table: &mut Table,
        cell_start: (usize, usize),
        cell_end: (usize, usize),
        table_closures: &Vec<TableClosure>,
    ) -> Result<()> {
        let script = &self.closure.prescript;
        let mut reader = script.split('!');
        // defines accumulator, registers, and memory
        while let Some(acc) = reader.next() {
            let acc = acc.trim();
            match acc {
                "auto" => {
                    let output_cell = (cell_end.0 + 1, cell_end.1);
                    self.accumulator = Some(output_cell);
                    if table.dimensions() < (output_cell.0 + 1, output_cell.1 + 1) {
                        table.create_empty_row();
                    }
                }
                "reg" => {
                    let reg_count = reader
                        .next()
                        .ok_or_else(|| {
                            eyre::eyre!("Register count not specified. Must be a positive integer.")
                        })?
                        .parse::<usize>()?;
                    self.registers = vec![Cell::Empty; reg_count];
                }
                _ => {
                    let (acc_i, acc_j) = acc.split_once(',').ok_or_else(|| {
                        eyre::eyre!("Accumulator defined incorrectly. Must be 'auto' or 'none' or a accepted comma sepated index pair.")
                    })?;
                    let temp_cell = Cell::Empty;
                    let acc_i = self.do_artihmetic(
                        acc_i,
                        &temp_cell,
                        (cell_end.0 + 1, cell_end.1),
                        table,
                        table_closures,
                    )?;
                    let acc_j = self.do_artihmetic(
                        acc_j,
                        &temp_cell,
                        (cell_start.0, cell_start.1),
                        table,
                        table_closures,
                    )?;
                    let Cell::Number(x) = acc_i else {
                        bail!("Accumulator defined incorrectly. Must be 'auto' or 'none' or a accepted comma sepated index pair.")
                    };
                    let Cell::Number(y) = acc_j else {
                        bail!("Accumulator defined incorrectly. Must be 'auto' or 'none' or a accepted comma sepated index pair.")
                    };

                    let output_cell = (x.ceil() as usize, y.ceil() as usize);
                    self.accumulator = Some(output_cell);
                    while table.dimensions().0 < output_cell.0 {
                        table.create_empty_row();
                    }
                }
            }
        }

        Ok(())
    }
    pub(crate) fn destroy(&mut self) {
        self.registers.clear();
        self.accumulator = None;
    }
}

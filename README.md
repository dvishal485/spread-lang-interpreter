# spread-lang-interpreter

## Backend for spread-lang

A dummy language interpreter to generate spreadsheet files (csv format) and visualize the data with AI generated graph and ask questions from the interpreted data.

The language essentially is meant to be **turing complete** so one can perform any operations using the compiler/repl and hence can be used as a programming language as well! Thanks to the [power of closures](#closure) (experimental)

---

## Agenda

- Make it easier introduce programming to school students. How?
  
  This project at the simplest level involves essentially using a sort of "code" to manipulate and modify data. This may not be the best way, but definately, as "Excel" based programs are something which students are used familar with, visualizing and using the Excel's operation using code may help to introduce to the world of working by logic building.
- To create a dummy implementation of an extensible module which can do work like a regular programming language.
  
  Closures are supposed to be highly extensible as they can borrow the whole data mutably and perform operations as prescribed in a regular language (automata).
- Play with visualization with real-time data to HTML based tables and simple graphs using GPT-3.
  
  Currently supports: Bar graph, Histogram, and Scatter plot. Support of complex graphs will be added in future versions.

---

## Features

- Data arranged in rows and columns
- Dummy programming language **(work in progress)**
- Auto correct support for basic commands
- Immutable data by default (data once added can only be mutated by using closures)
- Compile code to csv
- Interpreter (repl)
- Support of closures (a kind of function) **(alpha stage)**
- Data query using the power of GPT-3
- Graph generation using [Dash Ploty](https://dash.plotly.com/) with suggestions from GPT-3

---

## Working with Electron repl

Apart from the CLI interface, the electron based repl can be used to visualize data in form of regular tables so it is easier to understand what is happening.

### Prerequisites

1. [Node v18+](https://nodejs.org/en/download)
2. [Rust compiler](https://www.rust-lang.org/tools/install) and Cargo
3. Node package manager (`npm` or `yarn`).

To get started, follow the steps:

- Build the binary for [spread-lang-interpreter](https://github.com/dvishal485/spread-lang-interpreter) and place it in the `assets` directory.

  ```bash
  git clone https://github.com/dvishal485/spread-lang-interpreter
  cd spread-lang-interpreter
  cargo build --release
  cp ./target/release/spreadsheet ~/spreadsheet
  ```

- Clone the repository and move the binary built in last step into assets folder.

  ```bash
  git clone https://github.com/dvishal485/spread-lang.git
  cd spread-lang
  mv ~/spreadsheet ./assets/spreadsheet
  ```

- Install nodejs dependencies.

  ```bash
  yarn install
  ```

- Run the application.

  ```bash
  yarn start
  ```

---

## Language Syntax

- `table_var create` will create a table and assign it to variable `table_var`.
- Title and headers (title of columns) can be set while creating the table as:

  ```bash
  table_name create with title "table name" and headers "header 1" "header 2"
  ```

- Headers can be set using `table_var headers "header 1" "header 2"`.
- Row can be added with `table_var add_row "content 1" "1"`.
- To view a table, use `table_var view`. This also generates the graph using Ploty by querying GPT-3.
- Closures can be defined using `closure_var define [closure]`.
- Closures can be applied as:
  
  ```bash
  table_var apply closure_var start_row start_col end_row end_col
  ```

  where (`start_row`, `start_col`) are 0-based indices of the starting cell and (`end_row`, `end_col`) are 0-based indices of ending cell on which closure is to be applied.

- To query with context of data use `table_var prompt [Your query here]`.
- `t render` generates the table visualisation into static HTML pages.

---

## Closure

***Note:** Closures are currently highly experimental feature.*

### What is closure?

- Closure act as a function to be applied on a range of cells.
- Closure can be used to apply certain formulas/computation on the cells.
- Closure is essentially a command written considering a strict set of rules.
- Unlike other commands, closure can't be verified for correctness. Incorrect closure is possible to be declared but they can't be applied (will throw error at runtime).
- Closure can borrow data mutably and hence perform any kind of operation.

### Internal Structure of Closure

- A defination of closure consists of three parts:
  1. Prescript
  1. Script
  1. Postscript
- They are stored as a String and are not interpreted unless applied.
- Closure on applying can access table cells on which it is applied, and registers.
- Registers are temporary cells which can be used for intermediate computation.
- Number of Registers a closure can access is restricted and defined within prescript.
- Even developer feels they are black-magic.

### Anatomy of Closure

- Closure looks like this: `prescript;script;postscript`.
- Prescript contains information about output cell to be used, number of registers required by the closure.
- Script contains information about the set of instructions to be applied on cell range, usually defined with `each!`.
- Postscript is the destructor of the closure.
- All arithmetics are written in Postfix notation within a closure.

### Some common closures

- Closure to add all cells in range
  
  ```bash
  sum define auto!reg!1;each!reg!0!~op ~cell +;
  ```

  This uses `register 0` to add the numbers in range and `auto` places the output in the cell next to the last cell.

  **Input:**

  ```bash
  sum define auto!reg!1;each!reg!0!~op ~cell +;
  t create with title "weights"
  t headers "fruit name" "weight (kg)"
  t add_row "apple" "15"
  t add_row "mango" "20"
  t add_row "papaya" "12.4"
  t add_row "total"
  t apply sum 0 1 2 1
  t view
  ```

  **Output:**

  ```csv
  weights
  fruit name,weight (kg),
  apple,15,
  mango,20,
  papaya,12.4,
  total,47.4,
  ```

- A count closure can also be defined in a similar way, the only change is for each cell in range, rather than adding the contents of `Cell` in our register, we simply add 1 to it.

  ```bash
  count define auto!reg!1;each!reg!0!~op 1 +;
  ```

- Closure to increment a single cell

  ```bash
  increment define ~cell.x,~cell.y!reg!1;each!reg!0!~cell 1 +;
  ```

  This closure can be applied to a single cell to increment its value by 1.

  *Note:* In the current implementation, atleast 1 register is required to be declared whether it is used or not. This behaviour will be updated in future version to define register-less closures.

---

## License & Copyright

- This Project is [Apache-2.0](./LICENSE) Licensed
- Copyright 2023 [Vishal Das](https://github.com/dvishal485)

---

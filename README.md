# spread-lang-backend

A dummy language interpreter to generate spreadsheet files (csv format).

---
## Features

- Data arranged in rows and columns
- Dummy programming language **(work in progress)**
- Auto correct support for basic commands
- Compile code to csv
- Interpreter (repl)
- Support of closures **(alpha stage)**

---
## Language Syntax

- `table_var create` will create a table and assign it to variable `table_var`.
- Title and headers (title of columns) can be set while creating the table as:

  ```bash
  table_name create with title "table name" and headers "header 1" "header 2"
  ```
- Headers can be set using `table_var headers "header 1" "header 2"`.
- Row can be added with `table_var add_row "content 1" "1"`.
- To view a table, use `table_var view`.
- Closures can be defined using `closure_var define [closure]`.
- Closures can be applied as: 
  
  ```bash
  table_var apply closure_var start_i start_j end_i end_j
  ```

  where (start_i, start_j) are 0-based indices of the starting cell and (end_i, end_j) are 0-based indices of ending cell on which closure is to be applied.

---
## Closure

***Note:** Closures are currently highly experimental feature.*

### What is closure?

- Closure act as a function to be applied on a range of cells.
- Closure can be used to apply certain formulas/computation on the cells.
- Closure is essentially a command written considering a strict set of rules.
- Unlike other commands, closure can't be verified for correctness. Incorrect closure is possible to be declared but they can't be applied (will throw error at runtime).

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

  *Note:* I dunno why this requires register to be defined and used. They are not really required, but my code simply doesn't work without registers. I will probably fix this in near future.

---
## License & Copyright

- This Project is [Apache-2.0](./LICENSE) Licensed
- Copyright 2023 [Vishal Das](https://github.com/dvishal485)

---

# rust-lispy

A super minimal lisp to learn rust and learn how LLVM works. It's just a pet project, don't use this in
production deployments

## Usage

This program is used to tokenize, parse and compile a very minimal lisp into super optimized bytecode. The spec we use
is [super simple](lisp-spec). If you have a file with lisp code following that spec (see [`examples/`](examples/)) you
can run some of the following commands:

#### `help`

Run `cargo run help` or `cargo run` without any commands to get help text:

```sh
$ cargo run help
lispy 1.0
ocamlmycaml
Runs a limited subset of clojure

USAGE:
    rust-lispy <INPUT> <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <INPUT>    Sets the input file to use

SUBCOMMANDS:
    help        Prints this message or the help of the given subcommand(s)
    parse       Parse the file and print out the ASTs
    tokenize    Tokenize the file and print out the tokens
```

#### `tokenize`

Run the tokenizer on a file:
```sh
$ cargo run examples/print_sum.clj tokenize
OpenParen[line 1 char 0]
        Identifier("println")[line 1 char 1 -> line 1 char 7]
        OpenParen[line 1 char 9]
                Identifier("+")[line 1 char 10]
                Number(1.0)[line 1 char 12]
                Number(2.0)[line 1 char 14]
        CloseParen[line 1 char 15]
CloseParen[line 1 char 16]
```

#### `parse`

Run the tokenizer on a file:
```sh
$ cargo run examples/print_sum.clj parse
EvaluateExpr { callee: "println", args: [EvaluateExpr { callee: "+", args: [NumberExpr(1.0), NumberExpr(2.0)] }] }
```

#### `llvm-generate`

WIP

#### `compile`

WIP

### lisp spec

Like all lisps, we'll be using brackets to separate statements and nest statements within one another.
Each statement can contain a few primitive language features:
  * `def` - define a variable: 2 args, name and another statement or value
  * `fn` - declare a function prototype (use it with `def`): 2 args, list of arg names, function body as a list of statements
  * `if` - do some branching logic: 3 args, condition statement, true-branch list of statements, false-branch list of
      statements (WIP)

These features will work on a few primitives we support:
  * `Identifier`: a name which is simply a sequence of characters not wrapped in quotes
  * `Number (f64)`: numerical values
  * `StringLiteral`: string of characters wrapped in quotes (WIP)

If the above spec doesn't make sense to you, well that's ok. It makes sense to me the author, the grand master, the head
wizard. And that's all that matters. You can write some lisp code into files, and read those files using the commands
above.

## Development

Set up rust by following the instructions here: https://rustup.rs/. This repo uses stable rust.

## Running tests

We have unit tests that can be executing using `cargo`
```sh
cargo test
```


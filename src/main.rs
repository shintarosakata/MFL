mod compiler;
mod lexer;
mod parser;

use compiler::*;
use lexer::*;
use parser::*;

use inkwell::context::Context;
use inkwell::passes::PassManager;
use inkwell::OptimizationLevel;

use std::collections::HashMap;
use std::io::{self, Write};

use std::fs::File;
use std::io::prelude::*;

// 新しい行を出力せずにprintとflushに使用されるマクロ
macro_rules! print_flush {
    ( $( $x:expr ),* ) => {
        print!( $($x, )* );

        std::io::stdout().flush().expect("Could not flush to standard output.");
    };
}

#[no_mangle]
pub extern "C" fn putchard(x: f64) -> f64 {
    print_flush!("{}", x as u8 as char);
    x
}

#[no_mangle]
pub extern "C" fn printd(x: f64) -> f64 {
    println!("{}", x);
    x
}

/// Rustコンパイラに削除されないよう、上記の関数をグローバル配列に追加する。
#[used]
static EXTERNAL_FNS: [extern "C" fn(f64) -> f64; 2] = [putchard, printd];

/// Replのエントリーポイント
fn main() {
    let mut repl = false;
    for arg in std::env::args() {
        match arg.as_str() {
            "-a" => repl = true,
            _ => (),
        }
    }

    if repl {
        run_repl();
    } else {
        compile();
    }
}

fn compile() {
    let context = Context::create();
    let module = context.create_module("repl");
    let builder = context.create_builder();

    // Create FPM
    let fpm = PassManager::create(&module);

    fpm.add_instruction_combining_pass();
    fpm.add_reassociate_pass();
    fpm.add_gvn_pass();
    fpm.add_cfg_simplification_pass();
    fpm.add_basic_alias_analysis_pass();
    fpm.add_promote_memory_to_register_pass();
    fpm.add_instruction_combining_pass();
    fpm.add_reassociate_pass();

    fpm.initialize();

    // ファイルが見つかりませんでした
    let mut f = File::open("input.ks").expect("file not found");

    let mut input = String::new();
    f.read_to_string(&mut input)
        // ファイルの読み込み中に問題がありました
        .expect("something went wrong reading the file");

    // 優先順位mapの生成
    let mut prec = HashMap::with_capacity(6);

    prec.insert('=', 2);
    prec.insert('<', 10);
    prec.insert('+', 20);
    prec.insert('-', 20);
    prec.insert('*', 40);
    prec.insert('/', 40);

    // make module
    let module = context.create_module("main");

    match Parser::new(input, &mut prec).parse() {
        Ok(fun) => {
            Compiler::compile(&context, &builder, &fpm, &module, &fun).unwrap();
        }
        Err(err) => {
            println!("!> Error parsing expression: {}", err);
        }
    };
    module.print_to_file("main.ll").unwrap();
}

fn run_repl() {
    // use self::inkwell::support::add_symbol;
    let mut display_lexer_output = false;
    let mut display_parser_output = false;
    let mut display_compiler_output = false;

    for arg in std::env::args() {
        match arg.as_str() {
            "--dl" => display_lexer_output = true,
            "--dp" => display_parser_output = true,
            "--dc" => display_compiler_output = true,
            _ => (),
        }
    }

    let context = Context::create();
    let module = context.create_module("repl");
    let builder = context.create_builder();

    // Create FPM
    let fpm = PassManager::create(&module);

    fpm.add_instruction_combining_pass();
    fpm.add_reassociate_pass();
    fpm.add_gvn_pass();
    fpm.add_cfg_simplification_pass();
    fpm.add_basic_alias_analysis_pass();
    fpm.add_promote_memory_to_register_pass();
    fpm.add_instruction_combining_pass();
    fpm.add_reassociate_pass();

    fpm.initialize();

    let mut previous_exprs = Vec::new();

    loop {
        println!();
        print_flush!("?> ");

        // Read input from stdin
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Could not read from standard input.");

        if input.starts_with("exit") || input.starts_with("quit") {
            break;
        } else if input.chars().all(char::is_whitespace) {
            continue;
        }

        // 優先順位mapの生成
        let mut prec = HashMap::with_capacity(6);

        prec.insert('=', 2);
        prec.insert('<', 10);
        prec.insert('+', 20);
        prec.insert('-', 20);
        prec.insert('*', 40);
        prec.insert('/', 40);

        // 入力の解析および表示(optionall)
        if display_lexer_output {
            println!(
                "-> Attempting to parse lexed input: \n{:?}\n",
                Lexer::new(input.as_str()).collect::<Vec<Token>>()
            );
        }

        // make module
        let module = context.create_module("tmp");

        // 以前に解析された全ての関数を新しいモジュールに再コンパイル
        for prev in &previous_exprs {
            Compiler::compile(&context, &builder, &fpm, &module, prev)
                .expect("Cannot re-add previously compiled function.");
        }

        let (name, is_anonymous) = match Parser::new(input, &mut prec).parse() {
            Ok(fun) => {
                let is_anon = fun.is_anon;

                if display_parser_output {
                    if is_anon {
                        println!("-> Expression parsed: \n{:?}\n", fun.body);
                    } else {
                        println!("-> Function parsed: \n{:?}\n", fun);
                    }
                }

                match Compiler::compile(&context, &builder, &fpm, &module, &fun) {
                    Ok(function) => {
                        if display_compiler_output {
                            // Not printing a new line since LLVM automatically
                            // prefixes the generated string with one
                            print_flush!("-> Expression compiled to IR:");
                            function.print_to_stderr();
                        }

                        if !is_anon {
                            // only add it now to ensure it is correct
                            previous_exprs.push(fun);
                        }

                        (function.get_name().to_str().unwrap().to_string(), is_anon)
                    }
                    Err(err) => {
                        println!("!> Error compiling function: {}", err);
                        continue;
                    }
                }
            }
            Err(err) => {
                println!("!> Error parsing expression: {}", err);
                continue;
            }
        };

        if is_anonymous {
            let ee = module
                .create_jit_execution_engine(OptimizationLevel::None)
                .unwrap();

            let maybe_fn =
                unsafe { ee.get_function::<unsafe extern "C" fn() -> f64>(name.as_str()) };
            let compiled_fn = match maybe_fn {
                Ok(f) => f,
                Err(err) => {
                    println!("!> Error during execution: {:?}", err);
                    continue;
                }
            };

            unsafe {
                println!("=> {}", compiled_fn.call());
            }
        }
    }
}

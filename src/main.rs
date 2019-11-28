mod lexer;
mod parser;

use parser::*;
use std::collections::HashMap;

fn main() {
    let input = String::from("def id(n) -n \n");
    let mut prec = HashMap::with_capacity(6);
    prec.insert('=', 2);
    prec.insert('<', 10);
    prec.insert('+', 20);
    prec.insert('-', 20);
    prec.insert('*', 40);
    prec.insert('/', 40);

    let mut parser = Parser::new(input, &mut prec);

    let parsed = parser.parse();

    println!("{:?}", parsed)
}

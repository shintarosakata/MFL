mod lexer;
use lexer::*;

fn main() {
    let mut lexer = Lexer::new("var world\n");

    lexer.for_each(|item| println!("{:?}", item));
}

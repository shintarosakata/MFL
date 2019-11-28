use std::iter::Peekable;
use std::ops::DerefMut;
use std::str::Chars;
use Token::*;

/// プリミティブな構文トークン
#[derive(Debug, Clone)]
pub enum Token {
    Binary,
    Comma,
    Comment,
    Def,
    Else,
    EOF,
    Extern,
    For,
    Ident(String),
    If,
    In,
    LParen,
    Number(f64),
    Op(char),
    RParen,
    Then,
    Unary,
    Var,
}

/// Lexerで発生したエラーを定義
#[derive(Debug)]
pub struct LexError {
    pub error: &'static str,
    pub index: usize,
}

/// Lexerで発生したエラーを生成
impl LexError {
    #[allow(dead_code)]
    pub fn new(msg: &'static str) -> LexError {
        LexError {
            error: msg,
            index: 0,
        }
    }

    #[allow(dead_code)]
    pub fn with_index(msg: &'static str, index: usize) -> LexError {
        LexError {
            error: msg,
            index: index,
        }
    }
}

/// 字句解析結果を定義
/// 成功した場合はトークン、失敗した場合はLexErrorとなります
pub type LexResult = Result<Token, LexError>;

/// Stringの入力を変換するレクサーの定義
/// Peekableはpeek()メソッドを利用することにより、中身を確認することができる
#[derive(Debug)]
pub struct Lexer<'a> {
    input: &'a str,
    chars: Box<Peekable<Chars<'a>>>,
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// ソースコード'input'を引数として、新たな字句解析機を作成
    pub fn new(input: &'a str) -> Lexer<'a> {
        Lexer {
            input: input,
            chars: Box::new(input.chars().peekable()),
            pos: 0,
        }
    }

    /// ソースコードから次のトークンを実行して返す
    pub fn lex(&mut self) -> LexResult {
        let chars = self.chars.deref_mut();
        let src = self.input;

        let mut pos = self.pos;

        // 空白スキップとEOF判断
        loop {
            // Note:
            // 次の行は独自のスコープ内となっている。
            // charsの借用期間を制限して許可するために
            // ループ内でchar.next()によって再度借用される。
            {
                let ch = chars.peek();

                // EOFチェック
                if ch.is_none() {
                    self.pos = pos;

                    return Ok(EOF);
                }

                // 真上でnoneチェックを行っているため、unwrapを使っても安全性が保たれている
                if !ch.unwrap().is_whitespace() {
                    break;
                }
            }

            chars.next();
            pos += 1;
        }

        let start = pos;
        let next = chars.next();

        if next.is_none() {
            return Ok(EOF);
        }

        pos += 1;

        // 実際にNextTokenの取得をする
        let result = match next.unwrap() {
            '(' => Ok(LParen),
            ')' => Ok(RParen),
            ',' => Ok(Comma),

            '#' => {
                // 改行まで取得せずにloopする
                loop {
                    let ch = chars.next();
                    pos += 1;

                    if ch == Some('\n') {
                        break;
                    }
                }

                Ok(Comment)
            }

            '.' | '0'..='9' => {
                // Numberリテラルのパース
                loop {
                    let ch = match chars.peek() {
                        Some(ch) => *ch,
                        None => return Ok(EOF),
                    };

                    // Parse float.
                    if ch != '.' && !ch.is_digit(16) {
                        break;
                    }

                    chars.next();
                    pos += 1;
                }

                Ok(Number(src[start..pos].parse().unwrap()))
            }

            'a'..='z' | 'A'..='Z' | '_' => {
                // 識別子のパース
                loop {
                    let ch = match chars.peek() {
                        Some(ch) => *ch,
                        None => return Ok(EOF),
                    };

                    // 識別子の2文字目以降はアンダースコアか数字のみである
                    if ch != '_' && !ch.is_alphanumeric() {
                        break;
                    }

                    chars.next();
                    pos += 1;
                }

                match &src[start..pos] {
                    // 予約後として認識
                    "def" => Ok(Def),
                    "extern" => Ok(Extern),
                    "if" => Ok(If),
                    "then" => Ok(Then),
                    "else" => Ok(Else),
                    "for" => Ok(For),
                    "in" => Ok(In),
                    "unary" => Ok(Unary),
                    "binary" => Ok(Binary),
                    "var" => Ok(Var),
                    // 予約後ではない場合はユーザー定義識別子として認識
                    ident => Ok(Ident(ident.to_string())),
                }
            }

            // その他は全てオペレータとして認識
            op => {
                // Parse operator
                Ok(Op(op))
            }
        };

        // positionを現在地に進めて保存し、終了
        self.pos = pos;

        result
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        match self.lex() {
            Ok(EOF) | Err(_) => None,
            Ok(token) => Some(token),
        }
    }
}

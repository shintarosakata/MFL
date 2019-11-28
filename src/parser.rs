use crate::lexer::*;
use std::collections::HashMap;
use Token::*;

const ANONYMOUS_FUNCTION_NAME: &str = "anonymous";

/// プリミティブ式の定義
#[derive(Debug)]
pub enum Expr {
    Binary {
        op: char,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    Call {
        fn_name: String,
        args: Vec<Expr>,
    },

    Conditional {
        cond: Box<Expr>,
        consequence: Box<Expr>,
        alternative: Box<Expr>,
    },

    For {
        var_name: String,
        start: Box<Expr>,
        end: Box<Expr>,
        step: Option<Box<Expr>>,
        body: Box<Expr>,
    },

    Number(f64),

    Variable(String),

    VarIn {
        variables: Vec<(String, Option<Expr>)>,
        body: Box<Expr>,
    },
}

/// 関数のプロトタイプ(名前とパラメータ)を定義
#[derive(Debug)]
pub struct Prototype {
    pub name: String,
    pub args: Vec<String>,
    pub is_op: bool,
    pub prec: usize,
}

/// ユーザー定義、または外部関数の定義
#[derive(Debug)]
pub struct Function {
    pub prototype: Prototype,
    pub body: Option<Expr>,
    pub is_anon: bool,
}

/// 式パーサーを表す
#[derive(Debug)]
pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    prec: &'a mut HashMap<char, i32>,
}

// チェックせずにself.advanceを呼び出すためにlintを無視
// EOFが許容される場合の結果
#[allow(unused_must_use)]
impl<'a> Parser<'a> {
    /// 入力とHashMapを指定して新しいパーサーを作成する
    /// HashMapはバイナリ式の演算子と優先度
    pub fn new(input: String, op_precedence: &'a mut HashMap<char, i32>) -> Self {
        let mut lexer = Lexer::new(input.as_str());
        let tokens = lexer.by_ref().collect();

        Parser {
            tokens: tokens,
            prec: op_precedence,
            pos: 0,
        }
    }

    /// パーサーの中身を解析
    pub fn parse(&mut self) -> Result<Function, &'static str> {
        let result = match self.current()? {
            Def => self.parse_def(),
            Extern => self.parse_extern(),
            _ => self.parse_toplevel_expr(),
        };

        match result {
            Ok(result) => {
                if !self.at_end() {
                    Err("Unexpected token after parsed expression.")
                } else {
                    Ok(result)
                }
            }

            err => err,
        }
    }

    /// セーフチェックをせずに現在のトークンを返す
    fn curr(&self) -> Token {
        self.tokens[self.pos].clone()
    }

    /// セーフチェックをして現在のトークン、またはエラーを返す
    /// エラーの場合はファイルの終わりに予期せずに到達したことを示す
    fn current(&self) -> Result<Token, &'static str> {
        if self.pos >= self.tokens.len() {
            Err("Unexpected end of file.")
        } else {
            Ok(self.tokens[self.pos].clone())
        }
    }

    /// ポジションを進めて、エラーか空の成功をもつ結果を返す
    /// これにより、'?'構文を使用できる
    /// エラーの場合はファイルの終わりに予期せずに到達したことを示す
    fn advance(&mut self) -> Result<(), &'static str> {
        let npos = self.pos + 1;

        self.pos = npos;

        if npos < self.tokens.len() {
            Ok(())
        } else {
            Err("Unexpected end of file.")
        }
    }

    /// 入力の終わりに達したかどうかを返す
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// 現在のトークンの優先度を返す
    /// バイナリ演算子でない場合は-1
    fn get_tok_precedence(&self) -> i32 {
        if let Ok(Op(op)) = self.current() {
            *self.prec.get(&op).unwrap_or(&100)
        } else {
            -1
        }
    }

    /// 外部、ユーザー定義に関係なく、関数のプロトタイプを解析
    fn parse_prototype(&mut self) -> Result<Prototype, &'static str> {
        let (id, is_operator, precedence) = match self.curr() {
            Ident(id) => {
                self.advance()?;

                (id, false, 0)
            }

            Binary => {
                self.advance()?;

                let op = match self.curr() {
                    Op(ch) => ch,
                    _ => return Err("Expected operator in custom operator declaration."),
                };

                self.advance()?;

                let mut name = String::from("binary");

                name.push(op);

                let prec = if let Number(prec) = self.curr() {
                    self.advance()?;

                    prec as usize
                } else {
                    0
                };

                self.prec.insert(op, prec as i32);

                (name, true, prec)
            }

            Unary => {
                self.advance()?;

                let op = match self.curr() {
                    Op(ch) => ch,
                    _ => return Err("Expected operator in custom operator declaration."),
                };

                let mut name = String::from("unary");

                name.push(op);

                self.advance()?;

                (name, true, 0)
            }

            _ => return Err("Expected identifier in prototype declaration."),
        };

        match self.curr() {
            LParen => (),
            _ => return Err("Expected '(' character in prototype declaration."),
        }

        self.advance()?;

        if let RParen = self.curr() {
            self.advance();

            return Ok(Prototype {
                name: id,
                args: vec![],
                is_op: is_operator,
                prec: precedence,
            });
        }

        let mut args = vec![];

        // パラメータ宣言
        loop {
            match self.curr() {
                Ident(name) => args.push(name),
                _ => return Err("Expected identifier in parameter declaration."),
            }

            self.advance()?;

            match self.curr() {
                RParen => {
                    self.advance();
                    break;
                }
                // コンマが続く場合はさらに引数を取る
                Comma => {
                    self.advance();
                }
                _ => return Err("Expected ',' or ')' character in prototype declaration."),
            }
        }

        Ok(Prototype {
            name: id,
            args: args,
            is_op: is_operator,
            prec: precedence,
        })
    }

    /// ユーザー定義関数を解析
    fn parse_def(&mut self) -> Result<Function, &'static str> {
        // 最初の"Def"キーワードは解析せずにすすむ
        self.pos += 1;

        // 関数のシグネチャを解析
        let proto = self.parse_prototype()?;

        // 関数のボディを解析
        let body = self.parse_expr()?;

        Ok(Function {
            prototype: proto,
            body: Some(body),
            is_anon: false,
        })
    }

    /// 外部宣言関数の解析
    fn parse_extern(&mut self) -> Result<Function, &'static str> {
        // 最初の"Def"キーワードは解析せずにすすむ
        self.pos += 1;

        // 関数のシグネチャを解析
        let proto = self.parse_prototype()?;

        Ok(Function {
            prototype: proto,
            body: None,
            is_anon: false,
        })
    }

    /// 式の解析
    fn parse_expr(&mut self) -> Result<Expr, &'static str> {
        match self.parse_unary_expr() {
            Ok(left) => self.parse_binary_expr(0, left),
            err => err,
        }
    }

    /// リテラルナンバーの式の解析
    fn parse_nb_expr(&mut self) -> Result<Expr, &'static str> {
        // NumberをExpr::Numberに変換する
        match self.curr() {
            Number(nb) => {
                self.advance();
                Ok(Expr::Number(nb))
            }
            _ => Err("Expected number literal."),
        }
    }

    /// parenで囲まれた式の解析
    fn parse_paren_expr(&mut self) -> Result<Expr, &'static str> {
        match self.current()? {
            LParen => (),
            _ => return Err("Expected '(' character at start of parenthesized expression."),
        }

        self.advance()?;

        let expr = self.parse_expr()?;

        match self.current()? {
            RParen => (),
            _ => return Err("Expected ')' character at end of parenthesized expression."),
        }

        self.advance();

        Ok(expr)
    }

    /// 識別子(変数か関数呼び出し)で始まる式の解析
    fn parse_id_expr(&mut self) -> Result<Expr, &'static str> {
        let id = match self.curr() {
            Ident(id) => id,
            _ => return Err("Expected identifier."),
        };

        // 後に続くものがなかった場合は変数
        if self.advance().is_err() {
            return Ok(Expr::Variable(id));
        }

        // それ以外は関数のため、LParenが続く
        match self.curr() {
            LParen => {
                self.advance()?;

                // 引数なし
                if let RParen = self.curr() {
                    return Ok(Expr::Call {
                        fn_name: id,
                        args: vec![],
                    });
                }

                // RParenが続かない場合は引数を確保していく
                let mut args = vec![];

                loop {
                    args.push(self.parse_expr()?);

                    // カンマかRParenが期待される
                    match self.current()? {
                        Comma => (),
                        RParen => break,
                        _ => return Err("Expected ',' character in function call."),
                    }

                    self.advance()?;
                }

                self.advance();

                Ok(Expr::Call {
                    fn_name: id,
                    args: args,
                })
            }

            _ => Ok(Expr::Variable(id)),
        }
    }

    /// 単項式の解析
    fn parse_unary_expr(&mut self) -> Result<Expr, &'static str> {
        let op = match self.current()? {
            Op(ch) => {
                self.advance()?;
                ch
            }
            _ => return self.parse_primary(),
        };

        let mut name = String::from("unary");

        name.push(op);

        Ok(Expr::Call {
            fn_name: name,
            args: vec![self.parse_unary_expr()?],
        })
    }

    /// 左の式を指定して、バイナリ式を解析
    fn parse_binary_expr(&mut self, prec: i32, mut left: Expr) -> Result<Expr, &'static str> {
        loop {
            let curr_prec = self.get_tok_precedence();

            if curr_prec < prec || self.at_end() {
                return Ok(left);
            }

            let op = match self.curr() {
                Op(op) => op,
                _ => return Err("Invalid operator."),
            };

            self.advance()?;

            let mut right = self.parse_unary_expr()?;

            let next_prec = self.get_tok_precedence();

            if curr_prec < next_prec {
                right = self.parse_binary_expr(curr_prec + 1, right)?;
            }

            left = Expr::Binary {
                op: op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
    }

    /// conditional if..then..else式を解析
    fn parse_conditional_expr(&mut self) -> Result<Expr, &'static str> {
        // eat 'if' token
        self.advance()?;

        let cond = self.parse_expr()?;

        // eat 'then' token
        match self.current() {
            Ok(Then) => self.advance()?,
            _ => return Err("Expected 'then' keyword."),
        }

        let then_result = self.parse_expr()?;

        // eat 'else' token
        match self.current() {
            Ok(Else) => self.advance()?,
            _ => return Err("Expected 'else' keyword."),
        }

        let else_result = self.parse_expr()?;

        Ok(Expr::Conditional {
            cond: Box::new(cond),
            consequence: Box::new(then_result),
            alternative: Box::new(else_result),
        })
    }

    /// forループ式の解析
    fn parse_for_expr(&mut self) -> Result<Expr, &'static str> {
        // eat 'for' token
        self.advance()?;

        let name = match self.curr() {
            Ident(n) => n,
            _ => return Err("Expected identifier in for loop."),
        };

        // eat identifier
        self.advance()?;

        // eat '=' token
        match self.curr() {
            Op('=') => self.advance()?,
            _ => return Err("Expected '=' character in for loop."),
        }

        let start = self.parse_expr()?;

        // eat ',' token
        match self.current()? {
            Comma => self.advance()?,
            _ => return Err("Expected ',' character in for loop."),
        }

        let end = self.parse_expr()?;

        // parse (optional) step expression
        let step = match self.current()? {
            Comma => {
                self.advance()?;

                Some(self.parse_expr()?)
            }

            _ => None,
        };

        // eat 'in' token
        match self.current()? {
            In => self.advance()?,
            _ => return Err("Expected 'in' keyword in for loop."),
        }

        let body = self.parse_expr()?;

        Ok(Expr::For {
            var_name: name,
            start: Box::new(start),
            end: Box::new(end),
            step: step.map(Box::new),
            body: Box::new(body),
        })
    }

    /// var..in式の解析
    fn parse_var_expr(&mut self) -> Result<Expr, &'static str> {
        // eat 'var' token
        self.advance()?;

        let mut variables = Vec::new();

        // parse variables
        loop {
            let name = match self.curr() {
                Ident(name) => name,
                _ => return Err("Expected identifier in 'var..in' declaration."),
            };

            self.advance()?;

            // read (optional) initializer
            let initializer = match self.curr() {
                Op('=') => Some({
                    self.advance()?;
                    self.parse_expr()?
                }),

                _ => None,
            };

            variables.push((name, initializer));

            match self.curr() {
                Comma => {
                    self.advance()?;
                }
                In => {
                    self.advance()?;
                    break;
                }
                _ => return Err("Expected comma or 'in' keyword in variable declaration."),
            }
        }

        // parse body
        let body = self.parse_expr()?;

        Ok(Expr::VarIn {
            variables: variables,
            body: Box::new(body),
        })
    }

    /// プライマリ式(識別子、数値、またはカッコで囲まれた式)の解析
    fn parse_primary(&mut self) -> Result<Expr, &'static str> {
        match self.curr() {
            Ident(_) => self.parse_id_expr(),
            Number(_) => self.parse_nb_expr(),
            LParen => self.parse_paren_expr(),
            If => self.parse_conditional_expr(),
            For => self.parse_for_expr(),
            Var => self.parse_var_expr(),
            _ => Err("Unknown expression."),
        }
    }

    /// トップレベルの式を解析し、匿名関数を作成する。
    /// コンパイルを容易にするために存在する
    fn parse_toplevel_expr(&mut self) -> Result<Function, &'static str> {
        match self.parse_expr() {
            Ok(expr) => Ok(Function {
                prototype: Prototype {
                    name: ANONYMOUS_FUNCTION_NAME.to_string(),
                    args: vec![],
                    is_op: false,
                    prec: 0,
                },
                body: Some(expr),
                is_anon: true,
            }),

            Err(err) => Err(err),
        }
    }
}

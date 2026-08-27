use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(toks: Vec<Token>) -> Self {
        Self { toks, pos: 0 }
    }

    fn peek(&self) -> &Token { &self.toks[self.pos] }
    fn advance(&mut self) -> Token { let t = self.toks[self.pos].clone(); self.pos += 1; t }

    fn expect(&mut self, t: Token) {
        if *self.peek() != t {
            panic!("Parse error: expected {:?}, got {:?} at pos {}", t, self.peek(), self.pos);
        }
        self.advance();
    }

    fn ident(&mut self) -> String {
        match self.advance() {
            Token::Ident(s) => s,
            t => panic!("Expected identifier, got {:?}", t),
        }
    }

    pub fn parse_module(&mut self) -> Module {
        let mut items = Vec::new();
        while *self.peek() != Token::Eof {
            items.push(self.parse_item());
        }
        Module { items }
    }

    fn parse_item(&mut self) -> Item {
        match self.peek() {
            Token::KwImport => Item::Import(self.parse_import()),
            Token::KwClass => Item::Class(self.parse_class()),
            Token::KwFn => Item::Fn(self.parse_fn()),
            t => panic!("Unexpected top-level token: {:?}", t),
        }
    }

    /// import a.b.c
    /// import a.b.c as x
    fn parse_import(&mut self) -> ImportDecl {
        self.expect(Token::KwImport);
        let mut path = vec![self.ident()];
        while *self.peek() == Token::Dot {
            self.advance();
            path.push(self.ident());
        }
        let alias = if *self.peek() == Token::KwAs {
            self.advance();
            Some(self.ident())
        } else {
            None
        };
        self.expect(Token::Semi);
        ImportDecl { path, alias }
    }

    /// class Dog extends Animal { field bark_count; fn bark() { ... } }
    fn parse_class(&mut self) -> ClassDecl {
        self.expect(Token::KwClass);
        let name = self.ident();
        let parent = if *self.peek() == Token::KwExtends {
            self.advance();
            Some(self.ident())
        } else {
            None
        };
        self.expect(Token::LBrace);

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while *self.peek() != Token::RBrace {
            match self.peek() {
                Token::KwFn => methods.push(self.parse_fn()),
                Token::Ident(_) => {
                    // field declaration: "name;"
                    let fname = self.ident();
                    self.expect(Token::Semi);
                    fields.push(fname);
                }
                t => panic!("Unexpected token in class body: {:?}", t),
            }
        }
        self.expect(Token::RBrace);

        ClassDecl { name, parent, fields, methods }
    }

    fn parse_fn(&mut self) -> FnDecl {
        self.expect(Token::KwFn);
        let name = self.ident();
        self.expect(Token::LParen);
        let mut params = Vec::new();
        while *self.peek() != Token::RParen {
            params.push(self.ident());
            if *self.peek() == Token::Comma { self.advance(); }
        }
        self.expect(Token::RParen);
        self.expect(Token::LBrace);
        let body = self.parse_block();
        self.expect(Token::RBrace);
        FnDecl { name, params, body }
    }

    fn parse_block(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while *self.peek() != Token::RBrace {
            stmts.push(self.parse_stmt());
        }
        stmts
    }

    fn parse_stmt(&mut self) -> Stmt {
        match self.peek() {
            Token::KwLet => {
                self.advance();
                let name = self.ident();
                self.expect(Token::Eq);
                let val = self.parse_expr();
                self.expect(Token::Semi);
                Stmt::Let(name, val)
            }
            Token::KwIf => {
                self.advance();
                self.expect(Token::LParen);
                let cond = self.parse_expr();
                self.expect(Token::RParen);
                self.expect(Token::LBrace);
                let then_b = self.parse_block();
                self.expect(Token::RBrace);
                let else_b = if *self.peek() == Token::KwElse {
                    self.advance();
                    self.expect(Token::LBrace);
                    let b = self.parse_block();
                    self.expect(Token::RBrace);
                    b
                } else {
                    Vec::new()
                };
                Stmt::If(cond, then_b, else_b)
            }
            Token::KwWhile => {
                self.advance();
                self.expect(Token::LParen);
                let cond = self.parse_expr();
                self.expect(Token::RParen);
                self.expect(Token::LBrace);
                let body = self.parse_block();
                self.expect(Token::RBrace);
                Stmt::While(cond, body)
            }
            Token::KwReturn => {
                self.advance();
                if *self.peek() == Token::Semi {
                    self.advance();
                    Stmt::Return(None)
                } else {
                    let e = self.parse_expr();
                    self.expect(Token::Semi);
                    Stmt::Return(Some(e))
                }
            }
            Token::Ident(_) => {
                let save = self.pos;
                let name = self.ident();
                if *self.peek() == Token::Eq {
                    self.advance();
                    let val = self.parse_expr();
                    self.expect(Token::Semi);
                    Stmt::Assign(name, val)
                } else {
                    self.pos = save;
                    let e = self.parse_expr();
                    self.expect(Token::Semi);
                    Stmt::ExprStmt(e)
                }
            }
            t => panic!("Unexpected statement token: {:?}", t),
        }
    }

    fn parse_expr(&mut self) -> Expr { self.parse_cmp() }

    fn parse_cmp(&mut self) -> Expr {
        let mut lhs = self.parse_add();
        loop {
            let op = match self.peek() {
                Token::Lt => BinOpKind::Lt,
                Token::Gt => BinOpKind::Gt,
                Token::EqEq => BinOpKind::Eq,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_add();
            lhs = Expr::BinOp(Box::new(lhs), op, Box::new(rhs));
        }
        lhs
    }

    fn parse_add(&mut self) -> Expr {
        let mut lhs = self.parse_mul();
        loop {
            let op = match self.peek() {
                Token::Plus => BinOpKind::Add,
                Token::Minus => BinOpKind::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_mul();
            lhs = Expr::BinOp(Box::new(lhs), op, Box::new(rhs));
        }
        lhs
    }

    fn parse_mul(&mut self) -> Expr {
        let mut lhs = self.parse_postfix();
        loop {
            let op = match self.peek() {
                Token::Star => BinOpKind::Mul,
                Token::Slash => BinOpKind::Div,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_postfix();
            lhs = Expr::BinOp(Box::new(lhs), op, Box::new(rhs));
        }
        lhs
    }

    /// hậu tố: obj.field, obj.method(args), fn(args)
    fn parse_postfix(&mut self) -> Expr {
        let mut e = self.parse_primary();
        loop {
            match self.peek() {
                Token::Dot => {
                    self.advance();
                    let name = self.ident();
                    if *self.peek() == Token::LParen {
                        self.advance();
                        let args = self.parse_args();
                        e = Expr::MethodCall(Box::new(e), name, args);
                    } else {
                        e = Expr::FieldAccess(Box::new(e), name);
                    }
                }
                _ => break,
            }
        }
        e
    }

    fn parse_args(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();
        while *self.peek() != Token::RParen {
            args.push(self.parse_expr());
            if *self.peek() == Token::Comma { self.advance(); }
        }
        self.expect(Token::RParen);
        args
    }

    fn parse_primary(&mut self) -> Expr {
        match self.advance() {
            Token::Int(n) => Expr::IntLit(n),
            Token::Str(s) => Expr::StrLit(s),
            Token::KwTrue => Expr::IntLit(1),
            Token::KwFalse => Expr::IntLit(0),
            Token::Not => Expr::Not(Box::new(self.parse_postfix())),
            Token::Ident(name) => {
                if *self.peek() == Token::LParen {
                    self.advance();
                    let args = self.parse_args();
                    Expr::Call(name, args)
                } else {
                    Expr::Var(name)
                }
            }
            Token::LParen => {
                let e = self.parse_expr();
                self.expect(Token::RParen);
                e
            }
            t => panic!("Unexpected primary token: {:?}", t),
        }
    }
}
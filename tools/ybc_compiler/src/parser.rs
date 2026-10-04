use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(toks: Vec<Token>) -> Self { Self { toks, pos: 0 } }

    fn peek(&self) -> &Token {
        if self.pos < self.toks.len() { &self.toks[self.pos] } else { &Token::Eof }
    }

    fn advance(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        self.pos += 1;
        t
    }

    fn expect(&mut self, t: Token) {
        if *self.peek() != t {
            panic!("Parse error: expected {:?}, got {:?} at token #{}", t, self.peek(), self.pos);
        }
        self.advance();
    }

    fn ident(&mut self) -> String {
        match self.advance() {
            Token::Ident(s) => s,
            t => panic!("Expected identifier, got {:?} at token #{}", t, self.pos),
        }
    }

    pub fn parse_module(&mut self) -> Module {
        let package = if *self.peek() == Token::KwPackage { Some(self.parse_package()) } else { None };
        let mut items = Vec::new();
        while *self.peek() != Token::Eof {
            items.push(self.parse_item());
        }
        Module { package, items }
    }

    fn parse_package(&mut self) -> PackageDecl {
        self.expect(Token::KwPackage);
        let mut path = vec![self.ident()];
        while *self.peek() == Token::Dot {
            self.advance();
            path.push(self.ident());
        }
        self.expect(Token::Semi);
        PackageDecl { path }
    }

    fn parse_item(&mut self) -> Item {
        match self.peek() {
            Token::KwImport => Item::Import(self.parse_import()),
            Token::KwClass => Item::Class(self.parse_class()),
            Token::KwFn => Item::Fn(self.parse_fn()),
            t => panic!("Unexpected top-level token: {:?} at token #{}", t, self.pos),
        }
    }

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
        } else { None };
        self.expect(Token::Semi);
        ImportDecl { path, alias }
    }

    fn parse_class(&mut self) -> ClassDecl {
        self.expect(Token::KwClass);
        let name = self.ident();
        let parent = if *self.peek() == Token::KwExtends {
            self.advance();
            Some(self.ident())
        } else { None };
        self.expect(Token::LBrace);

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while *self.peek() != Token::RBrace {
            match self.peek() {
                Token::KwFn => methods.push(self.parse_fn()),
                Token::Ident(_) => {
                    fields.push(self.ident());
                    self.expect(Token::Semi);
                }
                t => panic!("Unexpected token in class body: {:?} at token #{}", t, self.pos),
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
        if *self.peek() != Token::RParen {
            loop {
                params.push(self.ident());
                if *self.peek() == Token::Comma {
                    self.advance();
                    if *self.peek() == Token::RParen { break; } // trailing comma
                } else {
                    break;
                }
            }
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
            Token::KwIf => self.parse_if(),
            Token::KwWhile => {
                self.advance();
                let cond = self.parse_expr();
                self.expect(Token::LBrace);
                let body = self.parse_block();
                self.expect(Token::RBrace);
                Stmt::While(cond, body)
            }
            Token::KwBreak => {
                self.advance();
                self.expect(Token::Semi);
                Stmt::Break
            }
            Token::KwContinue => {
                self.advance();
                self.expect(Token::Semi);
                Stmt::Continue
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
            _ => {
                let e = self.parse_expr();
                if *self.peek() == Token::Eq {
                    self.advance();
                    let val = self.parse_expr();
                    self.expect(Token::Semi);
                    match e {
                        Expr::Var(name) => Stmt::Assign(name, val),
                        Expr::IndexAccess(arr, idx) => Stmt::IndexAssign(*arr, *idx, val),
                        Expr::FieldAccess(obj, field) => Stmt::FieldAssign(*obj, field, val),
                        _ => panic!("Invalid assignment target: {:?}", e),
                    }
                } else {
                    self.expect(Token::Semi);
                    Stmt::ExprStmt(e)
                }
            }
        }
    }

    /// Parse `if (cond) { ... } [else if (cond) { ... }]* [else { ... }]`
    /// dưới dạng vòng lặp phẳng, không đệ quy — tránh mọi vấn đề về nested else-if.
    fn parse_if(&mut self) -> Stmt {
        self.expect(Token::KwIf);
        let cond = self.parse_expr();
        self.expect(Token::LBrace);
        let then_b = self.parse_block();
        self.expect(Token::RBrace);

        let mut else_b: Vec<Stmt> = Vec::new();

        if *self.peek() == Token::KwElse {
            self.advance();
            if *self.peek() == Token::KwIf {
                // Chuyển `else if` tiếp theo thành 1 Stmt::If rồi gán vào else_b.
                // Vòng while này làm phẳng chuỗi else-if dài thành 1 mảng.
                let inner = self.parse_if();
                else_b = vec![inner];
            } else {
                self.expect(Token::LBrace);
                else_b = self.parse_block();
                self.expect(Token::RBrace);
            }
        }

        Stmt::If(cond, then_b, else_b)
    }

    // ============================================
    // EXPRESSION PARSING
    // ============================================

    fn parse_expr(&mut self) -> Expr { self.parse_or() }

    fn parse_or(&mut self) -> Expr {
        let mut lhs = self.parse_and();
        while *self.peek() == Token::Or {
            self.advance();
            let rhs = self.parse_and();
            lhs = Expr::BinOp(Box::new(lhs), BinOpKind::Or, Box::new(rhs));
        }
        lhs
    }

    fn parse_and(&mut self) -> Expr {
        let mut lhs = self.parse_cmp();
        while *self.peek() == Token::And {
            self.advance();
            let rhs = self.parse_cmp();
            lhs = Expr::BinOp(Box::new(lhs), BinOpKind::And, Box::new(rhs));
        }
        lhs
    }

    fn parse_cmp(&mut self) -> Expr {
        let mut lhs = self.parse_add();
        loop {
            let op = match self.peek() {
                Token::Lt => BinOpKind::Lt,
                Token::Gt => BinOpKind::Gt,
                Token::EqEq => BinOpKind::Eq,
                Token::Neq => BinOpKind::Neq,
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
        let mut lhs = self.parse_unary();
        loop {
            let op = match self.peek() {
                Token::Star => BinOpKind::Mul,
                Token::Slash => BinOpKind::Div,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_unary();
            lhs = Expr::BinOp(Box::new(lhs), op, Box::new(rhs));
        }
        lhs
    }

    fn parse_unary(&mut self) -> Expr {
        match self.peek() {
            Token::Not => {
                self.advance();
                Expr::Not(Box::new(self.parse_unary()))
            }
            Token::Minus => {
                self.advance();
                let inner = self.parse_unary();
                Expr::BinOp(Box::new(Expr::IntLit(0)), BinOpKind::Sub, Box::new(inner))
            }
            _ => self.parse_postfix()
        }
    }

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
                Token::LBracket => {
                    self.advance();
                    let idx = self.parse_expr();
                    self.expect(Token::RBracket);
                    e = Expr::IndexAccess(Box::new(e), Box::new(idx));
                }
                _ => break,
            }
        }
        e
    }

    fn parse_args(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();
        if *self.peek() != Token::RParen {
            loop {
                args.push(self.parse_expr());
                if *self.peek() == Token::Comma {
                    self.advance();
                    if *self.peek() == Token::RParen { break; } // trailing comma OK
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RParen);
        args
    }

    fn parse_primary(&mut self) -> Expr {
        match self.peek() {
            Token::Int(_) => {
                let Token::Int(n) = self.advance() else { unreachable!() };
                Expr::IntLit(n)
            }
            Token::Str(_) => {
                let Token::Str(s) = self.advance() else { unreachable!() };
                Expr::StrLit(s)
            }
            Token::KwTrue => {
                self.advance();
                Expr::IntLit(1)
            }
            Token::KwFalse => {
                self.advance();
                Expr::IntLit(0)
            }
            Token::KwNew => {
                self.advance();
                let class_name = self.ident();
                self.expect(Token::LParen);
                let args = self.parse_args();
                Expr::MethodCall(Box::new(Expr::Var("new".to_string())), class_name, args)
            }
            Token::LBracket => {
                self.advance();
                let mut items = Vec::new();
                if *self.peek() != Token::RBracket {
                    loop {
                        items.push(self.parse_expr());
                        if *self.peek() == Token::Comma {
                            self.advance();
                            if *self.peek() == Token::RBracket { break; } // trailing comma OK
                        } else {
                            break;
                        }
                    }
                }
                self.expect(Token::RBracket);
                Expr::ArrayLit(items)
            }
            Token::LParen => {
                self.advance();
                let e = self.parse_expr();
                self.expect(Token::RParen);
                e
            }
            Token::Ident(_) => {
                let name = self.ident();
                if *self.peek() == Token::LParen {
                    self.advance();
                    let args = self.parse_args();
                    Expr::Call(name, args)
                } else {
                    Expr::Var(name)
                }
            }
            Token::Eq | Token::Comma | Token::Semi | Token::RParen | Token::RBracket | Token::Dot => {
                self.advance();
                self.parse_primary()
            }
            t => panic!("Unexpected primary token: {:?} at token #{}", t, self.pos),
        }
    }
}
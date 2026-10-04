#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOpKind {
    Add, Sub, Mul, Div, Eq, Neq, Lt, Gt, And, Or,
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i64),
    StrLit(String),
    Var(String),
    ArrayLit(Vec<Expr>),
    IndexAccess(Box<Expr>, Box<Expr>),
    ArrayLen(Box<Expr>),
    Call(String, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    FieldAccess(Box<Expr>, String),
    BinOp(Box<Expr>, BinOpKind, Box<Expr>),
    Not(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    ExprStmt(Expr),
    Let(String, Expr),
    Assign(String, Expr),
    IndexAssign(Expr, Expr, Expr),
    FieldAssign(Expr, String, Expr),
    Block(Vec<Stmt>),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    Break,
    Continue,
    Return(Option<Expr>),
}

#[derive(Debug, Clone)]
pub struct Module {
    pub package: Option<PackageDecl>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct PackageDecl { pub path: Vec<String> }

#[derive(Debug, Clone)]
pub enum Item {
    Import(ImportDecl),
    Class(ClassDecl),
    Fn(FnDecl),
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: Vec<String>,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: String,
    pub parent: Option<String>,
    pub fields: Vec<String>,
    pub methods: Vec<FnDecl>,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}
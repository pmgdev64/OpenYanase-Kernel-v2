// AST hỗ trợ class-children (class kế thừa) + import module ngoài

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i64),
    StrLit(String),
    Var(String),
    FieldAccess(Box<Expr>, String),      // obj.field
    MethodCall(Box<Expr>, String, Vec<Expr>), // obj.method(args)
    Call(String, Vec<Expr>),             // fn(args) hoặc syscall
    BinOp(Box<Expr>, BinOpKind, Box<Expr>),
    Not(Box<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOpKind { Add, Sub, Mul, Div, Lt, Gt, Eq }

#[derive(Debug, Clone)]
pub enum Stmt {
    ExprStmt(Expr),
    Let(String, Expr),
    Assign(String, Expr),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    Return(Option<Expr>),
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

/// class-children: 1 class có thể extends 1 class cha (đơn kế thừa),
/// field/method của cha được resolver gộp vào con nếu con không override
#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: String,
    pub parent: Option<String>,   // "class Dog extends Animal"
    pub fields: Vec<String>,
    pub methods: Vec<FnDecl>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Fn(FnDecl),
    Class(ClassDecl),
    Import(ImportDecl),
}

/// import module ngoài: "import utils.math as m" hoặc "import utils.math"
#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: Vec<String>,   // ["utils", "math"] -> tìm file utils/math.yl
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub items: Vec<Item>,
}
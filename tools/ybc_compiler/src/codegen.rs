use std::collections::HashMap;
use crate::ast::*;
use crate::resolver::ResolvedProgram;

const OP_NOP: u8 = 0;
const OP_PUSHINT: u8 = 1;
const OP_PUSHSTR: u8 = 2;
const OP_POP: u8 = 3;
const OP_ADD: u8 = 4;
const OP_SUB: u8 = 5;
const OP_MUL: u8 = 6;
const OP_DIV: u8 = 7;
const OP_LT: u8 = 8;
const OP_GT: u8 = 9;
const OP_EQ: u8 = 10;
const OP_NOT: u8 = 11;
const OP_JMPIFFALSE: u8 = 12;
const OP_JMP: u8 = 13;
const OP_CALLSYS: u8 = 14;
const OP_LOADLOCAL: u8 = 15;
const OP_STORELOCAL: u8 = 16;
const OP_DUP: u8 = 17;
const OP_HALT: u8 = 18;

const YBC_MAGIC: u32 = 0x59424331;

pub struct Codegen {
    code: Vec<u8>,
    strings: Vec<u8>,
    string_map: HashMap<String, u16>,
    locals: HashMap<String, u8>,
    next_local: u8,
}

fn syscall_id_for(name: &str) -> Option<u16> {
    match name {
        "print" => Some(1),
        "draw_rect" => Some(2),
        "get_tick" => Some(3),
        "sleep" => Some(4),
        "exit" => Some(5),
        _ => None,
    }
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            strings: Vec::new(),
            string_map: HashMap::new(),
            locals: HashMap::new(),
            next_local: 0,
        }
    }

    fn intern_str(&mut self, s: &str) -> u16 {
        if let Some(&idx) = self.string_map.get(s) {
            return idx;
        }
        let offset = self.strings.len() as u16;
        self.strings.extend_from_slice(&(s.len() as u16).to_le_bytes());
        self.strings.extend_from_slice(s.as_bytes());
        self.string_map.insert(s.to_string(), offset);
        offset
    }

    fn local_slot(&mut self, name: &str) -> u8 {
        if let Some(&slot) = self.locals.get(name) {
            return slot;
        }
        let slot = self.next_local;
        self.locals.insert(name.to_string(), slot);
        self.next_local += 1;
        slot
    }

    fn emit(&mut self, b: u8) { self.code.push(b); }
    fn emit_u32(&mut self, v: u32) { self.code.extend_from_slice(&v.to_le_bytes()); }
    fn emit_u16(&mut self, v: u16) { self.code.extend_from_slice(&v.to_le_bytes()); }
    fn emit_i64(&mut self, v: i64) { self.code.extend_from_slice(&v.to_le_bytes()); }

    fn gen_expr(&mut self, e: &Expr, prog: &ResolvedProgram) {
        match e {
            Expr::IntLit(n) => { self.emit(OP_PUSHINT); self.emit_i64(*n); }
            Expr::StrLit(s) => {
                let idx = self.intern_str(s);
                self.emit(OP_PUSHSTR); self.emit_u16(idx);
            }
            Expr::Var(name) => {
                let slot = self.local_slot(name);
                self.emit(OP_LOADLOCAL); self.emit(slot);
            }
            Expr::Not(inner) => { self.gen_expr(inner, prog); self.emit(OP_NOT); }
            Expr::BinOp(l, op, r) => {
                self.gen_expr(l, prog);
                self.gen_expr(r, prog);
                self.emit(match op {
                    BinOpKind::Add => OP_ADD,
                    BinOpKind::Sub => OP_SUB,
                    BinOpKind::Mul => OP_MUL,
                    BinOpKind::Div => OP_DIV,
                    BinOpKind::Lt => OP_LT,
                    BinOpKind::Gt => OP_GT,
                    BinOpKind::Eq => OP_EQ,
                });
            }
            Expr::Call(name, args) => {
                if let Some(sys_id) = syscall_id_for(name) {
                    for a in args { self.gen_expr(a, prog); }
                    self.emit(OP_CALLSYS);
                    self.emit_u16(sys_id);
                    self.emit(args.len() as u8);
                } else {
                    panic!("Unknown function/syscall: {} (user-defined fn calls chưa hỗ trợ trong VM đơn giản này)", name);
                }
            }
            Expr::FieldAccess(_, _) | Expr::MethodCall(_, _, _) => {
                panic!("Class field/method codegen chưa hỗ trợ trong VM bytecode phẳng hiện tại — cần mở rộng VM để có object heap trước khi bật lại nhánh này");
            }
        }
    }

    fn gen_stmt(&mut self, s: &Stmt, prog: &ResolvedProgram) {
        match s {
            Stmt::ExprStmt(e) => {
                self.gen_expr(e, prog);
                // Nếu expr có giá trị trả về (vd Call không phải exit/sleep), pop bỏ để cân bằng stack.
                // Đơn giản hoá: luôn pop sau ExprStmt trừ khi là syscall không trả giá trị (print/draw_rect/sleep/exit)
                if let Expr::Call(name, _) = e {
                    if matches!(name.as_str(), "get_tick") {
                        self.emit(OP_POP);
                    }
                }
            }
            Stmt::Let(name, val) => {
                self.gen_expr(val, prog);
                let slot = self.local_slot(name);
                self.emit(OP_STORELOCAL); self.emit(slot);
            }
            Stmt::Assign(name, val) => {
                self.gen_expr(val, prog);
                let slot = self.local_slot(name);
                self.emit(OP_STORELOCAL); self.emit(slot);
            }
            Stmt::If(cond, then_b, else_b) => {
                self.gen_expr(cond, prog);
                self.emit(OP_JMPIFFALSE);
                let jmp_else_pos = self.code.len();
                self.emit_u32(0); // placeholder

                for st in then_b { self.gen_stmt(st, prog); }

                self.emit(OP_JMP);
                let jmp_end_pos = self.code.len();
                self.emit_u32(0);

                let else_start = self.code.len() as u32;
                self.patch_u32(jmp_else_pos, else_start);

                for st in else_b { self.gen_stmt(st, prog); }

                let end_pos = self.code.len() as u32;
                self.patch_u32(jmp_end_pos, end_pos);
            }
            Stmt::While(cond, body) => {
                let loop_start = self.code.len() as u32;
                self.gen_expr(cond, prog);
                self.emit(OP_JMPIFFALSE);
                let jmp_exit_pos = self.code.len();
                self.emit_u32(0);

                for st in body { self.gen_stmt(st, prog); }

                self.emit(OP_JMP);
                self.emit_u32(loop_start);

                let exit_pos = self.code.len() as u32;
                self.patch_u32(jmp_exit_pos, exit_pos);
            }
            Stmt::Return(_) => {
                self.emit(OP_HALT);
            }
        }
    }

    fn patch_u32(&mut self, at: usize, val: u32) {
        self.code[at..at + 4].copy_from_slice(&val.to_le_bytes());
    }

    pub fn compile_entry(&mut self, entry_fn: &FnDecl, prog: &ResolvedProgram) -> Vec<u8> {
        for st in &entry_fn.body {
            self.gen_stmt(st, prog);
        }
        self.emit(OP_HALT);

        let mut out = Vec::new();
        out.extend_from_slice(&YBC_MAGIC.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // version
        out.push(self.next_local);
        out.push(0); // pad
        out.extend_from_slice(&(self.code.len() as u32).to_le_bytes());
        out.extend_from_slice(&(self.strings.len() as u32).to_le_bytes());
        out.extend_from_slice(&256u16.to_le_bytes()); // max_stack
        out.extend_from_slice(&0u32.to_le_bytes()); // entry_offset = 0

        out.extend_from_slice(&self.code);
        out.extend_from_slice(&self.strings);
        out
    }
}
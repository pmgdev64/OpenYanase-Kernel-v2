use std::collections::HashMap;
use crate::ast::*;
use crate::resolver::ResolvedProgram;
use crate::stdlib;

// --- Opcodes Cơ Bản ---
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

// --- Opcodes Hướng Đối Tượng ---
const OP_NEWOBJECT: u8 = 19;
const OP_GETFIELD: u8 = 20;
const OP_SETFIELD: u8 = 21;
const OP_CALLMETHOD: u8 = 22;
const OP_RET: u8 = 23;

// --- Opcodes Mảng ---
const OP_NEWARRAY: u8 = 24;
const OP_GETINDEX: u8 = 25;
const OP_SETINDEX: u8 = 26;
const OP_ARRAYLEN: u8 = 27;

const YBC_MAGIC: u32 = 0x59424331;

fn syscall_id_for(name: &str) -> Option<(u16, u8)> {
    stdlib::resolve(name).map(|f| (f.sys_id, f.argc))
}

#[derive(Debug, Clone)]
struct ClassInfo {
    id: u16,
    field_slots: HashMap<String, u8>,
    method_addrs: HashMap<String, u32>,
}

#[derive(Debug, Clone)]
struct LoopEnv {
    continue_pc: u32,
    break_patches: Vec<usize>,
}

pub struct Codegen {
    code: Vec<u8>,
    strings: Vec<u8>,
    string_map: HashMap<String, u16>,
    locals: HashMap<String, u8>,
    next_local: u8,
    classes: HashMap<String, ClassInfo>,
    next_class_id: u16,
    var_class_hint: HashMap<String, String>,
    current_class: Option<String>,
    loop_stack: Vec<LoopEnv>,
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            strings: Vec::new(),
            string_map: HashMap::new(),
            locals: HashMap::new(),
            next_local: 0,
            classes: HashMap::new(),
            next_class_id: 1,
            var_class_hint: HashMap::new(),
            current_class: None,
            loop_stack: Vec::new(),
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

    fn emit(&mut self, b: u8) {
        self.code.push(b);
    }

    fn emit_u32(&mut self, v: u32) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    fn emit_u16(&mut self, v: u16) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    fn emit_i64(&mut self, v: i64) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    fn patch_u32(&mut self, at: usize, val: u32) {
        self.code[at..at + 4].copy_from_slice(&val.to_le_bytes());
    }

    fn register_classes(&mut self, prog: &ResolvedProgram) {
        let mut names: Vec<&String> = prog.classes.keys().collect();
        names.sort();
        for name in names {
            let class = &prog.classes[name];
            let mut field_slots = HashMap::new();
            for (i, f) in class.fields.iter().enumerate() {
                field_slots.insert(f.clone(), i as u8);
            }
            let mut method_addrs = HashMap::new();
            for m in &class.methods {
                method_addrs.insert(m.name.clone(), 0);
            }
            self.classes.insert(
                name.clone(),
                ClassInfo {
                    id: self.next_class_id,
                    field_slots,
                    method_addrs,
                },
            );
            self.next_class_id += 1;
        }
    }

    fn emit_all_methods(&mut self, prog: &ResolvedProgram) -> Vec<(usize, String, String)> {
        let mut pending_patches = Vec::new();
        let mut class_names: Vec<String> = prog.classes.keys().cloned().collect();
        class_names.sort();

        for cname in &class_names {
            let class = prog.classes[cname].clone();
            for method in &class.methods {
                let addr = self.code.len() as u32;
                self.classes.get_mut(cname).unwrap().method_addrs.insert(method.name.clone(), addr);

                self.locals.clear();
                self.next_local = 0;
                self.local_slot("this");
                for p in &method.params {
                    self.local_slot(p);
                }

                let saved_class = self.current_class.clone();
                self.current_class = Some(cname.clone());

                for st in &method.body {
                    self.gen_stmt(st, prog, &mut pending_patches);
                }

                self.emit(OP_PUSHINT);
                self.emit_i64(0);
                self.emit(OP_RET);

                self.current_class = saved_class;
            }
        }
        pending_patches
    }

    fn infer_class_of(&self, e: &Expr) -> String {
        match e {
            Expr::Var(name) => self.var_class_hint.get(name).cloned().unwrap_or_else(|| {
                if name == "this" {
                    self.current_class.clone().expect("Cannot use 'this' outside of class")
                } else {
                    if self.classes.contains_key(name) {
                        name.clone()
                    } else {
                        panic!(
                            "Cannot infer class for variable '{}'. If '{}' is a system/stdlib module, verify the method call is registered in stdlib.rs!",
                            name, name
                        );
                    }
                }
            }),
            Expr::MethodCall(obj, _, _) => self.infer_class_of(obj),
            Expr::FieldAccess(obj, _) => self.infer_class_of(obj),
            _ => panic!("Complex class inference unsupported in this pass: {:?}", e),
        }
    }

    fn gen_expr(&mut self, e: &Expr, prog: &ResolvedProgram, patches: &mut Vec<(usize, String, String)>) {
        match e {
            Expr::IntLit(n) => {
                self.emit(OP_PUSHINT);
                self.emit_i64(*n);
            }
            Expr::StrLit(s) => {
                let idx = self.intern_str(s);
                self.emit(OP_PUSHSTR);
                self.emit_u16(idx);
            }
            Expr::Var(name) => {
                let slot = self.local_slot(name);
                self.emit(OP_LOADLOCAL);
                self.emit(slot);
            }
            Expr::ArrayLit(items) => {
                self.emit(OP_PUSHINT);
                self.emit_i64(items.len() as i64);
                self.emit(OP_NEWARRAY);

                for (i, item) in items.iter().enumerate() {
                    self.emit(OP_DUP);
                    self.emit(OP_PUSHINT);
                    self.emit_i64(i as i64);
                    self.gen_expr(item, prog, patches);
                    self.emit(OP_SETINDEX);
                }
            }
            Expr::IndexAccess(arr, idx) => {
                self.gen_expr(arr, prog, patches);
                self.gen_expr(idx, prog, patches);
                self.emit(OP_GETINDEX);
            }
            Expr::ArrayLen(arr) => {
                self.gen_expr(arr, prog, patches);
                self.emit(OP_ARRAYLEN);
            }
            Expr::Not(inner) => {
                self.gen_expr(inner, prog, patches);
                self.emit(OP_NOT);
            }
            Expr::BinOp(l, op, r) => {
                match op {
                    BinOpKind::And => {
                        // AND short-circuit
                        self.gen_expr(l, prog, patches);
                        self.emit(OP_DUP);
                        self.emit(OP_JMPIFFALSE);
                        let else_pos = self.code.len();
                        self.emit_u32(0);
                        self.emit(OP_POP);
                        self.gen_expr(r, prog, patches);
                        self.emit(OP_JMP);
                        let end_pos = self.code.len();
                        self.emit_u32(0);
                        self.patch_u32(else_pos, self.code.len() as u32);
                        self.emit(OP_PUSHINT);
                        self.emit_i64(0);
                        self.patch_u32(end_pos, self.code.len() as u32);
                    }
                    BinOpKind::Or => {
                        // OR short-circuit
                        self.gen_expr(l, prog, patches);
                        self.emit(OP_DUP);
                        self.emit(OP_JMPIFFALSE);
                        let else_pos = self.code.len();
                        self.emit_u32(0);
                        self.emit(OP_JMP);
                        let end_pos = self.code.len();
                        self.emit_u32(0);
                        self.patch_u32(else_pos, self.code.len() as u32);
                        self.emit(OP_POP);
                        self.gen_expr(r, prog, patches);
                        self.patch_u32(end_pos, self.code.len() as u32);
                    }
                    _ => {
                        self.gen_expr(l, prog, patches);
                        self.gen_expr(r, prog, patches);
                        match op {
                            BinOpKind::Add => self.emit(OP_ADD),
                            BinOpKind::Sub => self.emit(OP_SUB),
                            BinOpKind::Mul => self.emit(OP_MUL),
                            BinOpKind::Div => self.emit(OP_DIV),
                            BinOpKind::Lt => self.emit(OP_LT),
                            BinOpKind::Gt => self.emit(OP_GT),
                            BinOpKind::Eq => self.emit(OP_EQ),
                            BinOpKind::Neq => {
                                self.emit(OP_EQ);
                                self.emit(OP_NOT);
                            }
                            _ => {}
                        }
                    }
                }
            }
            Expr::FieldAccess(obj, field) => {
                self.gen_expr(obj, prog, patches);
                let class_name = self.infer_class_of(obj);

                let field_idx = *self.classes.get(&class_name)
                    .unwrap_or_else(|| panic!("Class {} not found", class_name))
                    .field_slots.get(field)
                    .unwrap_or_else(|| panic!("Field {} not found in class {}", field, class_name));

                self.emit(OP_GETFIELD);
                self.emit(field_idx);
            }
            Expr::MethodCall(obj, method, args) => {
                // Handle new ClassName()
                if let Expr::Var(v) = obj.as_ref() {
                    if v == "new" {
                        let class_id = self.classes.get(method)
                            .unwrap_or_else(|| panic!("Class {} not found for instantiation", method))
                            .id;

                        self.emit(OP_NEWOBJECT);
                        self.emit_u16(class_id);
                        return;
                    }

                    // Handle system modules
                    if matches!(v.as_str(),
                        "io" | "time" | "screen" | "gfx" | "proc" | "sound"
                        | "process" | "fs" | "system" | "console"
                    ) {
                        let qname = format!("{}.{}", v, method);
                        match syscall_id_for(&qname) {
                            Some((sys_id, expected_argc)) => {
                                if args.len() != expected_argc as usize {
                                    panic!(
                                        "Syscall '{}' expects {} args, got {}",
                                        qname, expected_argc, args.len()
                                    );
                                }
                                for a in args {
                                    self.gen_expr(a, prog, patches);
                                }
                                self.emit(OP_CALLSYS);
                                self.emit_u16(sys_id);
                                self.emit(args.len() as u8);
                                return;
                            }
                            None => {
                                panic!(
                                    "Unknown stdlib function '{}' — add it to compiler/src/stdlib.rs",
                                    qname
                                );
                            }
                        }
                    }
                }

                // Normal method call
                self.gen_expr(obj, prog, patches);
                for a in args {
                    self.gen_expr(a, prog, patches);
                }

                let class_name = self.infer_class_of(obj);
                self.emit(OP_CALLMETHOD);
                let patch_pos = self.code.len();
                self.emit_u32(0);
                self.emit(args.len() as u8);
                patches.push((patch_pos, class_name, method.clone()));
            }
            Expr::Call(name, args) => {
                // Handle syscall()
                if name == "syscall" {
                    if let Expr::IntLit(n) = args[0] {
                        for arg in &args[1..] {
                            self.gen_expr(arg, prog, patches);
                        }
                        self.emit(OP_CALLSYS);
                        self.emit_u16(n as u16);
                        self.emit((args.len() - 1) as u8);
                        return;
                    } else {
                        panic!("syscall arg 0 must be IntLit");
                    }
                }

                // Handle stdlib functions
                if let Some((sys_id, expected_argc)) = syscall_id_for(name) {
                    if args.len() != expected_argc as usize {
                        panic!("Syscall {} expects {} args, got {}", name, expected_argc, args.len());
                    }
                    for a in args {
                        self.gen_expr(a, prog, patches);
                    }
                    self.emit(OP_CALLSYS);
                    self.emit_u16(sys_id);
                    self.emit(args.len() as u8);
                    return;
                }
                panic!("Unknown function call: {}", name);
            }
        }
    }

    fn gen_stmt(&mut self, s: &Stmt, prog: &ResolvedProgram, patches: &mut Vec<(usize, String, String)>) {
        match s {
            Stmt::ExprStmt(e) => {
                self.gen_expr(e, prog, patches);
                if !matches!(e, Expr::Call(n, _) if matches!(n.as_str(), "print" | "sleep" | "exit")) {
                    self.emit(OP_POP);
                }
            }
            Stmt::Let(name, val) => {
                if let Expr::MethodCall(obj, cls, _) = val {
                    if let Expr::Var(v) = obj.as_ref() {
                        if v == "new" {
                            self.var_class_hint.insert(name.clone(), cls.clone());
                        }
                    }
                }
                self.gen_expr(val, prog, patches);
                let slot = self.local_slot(name);
                self.emit(OP_STORELOCAL);
                self.emit(slot);
            }
            Stmt::Assign(name, val) => {
                if let Expr::MethodCall(obj, cls, _) = val {
                    if let Expr::Var(v) = obj.as_ref() {
                        if v == "new" {
                            self.var_class_hint.insert(name.clone(), cls.clone());
                        }
                    }
                }
                self.gen_expr(val, prog, patches);
                let slot = self.local_slot(name);
                self.emit(OP_STORELOCAL);
                self.emit(slot);
            }
            Stmt::IndexAssign(arr, idx, val) => {
                self.gen_expr(arr, prog, patches);
                self.gen_expr(idx, prog, patches);
                self.gen_expr(val, prog, patches);
                self.emit(OP_SETINDEX);
            }
            Stmt::FieldAssign(obj, field, val) => {
                self.gen_expr(obj, prog, patches);
                self.gen_expr(val, prog, patches);
                let class_name = self.infer_class_of(obj);

                let field_idx = *self.classes.get(&class_name)
                    .unwrap_or_else(|| panic!("Class {} not found", class_name))
                    .field_slots.get(field)
                    .unwrap_or_else(|| panic!("Field {} not found in class {}", field, class_name));

                self.emit(OP_SETFIELD);
                self.emit(field_idx);
            }
            Stmt::Block(stmts) => {
                for st in stmts {
                    self.gen_stmt(st, prog, patches);
                }
            }
            Stmt::If(cond, then_b, else_b) => {
                self.gen_expr(cond, prog, patches);
                self.emit(OP_JMPIFFALSE);
                let jmp_else = self.code.len();
                self.emit_u32(0);

                for st in then_b {
                    self.gen_stmt(st, prog, patches);
                }

                self.emit(OP_JMP);
                let jmp_end = self.code.len();
                self.emit_u32(0);

                self.patch_u32(jmp_else, self.code.len() as u32);
                for st in else_b {
                    self.gen_stmt(st, prog, patches);
                }
                self.patch_u32(jmp_end, self.code.len() as u32);
            }
            Stmt::While(cond, body) => {
                let loop_start = self.code.len() as u32;
                self.gen_expr(cond, prog, patches);
                self.emit(OP_JMPIFFALSE);
                let jmp_exit = self.code.len();
                self.emit_u32(0);

                self.loop_stack.push(LoopEnv {
                    continue_pc: loop_start,
                    break_patches: Vec::new(),
                });

                for st in body {
                    self.gen_stmt(st, prog, patches);
                }

                self.emit(OP_JMP);
                self.emit_u32(loop_start);

                let exit_pos = self.code.len() as u32;
                self.patch_u32(jmp_exit, exit_pos);

                let env = self.loop_stack.pop().expect("Loop stack underflow");
                for bp in env.break_patches {
                    self.patch_u32(bp, exit_pos);
                }
            }
            Stmt::Break => {
                if self.loop_stack.is_empty() {
                    panic!("'break' statement outside of a loop");
                }

                self.emit(OP_JMP);
                let pos = self.code.len();
                self.emit_u32(0);

                self.loop_stack.last_mut().unwrap().break_patches.push(pos);
            }
            Stmt::Continue => {
                if let Some(env) = self.loop_stack.last() {
                    let target_pc = env.continue_pc;
                    self.emit(OP_JMP);
                    self.emit_u32(target_pc);
                } else {
                    panic!("'continue' statement outside of a loop");
                }
            }
            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    self.gen_expr(e, prog, patches);
                } else {
                    self.emit(OP_PUSHINT);
                    self.emit_i64(0);
                }

                if self.current_class.is_some() {
                    self.emit(OP_RET);
                } else {
                    self.emit(OP_HALT);
                }
            }
        }
    }

    pub fn compile_entry(&mut self, entry_fn: &FnDecl, prog: &ResolvedProgram) -> Vec<u8> {
        self.register_classes(prog);

        self.emit(OP_JMP);
        let jmp_main = self.code.len();
        self.emit_u32(0);

        let mut patches = self.emit_all_methods(prog);

        self.patch_u32(jmp_main, self.code.len() as u32);
        self.locals.clear();
        self.next_local = 0;
        self.current_class = None;

        for st in &entry_fn.body {
            self.gen_stmt(st, prog, &mut patches);
        }
        self.emit(OP_HALT);

        for (pos, cname, mname) in patches {
            let class_info = self.classes.get(&cname).unwrap_or_else(|| panic!("Class {} missing", cname));
            let addr = *class_info.method_addrs.get(&mname).unwrap_or_else(|| panic!("Method {} missing in {}", mname, cname));
            self.patch_u32(pos, addr);
        }

        let mut out = Vec::new();
        out.extend_from_slice(&YBC_MAGIC.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.push(self.next_local.max(1));
        out.push(0);
        out.extend_from_slice(&(self.code.len() as u32).to_le_bytes());
        out.extend_from_slice(&(self.strings.len() as u32).to_le_bytes());
        out.extend_from_slice(&256u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());

        out.extend_from_slice(&self.code);
        out.extend_from_slice(&self.strings);
        out
    }
}
#!/usr/bin/env python3
# Yanase Script Compiler (YSCC) - Complete Function, Mouse & Cursor Syscall Support

import sys
import struct
import tarfile
import io
import re

OPS = {
    "nop": (0x00, None), "push": (0x01, "i32"), "pop": (0x02, None),
    "dup": (0x03, None), "add": (0x04, None), "sub": (0x05, None),
    "mul": (0x06, None), "div": (0x07, None), "eq": (0x08, None),
    "lt": (0x09, None), "gt": (0x0A, None), "jmp": (0x0B, "label"),
    "jz": (0x0C, "label"), "call": (0x0D, "label"), "ret": (0x0E, None),
    "load8": (0x0F, None), "store8": (0x10, None),
    "syscall": (0x11, "u8"), "halt": (0x12, None),
}

SYSCALLS = {
    "write_byte": 0, "write_str": 1, "read_key": 2, "get_ticks": 3,
    "exit": 4, "read_char": 5, "getpid": 6, "sleep": 7, "rand": 8,
    "clear": 9, "screen_w": 10, "screen_h": 11, "mouse_x": 12,
    "mouse_y": 13, "mouse_btn": 14, "beep": 15, "set_gfx_mode": 16,
    "set_tty_mode": 17, "put_pixel": 18, "draw_rect": 19, "draw_str_gfx": 20,
    "draw_cursor": 21, "mouse_clicked": 22, "mouse_right_clicked": 23,
}

def emit_store_i32(addr):
    return [
        f"push {addr}", "store8",
        "push 256", "div", f"push {addr+1}", "store8",
        "push 256", "div", f"push {addr+2}", "store8",
        "push 256", "div", f"push {addr+3}", "store8",
        "pop"
    ]

def emit_load_i32(addr):
    return [
        f"push {addr+3}", "load8",
        "push 256", "mul", f"push {addr+2}", "load8", "add",
        "push 256", "mul", f"push {addr+1}", "load8", "add",
        "push 256", "mul", f"push {addr}", "load8", "add"
    ]

def emit_eval_expr(expr_str, var_table, func_table=None, compiler=None):
    if func_table is None: func_table = {}
    expr_str = expr_str.strip()
    if not expr_str: return ["push 0"]
    
    if expr_str.startswith('"') and expr_str.endswith('"'):
        text = expr_str[1:-1]
        if compiler:
            off, length = compiler.add_string(text)
            return [f"push {off}", f"push {length}"]
        return ["push 0", "push 0"]

    if expr_str.startswith("0x") or expr_str.startswith("0X"):
        try:
            val = int(expr_str, 16)
            return [f"push {val}"]
        except ValueError:
            pass

    if expr_str.isdigit() or (expr_str.startswith('-') and expr_str[1:].isdigit()):
        return [f"push {expr_str}"]
    if expr_str == "true": return ["push 1"]
    if expr_str == "false": return ["push 0"]
    
    if expr_str in var_table:
        return emit_load_i32(var_table[expr_str])
    
    if expr_str.endswith('()') or expr_str.endswith('();'):
        func = expr_str.replace('()', '').replace('();', '').strip()
        if func in SYSCALLS: return [f"syscall {SYSCALLS[func]}"]
        if func in func_table: return [f"call func_{func}"]
        return ["push 0"]
    
    if '(' in expr_str and ')' in expr_str:
        match = re.match(r'([a-zA-Z_]\w*)\s*\(([^)]*)\)', expr_str)
        if match:
            func_name, args_str = match.group(1), match.group(2)
            args = [a.strip() for a in args_str.split(',') if a.strip()]
            result = []
            for arg in args:
                result.extend(emit_eval_expr(arg, var_table, func_table, compiler))
            if func_name in SYSCALLS:
                result.append(f"syscall {SYSCALLS[func_name]}")
            elif func_name in func_table:
                result.append(f"call func_{func_name}")
            return result
                
    ops = [('==', 'eq'), ('!=', 'neq'), ('<', 'lt'), ('>', 'gt'),
           ('<=', 'le'), ('>=', 'ge'), ('+', 'add'), ('-', 'sub'),
           ('*', 'mul'), ('/', 'div')]
    for op_str, op_code in ops:
        if op_str in expr_str:
            parts = re.split(rf'\s*{re.escape(op_str)}\s*', expr_str, maxsplit=1)
            if len(parts) == 2:
                left, right = [p.strip() for p in parts]
                result = emit_eval_expr(left, var_table, func_table, compiler) + emit_eval_expr(right, var_table, func_table, compiler)
                if op_code == 'neq': result.extend(['eq', 'push 1', 'sub'])
                elif op_code == 'le': result.extend(['gt', 'push 1', 'sub'])
                elif op_code == 'ge': result.extend(['lt', 'push 1', 'sub'])
                else: result.append(op_code)
                return result
                
    if expr_str.startswith('-'):
        return emit_eval_expr(expr_str[1:].strip(), var_table, func_table, compiler) + ['push 0', 'sub']
    if expr_str.startswith('!'):
        return emit_eval_expr(expr_str[1:].strip(), var_table, func_table, compiler) + ['push 1', 'eq']
    return ["push 0"]

class YanaseCompiler:
    def __init__(self):
        self.asm_lines = []
        self.data_bytes = bytearray()
        self.string_table = {}
        self.var_table = {}
        self.func_table = {}
        self.label_counter = 0
        self.has_main = False
        self.in_main = False
        self.next_var_addr = 0
        self.num_addr = 0
        self.cnt_addr = 0
        self.buf_addr = 0
        
    def new_label(self):
        self.label_counter += 1
        return f"_L{self.label_counter}"
    
    def add_string(self, text_raw):
        text = bytes(text_raw, "utf-8").decode("unicode_escape")
        if text in self.string_table: return self.string_table[text]
        offset = len(self.data_bytes)
        raw = text.encode('utf-8')
        self.data_bytes.extend(raw)
        self.string_table[text] = (offset, len(raw))
        return offset, len(raw)
    
    def emit(self, line):
        self.asm_lines.append(line)
        
    def compile_stmts(self, lines, i):
        while i < len(lines):
            raw_line = lines[i]
            line = raw_line.split("//")[0].strip()
            line_clean = line.rstrip(';').strip()
            
            if not line_clean:
                i += 1
                continue
            
            if line_clean == '}':
                return i
                
            if line_clean == '{':
                i += 1
                continue
            
            if line_clean.startswith("return "):
                expr = line_clean[7:].strip()
                if expr:
                    self.asm_lines.extend(emit_eval_expr(expr, self.var_table, self.func_table, self))
                self.emit("ret")
                i += 1
                continue

            m_call = re.match(r'^([a-zA-Z_]\w*)\s*\((.*)\)$', line_clean)
            if m_call:
                func_name, args_str = m_call.group(1), m_call.group(2)
                args = [a.strip() for a in args_str.split(',') if a.strip()]
                
                for arg in args:
                    self.asm_lines.extend(emit_eval_expr(arg, self.var_table, self.func_table, self))
                
                if func_name in SYSCALLS:
                    self.emit(f"syscall {SYSCALLS[func_name]}")
                    if func_name == "read_char":
                        self.emit("pop")
                elif func_name in self.func_table:
                    self.emit(f"call func_{func_name}")
                i += 1
                continue
            
            m = re.search(r'print\s+"([^"]*)"', line_clean)
            if m:
                off, length = self.add_string(m.group(1))
                self.emit(f"push {off}")
                self.emit(f"push {length}")
                self.emit("syscall 1")
                i += 1
                continue
                
            m = re.search(r'(?:print_num|print)\s*\(([^)]+)\)', line_clean)
            if m:
                expr = m.group(1).strip()
                self.asm_lines.extend(emit_eval_expr(expr, self.var_table, self.func_table, self))
                self.emit("call __print_num")
                i += 1
                continue
                
            m = re.search(r'(?:let|var|int)?\s*([a-zA-Z_]\w*)\s*=\s*(.*)', line_clean)
            if m:
                vname, expr = m.groups()
                vname, expr = vname.strip(), expr.strip()
                if vname not in self.var_table:
                    self.var_table[vname] = self.next_var_addr
                    self.next_var_addr += 4
                self.asm_lines.extend(emit_eval_expr(expr, self.var_table, self.func_table, self))
                self.asm_lines.extend(emit_store_i32(self.var_table[vname]))
                i += 1
                continue

            m = re.search(r'if\s*\(([^)]+)\)', line_clean)
            if m:
                cond = m.group(1).strip()
                else_label = self.new_label()
                end_label = self.new_label()
                self.asm_lines.extend(emit_eval_expr(cond, self.var_table, self.func_table, self))
                self.emit(f"jz {else_label}")
                
                if i + 1 < len(lines) and lines[i+1].split("//")[0].strip() == '{':
                    i += 1
                    
                i = self.compile_stmts(lines, i + 1)
                self.emit(f"jmp {end_label}")
                self.emit(f"{else_label}:")
                
                if i + 1 < len(lines) and lines[i+1].split("//")[0].strip().startswith('else'):
                    i += 1
                    if i + 1 < len(lines) and lines[i+1].split("//")[0].strip() == '{':
                        i += 1
                    i = self.compile_stmts(lines, i + 1)
                
                self.emit(f"{end_label}:")
                i += 1
                continue
            
            m = re.search(r'while\s*\(([^)]+)\)', line_clean)
            if m:
                cond = m.group(1).strip()
                start_label = self.new_label()
                end_label = self.new_label()
                self.emit(f"{start_label}:")
                self.asm_lines.extend(emit_eval_expr(cond, self.var_table, self.func_table, self))
                self.emit(f"jz {end_label}")
                
                if i + 1 < len(lines) and lines[i+1].split("//")[0].strip() == '{':
                    i += 1
                    
                i = self.compile_stmts(lines, i + 1)
                self.emit(f"jmp {start_label}")
                self.emit(f"{end_label}:")
                i += 1
                continue
                
            i += 1
        return i

    def compile(self, src):
        lines = src.splitlines()
        
        for line in lines:
            line_clean = line.split("//")[0].strip()
            for m in re.finditer(r'print\s+"([^"]*)"', line_clean):
                self.add_string(m.group(1))
                
        for line in lines:
            line_clean = line.split("//")[0].strip()
            m = re.match(r'\b(func|fn)\s+([a-zA-Z_]\w*)\s*\(', line_clean)
            if m:
                self.func_table[m.group(2)] = True

        self.next_var_addr = (len(self.data_bytes) + 3) & ~3
        self.num_addr = self.next_var_addr
        self.cnt_addr = self.next_var_addr + 4
        self.buf_addr = self.next_var_addr + 8
        self.next_var_addr += 24
        
        self.asm_lines = ["jmp main"]
        
        i = 0
        while i < len(lines):
            line = lines[i].split("//")[0].strip()
            if not line:
                i += 1
                continue
            
            match = re.match(r'\b(func|fn)\s+([a-zA-Z_]\w*)\s*\(([^)]*)\)', line)
            if match:
                func_name = match.group(2)
                params = [p.strip() for p in match.group(3).split(',') if p.strip()]
                
                if func_name == "main":
                    self.emit("main:")
                    self.has_main = True
                    self.in_main = True
                else:
                    self.emit(f"func_{func_name}:")
                
                for p in reversed(params):
                    if p not in self.var_table:
                        self.var_table[p] = self.next_var_addr
                        self.next_var_addr += 4
                    self.asm_lines.extend(emit_store_i32(self.var_table[p]))

                if i + 1 < len(lines) and lines[i+1].split("//")[0].strip() == '{':
                    i += 1
                    
                i = self.compile_stmts(lines, i + 1)
                
                if self.in_main:
                    self.emit("syscall 4")
                    self.emit("halt")
                    self.in_main = False
                else:
                    self.emit("ret")
                
                i += 1
                continue
            i += 1
        
        if not self.has_main:
            print("Error: No 'func main()' found!")
            sys.exit(1)
        
        self.add_print_num_subroutine()
        return "\n".join(self.asm_lines), bytes(self.data_bytes)
    
    def add_print_num_subroutine(self):
        subroutine = [
            "__print_num:",
            *emit_store_i32(self.num_addr),
            *emit_load_i32(self.num_addr),
            "jz __pn_zero",
            "push 0",
            *emit_store_i32(self.cnt_addr),
            "__pn_loop:",
            *emit_load_i32(self.num_addr),
            "jz __pn_print",
            *emit_load_i32(self.num_addr),
            "dup", "push 10", "div", "push 10", "mul", "sub", "push 48", "add",
            *emit_load_i32(self.cnt_addr), f"push {self.buf_addr}", "add", "store8", "pop",
            *emit_load_i32(self.cnt_addr), "push 1", "add", *emit_store_i32(self.cnt_addr),
            *emit_load_i32(self.num_addr), "push 10", "div", *emit_store_i32(self.num_addr),
            "jmp __pn_loop",
            "__pn_zero:", "push 48", "syscall 0", "ret",
            "__pn_print:", *emit_load_i32(self.cnt_addr), "jz __pn_done",
            *emit_load_i32(self.cnt_addr), "push 1", "sub", *emit_store_i32(self.cnt_addr),
            *emit_load_i32(self.cnt_addr), f"push {self.buf_addr}", "add", "load8", "syscall 0",
            "jmp __pn_print",
            "__pn_done:", "ret"
        ]
        self.asm_lines.extend(subroutine)

def assemble(asm_text):
    lines = [line.split("#", 1)[0].strip() for line in asm_text.splitlines() if line.strip()]
    labels = {}
    pc = 0
    instrs = []
    for line in lines:
        if line.endswith(":"):
            labels[line[:-1]] = pc
            continue
        parts = line.split()
        if not parts: continue
        mnem = parts[0]
        if mnem not in OPS: raise Exception(f"Unknown opcode: {mnem}")
        opcode, arg_kind = OPS[mnem]
        size = 1 + (4 if arg_kind in ("i32", "label") else (1 if arg_kind == "u8" else 0))
        instrs.append((pc, mnem, parts[1:], opcode, arg_kind))
        pc += size
    
    out = bytearray()
    for pc, mnem, args, opcode, arg_kind in instrs:
        out.append(opcode)
        if arg_kind == "i32":
            try: out += struct.pack("<i", int(args[0]))
            except ValueError:
                if args[0] in labels: out += struct.pack("<i", labels[args[0]])
                else: raise Exception(f"Invalid integer or label: {args[0]}")
        elif arg_kind == "u8": out.append(int(args[0]))
        elif arg_kind == "label":
            if args[0] in labels: out += struct.pack("<i", labels[args[0]])
            else: raise Exception(f"Undefined label: {args[0]}")
    return bytes(out)

def pack_tar(tar_filename, bytecode, data_bytes):
    with tarfile.open(tar_filename, "w") as tar:
        b_info = tarfile.TarInfo(name="main.bc")
        b_info.size = len(bytecode)
        tar.addfile(tarinfo=b_info, fileobj=io.BytesIO(bytecode))
        if len(data_bytes) > 0:
            d_info = tarfile.TarInfo(name="data.bin")
            d_info.size = len(data_bytes)
            tar.addfile(tarinfo=d_info, fileobj=io.BytesIO(data_bytes))

def main():
    if len(sys.argv) < 3:
        print("Usage: python compiler.py <input.ys> <output.abp>")
        sys.exit(1)
    
    with open(sys.argv[1], "r", encoding="utf-8") as f:
        src = f.read()
    
    compiler = YanaseCompiler()
    asm_text, data_bytes = compiler.compile(src)
    pack_tar(sys.argv[2], assemble(asm_text), data_bytes)
    print("Build Successful!")

if __name__ == "__main__":
    main()
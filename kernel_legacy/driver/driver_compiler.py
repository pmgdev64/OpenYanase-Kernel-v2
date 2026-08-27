#!/usr/bin/env python3
# Yanase Driver Compiler (YSDC) - Fixed Version
import sys
import os
import struct
import tarfile
import io
import re
import argparse

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
    "driver_register": 30, "driver_ready": 31, "driver_io_port": 32,
    "driver_claim_irq": 33, "driver_release_irq": 34, "driver_info": 35,
    "driver_unregister": 36, "driver_send_event": 37, "driver_wait_event": 38,
    "driver_poll_event": 39, "driver_get_state": 40, "driver_get_count": 41,
    "dbgserial": 42,
}

# FIXED: Sửa đúng số tham số truyền cho driver_info (2) và driver_send_event (5)
SYSCALL_ARGS = {
    "write_byte": 1, "write_str": 2, "read_key": 0, "get_ticks": 0,
    "exit": 0, "read_char": 0, "getpid": 0, "sleep": 1, "rand": 0,
    "clear": 0, "screen_w": 0, "screen_h": 0, "mouse_x": 0,
    "mouse_y": 0, "mouse_btn": 0, "beep": (0, 1, 2), "set_gfx_mode": 0,
    "set_tty_mode": 0, "put_pixel": 3, "draw_rect": 5, "draw_str_gfx": 4,
    "draw_cursor": 0, "mouse_clicked": 0, "mouse_right_clicked": 0,
    "driver_register": 4, "driver_ready": 0, "driver_io_port": 2,
    "driver_claim_irq": 1, "driver_release_irq": 1, "driver_info": 2,
    "driver_unregister": 1, "driver_send_event": 5, "driver_wait_event": 0,
    "driver_poll_event": 0, "driver_get_state": 1, "driver_get_count": 0,
    "dbgserial": 2,
}

# FIXED: Thêm driver_wait_event & driver_poll_event vào danh sách trả về giá trị
SYSCALLS_WITH_RET = {
    "read_key", "get_ticks", "read_char", "getpid", "rand", "screen_w",
    "screen_h", "mouse_x", "mouse_y", "mouse_btn", "mouse_clicked",
    "mouse_right_clicked", "driver_register", "driver_ready", "driver_io_port",
    "driver_claim_irq", "driver_release_irq", "driver_info", "driver_unregister",
    "driver_send_event", "driver_get_state", "driver_get_count",
    "driver_wait_event", "driver_poll_event"
}

DRIVER_TYPES = {
    "block": 0, "net": 1, "input": 2, "display": 3,
    "audio": 4, "sound": 4, "beep": 4, "hid": 5, "bus": 6, "char": 7,
}


class Preprocessor:
    def __init__(self, include_dirs=None):
        self.include_dirs = include_dirs or []

    def resolve_path(self, current_file: str, import_path: str) -> str:
        curr_dir = os.path.dirname(os.path.abspath(current_file))
        target = os.path.join(curr_dir, import_path)
        if os.path.exists(target):
            return os.path.abspath(target)

        for inc_dir in self.include_dirs:
            target = os.path.join(inc_dir, import_path)
            if os.path.exists(target):
                return os.path.abspath(target)

        raise FileNotFoundError(f"Không tìm thấy file import: '{import_path}' (được gọi từ '{current_file}')")

    def process_file(self, filepath: str, call_stack=None, loaded_files=None) -> str:
        if call_stack is None:
            call_stack = []
        if loaded_files is None:
            loaded_files = set()

        abs_path = os.path.abspath(filepath)

        if abs_path in call_stack:
            cycle = " -> ".join([os.path.basename(p) for p in call_stack + [abs_path]])
            raise RuntimeError(f"[Import Error] Phát hiện vòng lặp Circular Import: {cycle}")

        if abs_path in loaded_files:
            return f"// [Bỏ qua file đã import trước đó: {os.path.basename(abs_path)}]\n"

        loaded_files.add(abs_path)
        call_stack.append(abs_path)

        if not os.path.exists(abs_path):
            raise FileNotFoundError(f"File không tồn tại: {filepath}")

        output_lines = [f"// === START IMPORT: {os.path.basename(abs_path)} ==="]

        with open(abs_path, "r", encoding="utf-8") as f:
            for line_no, line in enumerate(f, 1):
                stripped = line.strip()

                if stripped.startswith("import ") or stripped.startswith("#include "):
                    match = re.match(r'^(?:import|#include)\s+["<]?([^">]+)[">]?', stripped)
                    if match:
                        import_target = match.group(1).strip()
                        resolved = self.resolve_path(abs_path, import_target)
                        expanded = self.process_file(resolved, call_stack, loaded_files)
                        output_lines.append(expanded)
                    else:
                        raise SyntaxError(f"[{os.path.basename(abs_path)}:{line_no}] Cú pháp import không hợp lệ: {stripped}")
                else:
                    output_lines.append(line.rstrip())

        output_lines.append(f"// === END IMPORT: {os.path.basename(abs_path)} ===")
        call_stack.pop()

        return "\n".join(output_lines)


class YanaseDriverCompiler:
    def __init__(self):
        self.asm_lines = []
        self.data_bytes = bytearray()
        self.string_table = {}
        self.var_table = {}
        self.func_table = {}      # func_name -> arg_count
        self.label_counter = 0
        self.has_driver_entry = False
        self.next_var_addr = 0
        self.driver_name = "driver"
        self.driver_type = 4
        self.driver_priority = 5

    def error(self, msg, line_num=None):
        loc = f" on line {line_num}" if line_num else ""
        print(f"\n[Compiler Error]{loc}: {msg}\n", file=sys.stderr)
        sys.exit(1)

    def resolve_syscall(self, name):
        if name in SYSCALLS:
            return name
        if name.startswith("sys_") and name[4:] in SYSCALLS:
            return name[4:]
        return None

    def new_label(self):
        self.label_counter += 1
        return f"_L{self.label_counter}"
    
    def add_string(self, text_raw):
        text = bytes(text_raw, "utf-8").decode("unicode_escape")
        if text in self.string_table:
            return self.string_table[text]
        offset = len(self.data_bytes)
        raw = text.encode('utf-8')
        self.data_bytes.extend(raw)
        self.string_table[text] = (offset, len(raw))
        return offset, len(raw)
    
    def emit(self, line):
        self.asm_lines.append(line)

    def emit_load_var(self, addr):
        self.emit(f"push {addr+3}")
        self.emit("load8")
        self.emit("push 256")
        self.emit("mul")
        self.emit(f"push {addr+2}")
        self.emit("load8")
        self.emit("add")
        self.emit("push 256")
        self.emit("mul")
        self.emit(f"push {addr+1}")
        self.emit("load8")
        self.emit("add")
        self.emit("push 256")
        self.emit("mul")
        self.emit(f"push {addr}")
        self.emit("load8")
        self.emit("add")

    def emit_store_var(self, addr):
        self.emit("dup")
        self.emit(f"push {addr}")
        self.emit("store8")
        
        self.emit("push 256")
        self.emit("div")
        self.emit("dup")
        self.emit(f"push {addr+1}")
        self.emit("store8")
        
        self.emit("push 256")
        self.emit("div")
        self.emit("dup")
        self.emit(f"push {addr+2}")
        self.emit("store8")
        
        self.emit("push 256")
        self.emit("div")
        self.emit(f"push {addr+3}")
        self.emit("store8")

    def validate_arg_count(self, func_name, args, line_num):
        count = len(args)
        sc_name = self.resolve_syscall(func_name)
        if sc_name:
            expected = SYSCALL_ARGS.get(sc_name, None)
            if expected is not None:
                if isinstance(expected, tuple) and count not in expected:
                    self.error(f"Syscall '{func_name}' expects {expected} arguments, but got {count}", line_num)
                elif isinstance(expected, int) and count != expected:
                    self.error(f"Syscall '{func_name}' expects {expected} argument(s), but got {count}", line_num)
        elif func_name in self.func_table:
            expected = self.func_table[func_name]
            if count != expected:
                self.error(f"Function '{func_name}' expects {expected} argument(s), but got {count}", line_num)

    def emit_syscall_call(self, sc_name, args, line_num):
        if sc_name == "beep":
            if len(args) == 0:
                self.emit("push 100") # duration
                self.emit("push 440") # freq
            elif len(args) == 1:
                self.emit("push 100") # duration
                self.compile_expr(args[0], line_num) # freq
            elif len(args) == 2:
                self.compile_expr(args[1], line_num) # duration
                self.compile_expr(args[0], line_num) # freq
            self.emit(f"syscall {SYSCALLS[sc_name]}")
        else:
            for arg in reversed(args):
                self.compile_expr(arg, line_num)
            self.emit(f"syscall {SYSCALLS[sc_name]}")

    def compile_expr(self, expr, line_num):
        expr = expr.strip()
        if not expr:
            return
        
        m_op = re.match(r'^(.*?)\s*(\+|\-|\*|\/|==|<|>)\s*(.*)$', expr)
        if m_op:
            left, op, right = m_op.group(1), m_op.group(2), m_op.group(3)
            self.compile_expr(left, line_num)
            self.compile_expr(right, line_num)
            op_map = {'+': 'add', '-': 'sub', '*': 'mul', '/': 'div', '==': 'eq', '<': 'lt', '>': 'gt'}
            if op in op_map:
                self.emit(op_map[op])
            return

        if expr.isdigit() or (expr.startswith('-') and expr[1:].isdigit()):
            self.emit(f"push {expr}")
        elif expr.startswith('"') and expr.endswith('"'):
            off, length = self.add_string(expr[1:-1])
            self.emit(f"push {off}")
        elif '(' in expr and expr.endswith(')'):
            m_fn = re.match(r'^([a-zA-Z_]\w*)\s*\((.*)\)$', expr)
            if m_fn:
                func_name, args_str = m_fn.group(1), m_fn.group(2)
                args = [a.strip() for a in args_str.split(',') if a.strip()]
                
                sc_name = self.resolve_syscall(func_name)
                if not sc_name and func_name not in self.func_table:
                    self.error(f"Undefined function or syscall '{func_name}'", line_num)
                
                self.validate_arg_count(func_name, args, line_num)

                if sc_name:
                    self.emit_syscall_call(sc_name, args, line_num)
                elif func_name in self.func_table:
                    for arg in reversed(args):
                        self.compile_expr(arg, line_num)
                    self.emit(f"call {func_name}")
        elif re.match(r'^[a-zA-Z_]\w*$', expr):
            if expr in self.var_table:
                addr = self.var_table[expr]
                self.emit_load_var(addr)
            else:
                self.error(f"Undefined variable '{expr}'", line_num)
        else:
            self.error(f"Syntax error or invalid expression '{expr}'", line_num)

    def compile_stmts(self, lines, i):
        while i < len(lines):
            raw_line = lines[i]
            line_num = i + 1
            line = raw_line.split("//")[0].strip()
            line_clean = line.rstrip(';').strip()
            
            if not line_clean:
                i += 1
                continue
            
            # Xử lý dọn ngoặc C-style
            if line_clean == '}':
                return i

            if line_clean.startswith('}'):
                line_clean = line_clean[1:].strip()

            if line_clean.endswith('{'):
                line_clean = line_clean[:-1].strip()

            if not line_clean or line_clean == '{':
                i += 1
                continue
            
            if line_clean.startswith("return"):
                ret_expr = line_clean[6:].strip()
                if ret_expr:
                    self.compile_expr(ret_expr, line_num)
                self.emit("ret")
                i += 1
                continue

            # 1. Cấu trúc IF
            m_if = re.match(r'^if\s*\(([^)]+)\)', line_clean)
            if m_if:
                cond = m_if.group(1).strip()
                else_label = self.new_label()
                end_label = self.new_label()
                
                self.compile_expr(cond, line_num)
                self.emit(f"jz {else_label}")
                
                i = self.compile_stmts(lines, i + 1)
                self.emit(f"jmp {end_label}")
                self.emit(f"{else_label}:")
                
                if i + 1 < len(lines):
                    next_l = lines[i+1].split("//")[0].strip()
                    if next_l.startswith('else') or next_l.startswith('} else'):
                        i += 1
                        i = self.compile_stmts(lines, i + 1)
                
                self.emit(f"{end_label}:")
                i += 1
                continue
            
            # 2. Cấu trúc WHILE
            m_while = re.match(r'^while\s*\(([^)]+)\)', line_clean)
            if m_while:
                cond = m_while.group(1).strip()
                start_label = self.new_label()
                end_label = self.new_label()
                self.emit(f"{start_label}:")
                
                self.compile_expr(cond, line_num)
                self.emit(f"jz {end_label}")
                
                i = self.compile_stmts(lines, i + 1)
                self.emit(f"jmp {start_label}")
                self.emit(f"{end_label}:")
                i += 1
                continue

            # 3. Gọi hàm hoặc syscall dạng câu lệnh
            m_call = re.match(r'^([a-zA-Z_]\w*)\s*\((.*)\)$', line_clean)
            if m_call:
                func_name, args_str = m_call.group(1), m_call.group(2)
                args = [a.strip() for a in args_str.split(',') if a.strip()]
                
                sc_name = self.resolve_syscall(func_name)
                if not sc_name and func_name not in self.func_table:
                    self.error(f"Undefined function or syscall '{func_name}'", line_num)
                
                self.validate_arg_count(func_name, args, line_num)
                
                if sc_name:
                    self.emit_syscall_call(sc_name, args, line_num)
                    if sc_name in SYSCALLS_WITH_RET:
                        self.emit("pop") # FIXED: Pop bỏ giá trị trả về nếu không dùng trong gán
                elif func_name in self.func_table:
                    for arg in reversed(args):
                        self.compile_expr(arg, line_num)
                    self.emit(f"call {func_name}")
                i += 1
                continue
            
            # 4. Lệnh print
            m_print = re.match(r'^print\s+"([^"]*)"', line_clean)
            if m_print:
                off, length = self.add_string(m_print.group(1))
                self.emit(f"push {off}")
                self.emit(f"push {length}")
                self.emit(f"syscall {SYSCALLS['dbgserial']}")
                i += 1
                continue
            
            # 5. Phép gán biến
            m_assign = re.match(r'^(?:let|var|int)?\s*([a-zA-Z_]\w*)\s*=(?!=)\s*(.*)$', line_clean)
            if m_assign:
                vname, expr = m_assign.groups()
                vname, expr = vname.strip(), expr.strip()
                
                self.compile_expr(expr, line_num)
                
                if vname not in self.var_table:
                    self.var_table[vname] = self.next_var_addr
                    self.next_var_addr += 4
                
                addr = self.var_table[vname]
                self.emit_store_var(addr)
                i += 1
                continue
                
            self.error(f"Unrecognized statement '{line_clean}'", line_num)
            i += 1
        return i

    def compile(self, src):
        lines = src.splitlines()
        
        # Pass 1: Parse header directives
        for i, line in enumerate(lines):
            line_clean = line.split("//")[0].strip()
            m = re.search(r'@driver\s+(\w+)\s+(\w+)\s+(\d+)', line_clean)
            if m:
                self.driver_name = m.group(1)
                self.driver_type = DRIVER_TYPES.get(m.group(2).lower(), 4)
                self.driver_priority = int(m.group(3))
        
        # Pass 2: Thu thập toàn bộ hàm định nghĩa trong toàn bộ mã nguồn
        for i, line in enumerate(lines):
            line_clean = line.split("//")[0].strip()
            m = re.match(r'\b(func|fn)\s+([a-zA-Z_]\w*)\s*\(([^)]*)\)', line_clean)
            if m:
                fname = m.group(2)
                params_str = m.group(3).strip()
                params = [p.strip() for p in params_str.split(',') if p.strip()]
                self.func_table[fname] = len(params)

        # Pass 3: Thu thập chuỗi ký tự
        for line in lines:
            line_clean = line.split("//")[0].strip()
            for m in re.finditer(r'print\s+"([^"]*)"', line_clean):
                self.add_string(m.group(1))

        self.next_var_addr = (len(self.data_bytes) + 3) & ~3
        self.asm_lines = ["jmp DriverEntry"]
        
        # Pass 4: Biên dịch mã nguồn chính
        i = 0
        while i < len(lines):
            line = lines[i].split("//")[0].strip()
            if not line:
                i += 1
                continue
            
            match = re.match(r'\b(func|fn)\s+([a-zA-Z_]\w*)\s*\(([^)]*)\)', line)
            if match:
                func_name = match.group(2)
                
                loop_label = None
                if func_name == "DriverEntry":
                    self.emit("DriverEntry:")
                    self.has_driver_entry = True
                    
                    name_off, name_len = self.add_string(self.driver_name)
                    self.emit(f"push {self.driver_priority}")
                    self.emit(f"push {self.driver_type}")
                    self.emit(f"push {name_len}")
                    self.emit(f"push {name_off}")
                    self.emit("syscall 30")  # driver_register
                    self.emit("pop")
                    
                    loop_label = self.new_label()
                    self.emit(f"{loop_label}:")
                else:
                    self.emit(f"{func_name}:")
                
                params_str = match.group(3).strip()
                params = [p.strip() for p in params_str.split(',') if p.strip()]
                for param in params:
                    if param not in self.var_table:
                        self.var_table[param] = self.next_var_addr
                        self.next_var_addr += 4
                    addr = self.var_table[param]
                    self.emit_store_var(addr)

                if i + 1 < len(lines) and lines[i+1].split("//")[0].strip() == '{':
                    i += 1
                    
                i = self.compile_stmts(lines, i + 1)
                
                if func_name == "DriverEntry":
                    self.emit(f"jmp {loop_label}")
                
                i += 1
                continue
            i += 1
        
        if not self.has_driver_entry:
            self.error("Missing mandatory function 'DriverEntry()'")
        
        return "\n".join(self.asm_lines), bytes(self.data_bytes)


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
        if mnem not in OPS:
            raise Exception(f"Unknown opcode: {mnem}")
        opcode, arg_kind = OPS[mnem]
        size = 1 + (4 if arg_kind in ("i32", "label") else (1 if arg_kind == "u8" else 0))
        instrs.append((pc, mnem, parts[1:], opcode, arg_kind))
        pc += size
    
    out = bytearray()
    for pc, mnem, args, opcode, arg_kind in instrs:
        out.append(opcode)
        if arg_kind == "i32":
            try:
                out += struct.pack("<i", int(args[0]))
            except ValueError:
                if args[0] in labels:
                    out += struct.pack("<i", labels[args[0]])
                else:
                    raise Exception(f"Invalid integer or label: {args[0]}")
        elif arg_kind == "u8":
            out.append(int(args[0]))
        elif arg_kind == "label":
            if args[0] in labels:
                out += struct.pack("<i", labels[args[0]])
            else:
                raise Exception(f"Undefined label: {args[0]}")
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
    parser = argparse.ArgumentParser(description="Yanase Driver Compiler (YSDC)")
    parser.add_argument("input", help="File mã nguồn chính (.yd)")
    parser.add_argument("output", help="File driver đầu ra (.drv)")
    parser.add_argument("-I", "--include", action="append", default=[], help="Thư mục chứa thư viện import")

    args = parser.parse_args()

    preprocessor = Preprocessor(include_dirs=args.include)
    
    try:
        full_src = preprocessor.process_file(args.input)
    except Exception as e:
        print(f"\n[Preprocessor Error]: {e}\n", file=sys.stderr)
        sys.exit(1)

    compiler = YanaseDriverCompiler()
    asm_text, data_bytes = compiler.compile(full_src)
    bytecode = assemble(asm_text)
    
    output = args.output
    if not output.endswith('.drv'):
        output = output.rsplit('.', 1)[0] + '.drv'
    
    pack_tar(output, bytecode, data_bytes)
    print(f"[+] Driver Build Successful! Output: {output}")


if __name__ == "__main__":
    main()
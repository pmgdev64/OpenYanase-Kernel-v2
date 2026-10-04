// src/stdlib.rs

pub struct StdFn {
    pub qualified_name: &'static str,
    pub sys_id: u16,
    pub argc: u8,
}

pub const STDLIB: &[StdFn] = &[
    // --- IO ---
    StdFn { qualified_name: "io.write_line",    sys_id: 55,  argc: 1 },
    StdFn { qualified_name: "io.println",       sys_id: 55,  argc: 1 },
    StdFn { qualified_name: "io.write_str",     sys_id: 1,   argc: 1 },
    StdFn { qualified_name: "io.write",         sys_id: 1,   argc: 1 },
    StdFn { qualified_name: "io.write_int",     sys_id: 1,   argc: 1 },
    StdFn { qualified_name: "io.write_char",    sys_id: 54,  argc: 1 },
    StdFn { qualified_name: "io.read_char",     sys_id: 50,  argc: 0 },
    StdFn { qualified_name: "io.read_key",      sys_id: 50,  argc: 0 },
    StdFn { qualified_name: "io.wait_key",      sys_id: 56,  argc: 0 },
    StdFn { qualified_name: "io.read_line",     sys_id: 50,  argc: 0 },
    StdFn { qualified_name: "io.write_port",    sys_id: 100, argc: 2 },
    StdFn { qualified_name: "io.read_port",     sys_id: 101, argc: 1 },

    // --- CONSOLE ---
    StdFn { qualified_name: "console.clear",     sys_id: 53,  argc: 0 },
    StdFn { qualified_name: "console.backspace", sys_id: 57,  argc: 0 },

    // --- GRAPHICS ---
    StdFn { qualified_name: "gfx.init",             sys_id: 28,  argc: 0 },
    StdFn { qualified_name: "gfx.paint_block",      sys_id: 2,   argc: 5 },
    StdFn { qualified_name: "gfx.clear",            sys_id: 20,  argc: 2 },
    StdFn { qualified_name: "gfx.draw_pixel",       sys_id: 21,  argc: 3 },
    StdFn { qualified_name: "gfx.draw_line",        sys_id: 22,  argc: 5 },
    StdFn { qualified_name: "gfx.draw_rect",        sys_id: 2,   argc: 5 },
    StdFn { qualified_name: "gfx.fill_rect",        sys_id: 2,   argc: 5 },
    StdFn { qualified_name: "gfx.draw_char",        sys_id: 25,  argc: 4 },
    StdFn { qualified_name: "gfx.draw_str",         sys_id: 26,  argc: 4 },
    StdFn { qualified_name: "gfx.draw_text",        sys_id: 26,  argc: 4 },
    StdFn { qualified_name: "gfx.flush",            sys_id: 27,  argc: 0 },

    // --- MOUSE ---
    StdFn { qualified_name: "gfx.get_mouse_x",      sys_id: 40,  argc: 0 },
    StdFn { qualified_name: "gfx.get_mouse_y",      sys_id: 41,  argc: 0 },
    StdFn { qualified_name: "gfx.is_mouse_pressed", sys_id: 42,  argc: 0 },
    StdFn { qualified_name: "gfx.draw_cursor",      sys_id: 43,  argc: 0 },

    // --- SCREEN ---
    StdFn { qualified_name: "screen.width",    sys_id: 9,   argc: 0 },
    StdFn { qualified_name: "screen.height",   sys_id: 10,  argc: 0 },
    StdFn { qualified_name: "screen.flush",    sys_id: 27,  argc: 0 },

    // --- TIME ---
    StdFn { qualified_name: "time.now_ticks",  sys_id: 3,   argc: 0 },
    StdFn { qualified_name: "time.now_secs",   sys_id: 7,   argc: 0 },
    StdFn { qualified_name: "time.sleep_ms",   sys_id: 4,   argc: 1 },
    StdFn { qualified_name: "time.sleep",      sys_id: 4,   argc: 1 },

    // --- PROCESS / PROC ---
    StdFn { qualified_name: "proc.pause_for",      sys_id: 4,   argc: 1 },
    StdFn { qualified_name: "proc.terminate",      sys_id: 5,   argc: 1 },
    StdFn { qualified_name: "proc.self_id",        sys_id: 6,   argc: 0 },
    StdFn { qualified_name: "process.get_pid",     sys_id: 6,   argc: 0 },
    StdFn { qualified_name: "process.get_uptime",  sys_id: 3,   argc: 0 },
    StdFn { qualified_name: "process.get_memory",  sys_id: 105, argc: 0 },
    StdFn { qualified_name: "process.run",         sys_id: 106, argc: 1 },
    StdFn { qualified_name: "process.exit",        sys_id: 5,   argc: 1 },

    // --- FILESYSTEM ---
    StdFn { qualified_name: "fs.list_dir",     sys_id: 108, argc: 1 },
    StdFn { qualified_name: "fs.read_file",    sys_id: 109, argc: 1 },
    StdFn { qualified_name: "fs.cd",           sys_id: 110, argc: 1 },
    StdFn { qualified_name: "fs.get_cwd",      sys_id: 111, argc: 0 },
    StdFn { qualified_name: "fs.is_dir",       sys_id: 112, argc: 1 },

    // --- IPC ---
    StdFn { qualified_name: "ipc.create_port", sys_id: 120, argc: 2 },
    StdFn { qualified_name: "ipc.send",        sys_id: 121, argc: 4 },
    StdFn { qualified_name: "ipc.recv",        sys_id: 122, argc: 2 },

    // --- SYSTEM ---
    StdFn { qualified_name: "system.exit",      sys_id: 5,   argc: 1 },
    StdFn { qualified_name: "system.pid",       sys_id: 6,   argc: 0 },
    StdFn { qualified_name: "system.width",     sys_id: 9,   argc: 0 },
    StdFn { qualified_name: "system.height",    sys_id: 10,  argc: 0 },
    StdFn { qualified_name: "system.beep",      sys_id: 8,   argc: 1 },
    StdFn { qualified_name: "system.exec",      sys_id: 52,  argc: 1 },
    StdFn { qualified_name: "system.exec_arr",  sys_id: 58,  argc: 2 },
    StdFn { qualified_name: "system.reboot",    sys_id: 114, argc: 0 },
    StdFn { qualified_name: "system.shutdown",  sys_id: 113, argc: 0 },
];

pub fn resolve(qualified_name: &str) -> Option<&'static StdFn> {
    STDLIB.iter().find(|f| f.qualified_name == qualified_name)
}
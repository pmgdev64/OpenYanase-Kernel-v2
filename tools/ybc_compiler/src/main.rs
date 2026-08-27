mod lexer;
mod ast;
mod parser;
mod module;
mod resolver;
mod codegen;

use std::path::PathBuf;
use module::ModuleLoader;
use resolver::resolve_all;
use codegen::Codegen;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: ybcc <entry.yl> <output.ybc> [--include=dir1,dir2]");
        std::process::exit(1);
    }

    let entry_file = PathBuf::from(&args[1]);
    let output_file = PathBuf::from(&args[2]);

    let mut search_roots = vec![entry_file.parent().unwrap_or(&PathBuf::from(".")).to_path_buf()];
    for arg in &args[3..] {
        if let Some(dirs) = arg.strip_prefix("--include=") {
            for d in dirs.split(',') {
                search_roots.push(PathBuf::from(d));
            }
        }
    }

    let mut loader = ModuleLoader::new(search_roots);

    let entry_stem = entry_file.file_stem().unwrap().to_string_lossy().to_string();
    let entry_src = std::fs::read_to_string(&entry_file).expect("cannot read entry file");
    let toks = lexer::Lexer::new(&entry_src).tokenize();
    let entry_module = parser::Parser::new(toks).parse_module();

    // Load đệ quy mọi import trong entry module — cho phép module nằm rải rác qua search_roots
    for item in &entry_module.items {
        if let ast::Item::Import(imp) = item {
            loader.load(&imp.path).expect("import resolution failed");
        }
    }

    loader.loaded.insert(entry_stem.clone(), entry_module);

    let resolved = resolve_all(&loader.loaded);

    let entry_fn = resolved.functions.get("main")
        .expect("entry module must define fn main()");

    let mut cg = Codegen::new();
    let bytes = cg.compile_entry(entry_fn, &resolved);

    std::fs::write(&output_file, &bytes).expect("cannot write output .ybc");
    println!("Compiled {} -> {} ({} bytes)", entry_file.display(), output_file.display(), bytes.len());
}
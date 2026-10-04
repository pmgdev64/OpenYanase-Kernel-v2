// src/main.rs
mod lexer;
mod ast;
mod parser;
mod module;
mod resolver;
mod codegen;
mod tar_writer;
mod stdlib;

use std::path::PathBuf;
use module::ModuleLoader;
use resolver::resolve_all;
use codegen::Codegen;
use tar_writer::TarWriter;

struct Args {
    entry_file: PathBuf,
    output_file: PathBuf,
    include_dirs: Vec<PathBuf>,
    pack: bool,
    app_name: Option<String>,
}

fn parse_args() -> Args {
    let raw: Vec<String> = std::env::args().collect();
    if raw.len() < 3 {
        eprintln!("Usage: ybcc <entry.yl> <output> [--include=dir1,dir2] [-pack] [--name=AppName]");
        eprintln!();
        eprintln!("  Without -pack: output là file .ybc thô");
        eprintln!("  With -pack:    output là file .abp (tar chứa manifest.txt + main.ybc)");
        std::process::exit(1);
    }

    let entry_file = PathBuf::from(&raw[1]);
    let output_file = PathBuf::from(&raw[2]);
    let mut include_dirs = vec![
        entry_file.parent().unwrap_or(&PathBuf::from(".")).to_path_buf()
    ];
    let mut pack = false;
    let mut app_name = None;

    for arg in &raw[3..] {
        if arg == "-pack" {
            pack = true;
        } else if let Some(dirs) = arg.strip_prefix("--include=") {
            for d in dirs.split(',') {
                include_dirs.push(PathBuf::from(d));
            }
        } else if let Some(name) = arg.strip_prefix("--name=") {
            app_name = Some(name.to_string());
        } else {
            eprintln!("Unknown argument: {}", arg);
            std::process::exit(1);
        }
    }

    Args { entry_file, output_file, include_dirs, pack, app_name }
}

fn compile_to_ybc(args: &Args) -> Vec<u8> {
    let mut loader = ModuleLoader::new(args.include_dirs.clone());

    let entry_stem = args.entry_file.file_stem().unwrap().to_string_lossy().to_string();
    let entry_src = std::fs::read_to_string(&args.entry_file)
        .unwrap_or_else(|e| { eprintln!("Cannot read entry file: {}", e); std::process::exit(1); });

    let toks = lexer::Lexer::new(&entry_src).tokenize();
    let entry_module = parser::Parser::new(toks).parse_module();

    for item in &entry_module.items {
        if let ast::Item::Import(imp) = item {
            if let Err(e) = loader.load(&imp.path) {
                eprintln!("Import error: {}", e);
                std::process::exit(1);
            }
        }
    }

    loader.loaded.insert(entry_stem.clone(), entry_module);

    let resolved = resolve_all(&loader.loaded);

    let entry_fn = resolved.functions.get("main")
        .unwrap_or_else(|| { eprintln!("Entry module must define fn main()"); std::process::exit(1); });

    let mut cg = Codegen::new();
    cg.compile_entry(entry_fn, &resolved)
}

fn main() {
    let args = parse_args();

    let ybc_bytes = compile_to_ybc(&args);

    if !args.pack {
        std::fs::write(&args.output_file, &ybc_bytes)
            .unwrap_or_else(|e| { eprintln!("Cannot write output: {}", e); std::process::exit(1); });
        println!("Compiled {} -> {} ({} bytes)",
            args.entry_file.display(), args.output_file.display(), ybc_bytes.len());
        return;
    }

    let app_name = args.app_name.clone().unwrap_or_else(|| {
        args.entry_file.file_stem().unwrap().to_string_lossy().to_string()
    });

    let manifest = format!("name={}\nentry=main.ybc\nheap=65536\n", app_name);

    let mut tar = TarWriter::new();
    tar.add_file("manifest.txt", manifest.as_bytes());
    tar.add_file("main.ybc", &ybc_bytes);
    let abp_bytes = tar.finish();

    let mut out_path = args.output_file.clone();
    if out_path.extension().map_or(true, |e| e != "abp") {
        out_path.set_extension("abp");
    }

    std::fs::write(&out_path, &abp_bytes)
        .unwrap_or_else(|e| { eprintln!("Cannot write .abp: {}", e); std::process::exit(1); });

    println!("Packed {} -> {}", args.entry_file.display(), out_path.display());
    println!("  App name: {}", app_name);
    println!("  Bytecode: {} bytes", ybc_bytes.len());
    println!("  Package:  {} bytes", abp_bytes.len());
}
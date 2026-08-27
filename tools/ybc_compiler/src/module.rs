use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::ast::Module;
use crate::lexer::Lexer;
use crate::parser::Parser;

pub struct ModuleLoader {
    pub search_roots: Vec<PathBuf>,
    pub loaded: HashMap<String, Module>, // key = "a.b.c" path canonical
}

impl ModuleLoader {
    pub fn new(search_roots: Vec<PathBuf>) -> Self {
        Self { search_roots, loaded: HashMap::new() }
    }

    /// path = ["utils", "math"] -> tìm utils/math.yl trong mọi search_root
    pub fn resolve_path(&self, path: &[String]) -> Option<PathBuf> {
        let rel: PathBuf = path.iter().collect();
        for root in &self.search_roots {
            let candidate = root.join(&rel).with_extension("yl");
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    /// Load module (và đệ quy load mọi import bên trong nó), tránh load lặp
    pub fn load(&mut self, path: &[String]) -> Result<String, String> {
        let key = path.join(".");
        if self.loaded.contains_key(&key) {
            return Ok(key);
        }

        let file_path = self.resolve_path(path)
            .ok_or_else(|| format!("Module not found: {}", key))?;

        let src = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("Cannot read {}: {}", file_path.display(), e))?;

        let toks = Lexer::new(&src).tokenize();
        let module = Parser::new(toks).parse_module();

        // Load trước tất cả import bên trong module này (đệ quy)
        for item in &module.items {
            if let crate::ast::Item::Import(imp) = item {
                self.load(&imp.path)?;
            }
        }

        self.loaded.insert(key.clone(), module);
        Ok(key)
    }
}
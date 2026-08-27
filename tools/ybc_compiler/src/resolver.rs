use std::collections::HashMap;
use crate::ast::*;

pub struct ResolvedProgram {
    pub functions: HashMap<String, FnDecl>,   // key: "modpath::fnname" hoặc "ClassName::method"
    pub classes: HashMap<String, ClassDecl>,  // đã merge field/method từ parent
}

pub fn resolve_all(modules: &HashMap<String, Module>) -> ResolvedProgram {
    let mut raw_classes: HashMap<String, ClassDecl> = HashMap::new();
    let mut functions: HashMap<String, FnDecl> = HashMap::new();

    for (modname, module) in modules {
        for item in &module.items {
            match item {
                Item::Class(c) => {
                    raw_classes.insert(c.name.clone(), c.clone());
                }
                Item::Fn(f) => {
                    let key = format!("{}::{}", modname, f.name);
                    functions.insert(key, f.clone());
                    // Cũng đăng ký tên ngắn để gọi trực tiếp trong cùng module không cần prefix
                    functions.entry(f.name.clone()).or_insert(f.clone());
                }
                Item::Import(_) => {}
            }
        }
    }

    // Merge class-children: đệ quy lên chain "extends" để gộp field + method của cha
    let mut merged: HashMap<String, ClassDecl> = HashMap::new();
    let names: Vec<String> = raw_classes.keys().cloned().collect();
    for name in names {
        let resolved = merge_class_chain(&name, &raw_classes, &mut merged);
        merged.insert(name, resolved);
    }

    ResolvedProgram { functions, classes: merged }
}

fn merge_class_chain(
    name: &str,
    raw: &HashMap<String, ClassDecl>,
    cache: &mut HashMap<String, ClassDecl>,
) -> ClassDecl {
    if let Some(c) = cache.get(name) {
        return c.clone();
    }

    let this_class = raw.get(name)
        .unwrap_or_else(|| panic!("Class not found: {}", name))
        .clone();

    let merged = match &this_class.parent {
        None => this_class,
        Some(parent_name) => {
            let parent_resolved = merge_class_chain(parent_name, raw, cache);

            // Field: cha trước, con sau (con không trùng tên với cha)
            let mut fields = parent_resolved.fields.clone();
            for f in &this_class.fields {
                if !fields.contains(f) {
                    fields.push(f.clone());
                }
            }

            // Method: override theo tên — nếu con định nghĩa lại method cùng tên cha, dùng bản con
            let mut methods = parent_resolved.methods.clone();
            for m in &this_class.methods {
                if let Some(existing) = methods.iter_mut().find(|em| em.name == m.name) {
                    *existing = m.clone();
                } else {
                    methods.push(m.clone());
                }
            }

            ClassDecl {
                name: this_class.name.clone(),
                parent: this_class.parent.clone(),
                fields,
                methods,
            }
        }
    };

    cache.insert(name.to_string(), merged.clone());
    merged
}
use std::collections::HashMap;
use crate::ast::*;
use crate::stdlib;

pub struct ResolvedProgram {
    pub functions: HashMap<String, FnDecl>, // key: "pkg.pkg.FnName"
    pub classes: HashMap<String, ClassDecl>,
}

pub fn resolve_all(modules: &HashMap<String, Module>) -> ResolvedProgram {
    let mut raw_classes: HashMap<String, ClassDecl> = HashMap::new();
    let mut functions: HashMap<String, FnDecl> = HashMap::new();

    for (modname, module) in modules {
        let pkg_prefix = module.package.as_ref()
            .map(|p| p.path.join("."))
            .unwrap_or_else(|| modname.clone());

        for item in &module.items {
            match item {
                Item::Class(c) => {
                    let qualified = format!("{}.{}", pkg_prefix, c.name);
                    raw_classes.insert(qualified.clone(), c.clone());
                    raw_classes.entry(c.name.clone()).or_insert(c.clone());
                }
                Item::Fn(f) => {
                    let qualified = format!("{}.{}", pkg_prefix, f.name);
                    functions.insert(qualified, f.clone());
                    functions.entry(f.name.clone()).or_insert(f.clone());
                }
                Item::Import(_) => {}
            }
        }
    }

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
    if let Some(c) = cache.get(name) { return c.clone(); }

    let this_class = raw.get(name)
        .unwrap_or_else(|| panic!("Class not found: {}", name)).clone();

    let merged = match &this_class.parent {
        None => this_class,
        Some(parent_name) => {
            let parent_resolved = merge_class_chain(parent_name, raw, cache);
            let mut fields = parent_resolved.fields.clone();
            for f in &this_class.fields {
                if !fields.contains(f) { fields.push(f.clone()); }
            }
            let mut methods = parent_resolved.methods.clone();
            for m in &this_class.methods {
                if let Some(e) = methods.iter_mut().find(|em| em.name == m.name) {
                    *e = m.clone();
                } else {
                    methods.push(m.clone());
                }
            }
            ClassDecl { name: this_class.name.clone(), parent: this_class.parent.clone(), fields, methods }
        }
    };
    cache.insert(name.to_string(), merged.clone());
    merged
}
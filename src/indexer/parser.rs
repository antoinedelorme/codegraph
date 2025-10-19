// Language parsers

use std::collections::HashMap;
use tree_sitter::{Parser as TreeParser, Tree};

use crate::index::{Location, Parser, Relationship, RelationshipKind, Symbol, SymbolKind, Visibility};

/// Python parser using tree-sitter
pub struct PythonParser;

// Rust parser using tree-sitter
pub struct RustParser;

// Go parser using tree-sitter
pub struct GoParser;

// Java parser using tree-sitter
pub struct JavaParser;

// Intent parser (basic implementation)
pub struct IntentParser;

impl PythonParser {
    pub fn new() -> Self {
        Self
    }

    fn parse_tree(&self, content: &str) -> anyhow::Result<Tree> {
        let mut parser = TreeParser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into())?;

        let tree = parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Python code"))?;

        Ok(tree)
    }

    fn extract_symbols(&self, tree: &Tree, content: &str, file_path: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let root = tree.root_node();

        // Walk the tree to find symbols
        let mut cursor = root.walk();
        self.walk_tree(&mut cursor, content, file_path, &mut symbols, Vec::new());

        symbols
    }

    fn walk_tree(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        scope_stack: Vec<String>,
    ) {
        let node = cursor.node();

        match node.kind() {
            "function_definition" => {
                if let Some(symbol) = self.extract_function(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "class_definition" => {
                if let Some(symbol) = self.extract_class(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "assignment" => {
                if let Some(symbol) = self.extract_variable(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "import_statement" | "import_from_statement" => {
                if let Some(symbol) = self.extract_import(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            _ => {}
        }

        // Recurse into children
        if cursor.goto_first_child() {
            let mut new_scope = scope_stack.clone();
            if let "class_definition" | "function_definition" = node.kind() {
                if let Some(name) = self.get_node_text(node.child_by_field_name("name"), content) {
                    new_scope.push(name);
                }
            }

            self.walk_tree(cursor, content, file_path, symbols, new_scope);

            while cursor.goto_next_sibling() {
                self.walk_tree(cursor, content, file_path, symbols, scope_stack.clone());
            }

            cursor.goto_parent();
        }
    }

    fn extract_function(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", scope_stack.join("."), name)
        };

        let location = self.node_location(node, file_path);

        // Extract parameters
        let parameters_node = node.child_by_field_name("parameters");
        let mut parameters = Vec::new();
        if let Some(params) = parameters_node {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                if child.kind() == "identifier" {
                    if let Some(param_name) = self.get_node_text(Some(child), content) {
                        parameters.push(param_name);
                    }
                }
            }
        }

        let signature = format!("def {}({})", name, parameters.join(", "));

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Function,
            name,
            qualified_name,
            location,
            signature: Some(signature),
            type_info: None,
            visibility: Visibility::Public, // Python doesn't have visibility
            language: "python".to_string(),
            metadata: serde_json::json!({
                "parameters": parameters
            }),
            content_hash: "".to_string(), // TODO: calculate
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_class(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", scope_stack.join("."), name)
        };

        let location = self.node_location(node, file_path);

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Class,
            name,
            qualified_name,
            location,
            signature: None,
            type_info: None,
            visibility: Visibility::Public,
            language: "python".to_string(),
            metadata: serde_json::json!({}),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_variable(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        // Simple assignment like: x = 1
        let left_node = node.child_by_field_name("left")?;
        if left_node.kind() != "identifier" {
            return None; // Not a simple variable assignment
        }

        let name = self.get_node_text(Some(left_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", scope_stack.join("."), name)
        };

        let location = self.node_location(node, file_path);

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Variable,
            name,
            qualified_name,
            location,
            signature: None,
            type_info: None,
            visibility: Visibility::Public,
            language: "python".to_string(),
            metadata: serde_json::json!({}),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_import(&self, node: tree_sitter::Node, content: &str, file_path: &str, _scope_stack: &[String]) -> Option<Symbol> {
        // For now, just extract the module name
        let name = if node.kind() == "import_statement" {
            node.child_by_field_name("name")?
                .child(0)? // dotted_name
                .child(0)? // identifier
        } else { // import_from_statement
            node.child_by_field_name("module")?
        };

        let module_name = self.get_node_text(Some(name), content)?;

        let location = self.node_location(node, file_path);

        Some(Symbol {
            id: format!("{}:import:{}", file_path, module_name),
            kind: SymbolKind::Import,
            name: module_name.clone(),
            qualified_name: module_name,
            location,
            signature: None,
            type_info: None,
            visibility: Visibility::Public,
            language: "python".to_string(),
            metadata: serde_json::json!({}),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_relationships(&self, tree: &Tree, content: &str, file_path: &str, symbols: &[Symbol]) -> Vec<Relationship> {
        let mut relationships = Vec::new();
        let root = tree.root_node();

        // Create a map of qualified names to symbol IDs for lookup
        let symbol_map: HashMap<&str, &Symbol> = symbols.iter()
            .map(|s| (s.qualified_name.as_str(), s))
            .collect();

        let mut cursor = root.walk();
        self.extract_relationships_from_tree(&mut cursor, content, file_path, &symbol_map, &mut relationships, Vec::new());

        relationships
    }

    fn extract_relationships_from_tree(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        content: &str,
        file_path: &str,
        symbol_map: &HashMap<&str, &Symbol>,
        relationships: &mut Vec<Relationship>,
        context_stack: Vec<String>,
    ) {
        let node = cursor.node();

        match node.kind() {
            "call" => {
                if let Some(rel) = self.extract_call_relationship(node, content, file_path, symbol_map, &context_stack) {
                    relationships.push(rel);
                }
            }
            "attribute" => {
                if let Some(rel) = self.extract_attribute_relationship(node, content, file_path, symbol_map, &context_stack) {
                    relationships.push(rel);
                }
            }
            _ => {}
        }

        // Recurse
        if cursor.goto_first_child() {
            let mut new_context = context_stack.clone();
            if let "class_definition" | "function_definition" = node.kind() {
                if let Some(name) = self.get_node_text(node.child_by_field_name("name"), content) {
                    new_context.push(name);
                }
            }

            self.extract_relationships_from_tree(cursor, content, file_path, symbol_map, relationships, new_context);

            while cursor.goto_next_sibling() {
                let mut sibling_context = context_stack.clone();
                if let "class_definition" | "function_definition" = cursor.node().kind() {
                    if let Some(name) = self.get_node_text(cursor.node().child_by_field_name("name"), content) {
                        sibling_context.push(name);
                    }
                }
                self.extract_relationships_from_tree(cursor, content, file_path, symbol_map, relationships, sibling_context);
            }

            cursor.goto_parent();
        }
    }

    fn extract_call_relationship(
        &self,
        node: tree_sitter::Node,
        content: &str,
        file_path: &str,
        symbol_map: &HashMap<&str, &Symbol>,
        context_stack: &[String],
    ) -> Option<Relationship> {
        let function_node = node.child_by_field_name("function")?;

        let (function_name, is_method_call) = if function_node.kind() == "attribute" {
            // Handle method calls like obj.method()
            let attribute_node = function_node.child_by_field_name("attribute")?;
            let method_name = self.get_node_text(Some(attribute_node), content)?;
            (method_name, true)
        } else {
            // Handle direct function calls
            let function_name = self.get_node_text(Some(function_node), content)?;
            (function_name, false)
        };

        let called_symbol = if is_method_call {
            // For method calls, look for any method with this name
            symbol_map.values()
                .find(|s| s.kind == SymbolKind::Method && s.name == function_name)
        } else {
            // For direct calls, look for functions or classes
            symbol_map.values()
                .find(|s| (s.kind == SymbolKind::Function || s.kind == SymbolKind::Class) &&
                          (s.qualified_name == function_name || s.qualified_name.ends_with(&format!(".{}", function_name))))
        };

        if let Some(called_symbol) = called_symbol {
            // Only create relationship if we have a valid calling context
            if !context_stack.is_empty() {
                let caller_qualified_name = context_stack.join(".");
                if let Some(caller_symbol) = symbol_map.get(caller_qualified_name.as_str()) {
                    let location = self.node_location(node, file_path);

                    return Some(Relationship {
                        from_id: caller_symbol.id.clone(),
                        to_id: called_symbol.id.clone(),
                        kind: RelationshipKind::Calls,
                        location,
                        metadata: serde_json::json!({}),
                    });
                }
            }
        }
        None
    }

    fn extract_attribute_relationship(
        &self,
        node: tree_sitter::Node,
        content: &str,
        file_path: &str,
        symbol_map: &HashMap<&str, &Symbol>,
        context_stack: &[String],
    ) -> Option<Relationship> {
        let object_node = node.child_by_field_name("object")?;
        let attribute_node = node.child_by_field_name("attribute")?;

        let object_name = self.get_node_text(Some(object_node), content)?;
        let attribute_name = self.get_node_text(Some(attribute_node), content)?;

        let qualified_name = format!("{}.{}", object_name, attribute_name);

        if let Some(referenced_symbol) = symbol_map.get(qualified_name.as_str()) {
            // Only create relationship if we have a valid calling context
            if !context_stack.is_empty() {
                let caller_qualified_name = context_stack.join(".");
                if let Some(caller_symbol) = symbol_map.get(caller_qualified_name.as_str()) {
                    let location = self.node_location(node, file_path);

                    return Some(Relationship {
                        from_id: caller_symbol.id.clone(),
                        to_id: referenced_symbol.id.clone(),
                        kind: RelationshipKind::References,
                        location,
                        metadata: serde_json::json!({}),
                    });
                }
            }
        }
        None
    }

    fn get_node_text(&self, node: Option<tree_sitter::Node>, content: &str) -> Option<String> {
        node.map(|n| content[n.byte_range()].to_string())
    }

    fn node_location(&self, node: tree_sitter::Node, file_path: &str) -> Location {
        let start = node.start_position();
        let end = node.end_position();

        Location {
            file: file_path.to_string(),
            line: start.row as u32,
            column: start.column as u32,
            end_line: end.row as u32,
            end_column: end.column as u32,
        }
    }

}

impl crate::index::Parser for PythonParser {
    fn can_parse(&self, file_path: &str) -> bool {
        file_path.ends_with(".py")
    }

    fn parse(&self, content: &str, file_path: &str) -> anyhow::Result<(Vec<Symbol>, Vec<Relationship>)> {
        let tree = self.parse_tree(content)?;
        let symbols = self.extract_symbols(&tree, content, file_path);
        let relationships = self.extract_relationships(&tree, content, file_path, &symbols);

        Ok((symbols, relationships))
    }

    fn extract_relationships_with_global_context(&self, content: &str, file_path: &str, global_symbol_map: &std::collections::HashMap<&str, &Symbol>) -> anyhow::Result<Vec<Relationship>> {
        let tree = self.parse_tree(content)?;
        let mut relationships = Vec::new();
        let root = tree.root_node();

        let mut cursor = root.walk();
        self.extract_relationships_from_tree(&mut cursor, content, file_path, global_symbol_map, &mut relationships, Vec::new());

        Ok(relationships)
    }
}

impl RustParser {
    pub fn new() -> Self {
        Self
    }

    fn parse_tree(&self, content: &str) -> anyhow::Result<Tree> {
        let mut parser = TreeParser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;

        let tree = parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Rust code"))?;

        Ok(tree)
    }

    fn extract_symbols(&self, tree: &Tree, content: &str, file_path: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let root = tree.root_node();

        // Walk the tree to find symbols
        let mut cursor = root.walk();
        self.walk_tree(&mut cursor, content, file_path, &mut symbols, Vec::new());

        symbols
    }

    fn walk_tree(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        scope_stack: Vec<String>,
    ) {
        let node = cursor.node();

        match node.kind() {
            "function_item" => {
                if let Some(symbol) = self.extract_function(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "struct_item" => {
                if let Some(symbol) = self.extract_struct(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "enum_item" => {
                if let Some(symbol) = self.extract_enum(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "impl_item" => {
                if let Some(symbol) = self.extract_impl(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "trait_item" => {
                if let Some(symbol) = self.extract_trait(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "const_item" => {
                if let Some(symbol) = self.extract_const(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "static_item" => {
                if let Some(symbol) = self.extract_static(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            _ => {}
        }

        // Recurse into children
        if cursor.goto_first_child() {
            let mut new_scope = scope_stack.clone();
            if let "impl_item" | "function_item" = node.kind() {
                if let Some(name) = self.get_node_text(node.child_by_field_name("name"), content) {
                    new_scope.push(name);
                }
            }

            self.walk_tree(cursor, content, file_path, symbols, new_scope);

            while cursor.goto_next_sibling() {
                self.walk_tree(cursor, content, file_path, symbols, scope_stack.clone());
            }

            cursor.goto_parent();
        }
    }

    fn extract_function(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", scope_stack.join("::"), name)
        };

        let location = self.node_location(node, file_path);

        // Extract parameters
        let parameters_node = node.child_by_field_name("parameters");
        let mut parameters = Vec::new();
        if let Some(params) = parameters_node {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                if child.kind() == "parameter" {
                    if let Some(param_node) = child.child_by_field_name("pattern") {
                        if let Some(param_name) = self.get_node_text(Some(param_node), content) {
                            parameters.push(param_name);
                        }
                    }
                }
            }
        }

        let signature = format!("fn {}({})", name, parameters.join(", "));

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Function,
            name,
            qualified_name,
            location,
            signature: Some(signature),
            type_info: None,
            visibility: Visibility::Public, // Rust has complex visibility, default to public
            language: "rust".to_string(),
            metadata: serde_json::json!({
                "parameters": parameters
            }),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_struct(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", scope_stack.join("::"), name)
        };

        let location = self.node_location(node, file_path);

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Class, // Rust structs are similar to classes
            name,
            qualified_name,
            location,
            signature: None,
            type_info: None,
            visibility: Visibility::Public,
            language: "rust".to_string(),
            metadata: serde_json::json!({}),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_enum(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", scope_stack.join("::"), name)
        };

        let location = self.node_location(node, file_path);

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Type,
            name,
            qualified_name,
            location,
            signature: None,
            type_info: None,
            visibility: Visibility::Public,
            language: "rust".to_string(),
            metadata: serde_json::json!({}),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_impl(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let type_node = node.child_by_field_name("type")?;
        let type_name = self.get_node_text(Some(type_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            format!("impl {}", type_name)
        } else {
            format!("{}::impl {}", scope_stack.join("::"), type_name)
        };

        let location = self.node_location(node, file_path);

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Context,
            name: format!("impl {}", type_name),
            qualified_name,
            location,
            signature: None,
            type_info: None,
            visibility: Visibility::Public,
            language: "rust".to_string(),
            metadata: serde_json::json!({
                "implements": type_name
            }),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_trait(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", scope_stack.join("::"), name)
        };

        let location = self.node_location(node, file_path);

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Type,
            name,
            qualified_name,
            location,
            signature: None,
            type_info: None,
            visibility: Visibility::Public,
            language: "rust".to_string(),
            metadata: serde_json::json!({}),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_const(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", scope_stack.join("::"), name)
        };

        let location = self.node_location(node, file_path);

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Variable,
            name,
            qualified_name,
            location,
            signature: None,
            type_info: None,
            visibility: Visibility::Public,
            language: "rust".to_string(),
            metadata: serde_json::json!({}),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_static(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", scope_stack.join("::"), name)
        };

        let location = self.node_location(node, file_path);

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Variable,
            name,
            qualified_name,
            location,
            signature: None,
            type_info: None,
            visibility: Visibility::Public,
            language: "rust".to_string(),
            metadata: serde_json::json!({}),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_relationships(&self, tree: &Tree, content: &str, file_path: &str, symbols: &[Symbol]) -> Vec<Relationship> {
        let mut relationships = Vec::new();
        let root = tree.root_node();

        // Create a map of qualified names to symbol IDs for lookup
        let symbol_map: HashMap<&str, &Symbol> = symbols.iter()
            .map(|s| (s.qualified_name.as_str(), s))
            .collect();

        let mut cursor = root.walk();
        self.extract_relationships_from_tree(&mut cursor, content, file_path, &symbol_map, &mut relationships, Vec::new());

        relationships
    }

    fn extract_relationships_from_tree(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        content: &str,
        file_path: &str,
        symbol_map: &HashMap<&str, &Symbol>,
        relationships: &mut Vec<Relationship>,
        context_stack: Vec<String>,
    ) {
        let node = cursor.node();

        match node.kind() {
            "call_expression" => {
                if let Some(rel) = self.extract_call_relationship(node, content, file_path, symbol_map, &context_stack) {
                    relationships.push(rel);
                }
            }
            _ => {}
        }

        // Recurse
        if cursor.goto_first_child() {
            let mut new_context = context_stack.clone();
            if let "function_item" = node.kind() {
                if let Some(name) = self.get_node_text(node.child_by_field_name("name"), content) {
                    new_context.push(name);
                }
            }

            self.extract_relationships_from_tree(cursor, content, file_path, symbol_map, relationships, new_context);

            while cursor.goto_next_sibling() {
                let mut sibling_context = context_stack.clone();
                if let "function_item" = cursor.node().kind() {
                    if let Some(name) = self.get_node_text(cursor.node().child_by_field_name("name"), content) {
                        sibling_context.push(name);
                    }
                }
                self.extract_relationships_from_tree(cursor, content, file_path, symbol_map, relationships, sibling_context);
            }

            cursor.goto_parent();
        }
    }

    fn extract_call_relationship(
        &self,
        node: tree_sitter::Node,
        content: &str,
        file_path: &str,
        symbol_map: &HashMap<&str, &Symbol>,
        context_stack: &[String],
    ) -> Option<Relationship> {
        let function_node = node.child_by_field_name("function")?;
        let function_name = self.get_node_text(Some(function_node), content)?;

        // Find the symbol being called
        let called_symbol = symbol_map.values()
            .find(|s| s.qualified_name == function_name || s.qualified_name.ends_with(&format!("::{}", function_name)));

        if let Some(called_symbol) = called_symbol {
            // Only create relationship if we have a valid calling context
            if !context_stack.is_empty() {
                let caller_qualified_name = context_stack.join("::");
                if let Some(caller_symbol) = symbol_map.get(caller_qualified_name.as_str()) {
                    let location = self.node_location(node, file_path);

                    return Some(Relationship {
                        from_id: caller_symbol.id.clone(),
                        to_id: called_symbol.id.clone(),
                        kind: RelationshipKind::Calls,
                        location,
                        metadata: serde_json::json!({}),
                    });
                }
            }
        }
        None
    }

    fn get_node_text(&self, node: Option<tree_sitter::Node>, content: &str) -> Option<String> {
        node.map(|n| content[n.byte_range()].to_string())
    }

    fn node_location(&self, node: tree_sitter::Node, file_path: &str) -> Location {
        let start = node.start_position();
        let end = node.end_position();

        Location {
            file: file_path.to_string(),
            line: start.row as u32,
            column: start.column as u32,
            end_line: end.row as u32,
            end_column: end.column as u32,
        }
    }
}

impl crate::index::Parser for RustParser {
    fn can_parse(&self, file_path: &str) -> bool {
        file_path.ends_with(".rs")
    }

    fn parse(&self, content: &str, file_path: &str) -> anyhow::Result<(Vec<Symbol>, Vec<Relationship>)> {
        let tree = self.parse_tree(content)?;
        let symbols = self.extract_symbols(&tree, content, file_path);
        let relationships = self.extract_relationships(&tree, content, file_path, &symbols);

        Ok((symbols, relationships))
    }

    fn extract_relationships_with_global_context(&self, content: &str, file_path: &str, global_symbol_map: &std::collections::HashMap<&str, &Symbol>) -> anyhow::Result<Vec<Relationship>> {
        let tree = self.parse_tree(content)?;
        let mut relationships = Vec::new();
        let root = tree.root_node();

        let mut cursor = root.walk();
        self.extract_relationships_from_tree(&mut cursor, content, file_path, global_symbol_map, &mut relationships, Vec::new());

        Ok(relationships)
    }
}

impl GoParser {
    pub fn new() -> Self {
        Self
    }

    fn parse_tree(&self, content: &str) -> anyhow::Result<Tree> {
        let mut parser = TreeParser::new();
        parser.set_language(&tree_sitter_go::LANGUAGE.into())?;

        let tree = parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Go code"))?;

        Ok(tree)
    }

    fn extract_symbols(&self, tree: &Tree, content: &str, file_path: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let root = tree.root_node();

        // Walk the tree to find symbols
        let mut cursor = root.walk();
        self.walk_tree(&mut cursor, content, file_path, &mut symbols, Vec::new());

        symbols
    }

    fn walk_tree(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        scope_stack: Vec<String>,
    ) {
        let node = cursor.node();

        match node.kind() {
            "function_declaration" => {
                if let Some(symbol) = self.extract_function(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "method_declaration" => {
                if let Some(symbol) = self.extract_method(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "type_declaration" => {
                if let Some(symbol) = self.extract_type(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "const_declaration" => {
                let const_symbols = self.extract_const_declaration(node, content, file_path, &scope_stack);
                symbols.extend(const_symbols);
            }
            "var_declaration" => {
                let var_symbols = self.extract_var_declaration(node, content, file_path, &scope_stack);
                symbols.extend(var_symbols);
            }
            _ => {}
        }

        // Recurse into children
        if cursor.goto_first_child() {
            let mut new_scope = scope_stack.clone();
            if let "function_declaration" | "method_declaration" = node.kind() {
                if let Some(name) = self.get_node_text(node.child_by_field_name("name"), content) {
                    new_scope.push(name);
                }
            }

            self.walk_tree(cursor, content, file_path, symbols, new_scope);

            while cursor.goto_next_sibling() {
                self.walk_tree(cursor, content, file_path, symbols, scope_stack.clone());
            }

            cursor.goto_parent();
        }
    }

    fn extract_function(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", scope_stack.join("."), name)
        };

        let location = self.node_location(node, file_path);

        // Extract parameters
        let parameters_node = node.child_by_field_name("parameters");
        let mut parameters = Vec::new();
        if let Some(params) = parameters_node {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                if child.kind() == "parameter_declaration" {
                    if let Some(param_node) = child.child_by_field_name("name") {
                        if let Some(param_name) = self.get_node_text(Some(param_node), content) {
                            parameters.push(param_name);
                        }
                    }
                }
            }
        }

        let signature = format!("func {}({})", name, parameters.join(", "));

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Function,
            name,
            qualified_name,
            location,
            signature: Some(signature),
            type_info: None,
            visibility: Visibility::Public, // Go has package-level visibility
            language: "go".to_string(),
            metadata: serde_json::json!({
                "parameters": parameters
            }),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_method(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        // Get receiver type
        let receiver_node = node.child_by_field_name("receiver")?;
        let receiver_type = self.get_node_text(Some(receiver_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            format!("{}::{}", receiver_type, name)
        } else {
            format!("{}.{}::{}", scope_stack.join("."), receiver_type, name)
        };

        let location = self.node_location(node, file_path);

        // Extract parameters
        let parameters_node = node.child_by_field_name("parameters");
        let mut parameters = Vec::new();
        if let Some(params) = parameters_node {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                if child.kind() == "parameter_declaration" {
                    if let Some(param_node) = child.child_by_field_name("name") {
                        if let Some(param_name) = self.get_node_text(Some(param_node), content) {
                            parameters.push(param_name);
                        }
                    }
                }
            }
        }

        let signature = format!("func ({} {}) {}({})", receiver_node, receiver_type, name, parameters.join(", "));

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Method,
            name,
            qualified_name,
            location,
            signature: Some(signature),
            type_info: Some(receiver_type.clone()),
            visibility: Visibility::Public,
            language: "go".to_string(),
            metadata: serde_json::json!({
                "receiver": receiver_type,
                "parameters": parameters
            }),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_type(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        let type_node = node.child_by_field_name("type")?;
        let type_kind = if type_node.kind() == "struct_type" {
            SymbolKind::Class // Go structs are similar to classes
        } else if type_node.kind() == "interface_type" {
            SymbolKind::Type
        } else {
            SymbolKind::Type
        };

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", scope_stack.join("."), name)
        };

        let location = self.node_location(node, file_path);

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: type_kind,
            name,
            qualified_name,
            location,
            signature: None,
            type_info: None,
            visibility: Visibility::Public,
            language: "go".to_string(),
            metadata: serde_json::json!({}),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_const_declaration(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Vec<Symbol> {
        let mut symbols = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "const_spec" {
                let mut spec_cursor = child.walk();
                for spec_child in child.children(&mut spec_cursor) {
                    if spec_child.kind() == "identifier" {
                        if let Some(name) = self.get_node_text(Some(spec_child), content) {
                            let qualified_name = if scope_stack.is_empty() {
                                name.clone()
                            } else {
                                format!("{}.{}", scope_stack.join("."), name)
                            };

                            let location = self.node_location(spec_child, file_path);

                            symbols.push(Symbol {
                                id: format!("{}:{}", file_path, qualified_name),
                                kind: SymbolKind::Variable,
                                name,
                                qualified_name,
                                location,
                                signature: None,
                                type_info: None,
                                visibility: Visibility::Public,
                                language: "go".to_string(),
                                metadata: serde_json::json!({
                                    "const": true
                                }),
                                content_hash: "".to_string(),
                                last_indexed: chrono::Utc::now().timestamp(),
                            });
                        }
                    }
                }
            }
        }

        symbols
    }

    fn extract_var_declaration(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Vec<Symbol> {
        let mut symbols = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "var_spec" {
                let mut spec_cursor = child.walk();
                for spec_child in child.children(&mut spec_cursor) {
                    if spec_child.kind() == "identifier" {
                        if let Some(name) = self.get_node_text(Some(spec_child), content) {
                            let qualified_name = if scope_stack.is_empty() {
                                name.clone()
                            } else {
                                format!("{}.{}", scope_stack.join("."), name)
                            };

                            let location = self.node_location(spec_child, file_path);

                            symbols.push(Symbol {
                                id: format!("{}:{}", file_path, qualified_name),
                                kind: SymbolKind::Variable,
                                name,
                                qualified_name,
                                location,
                                signature: None,
                                type_info: None,
                                visibility: Visibility::Public,
                                language: "go".to_string(),
                                metadata: serde_json::json!({
                                    "var": true
                                }),
                                content_hash: "".to_string(),
                                last_indexed: chrono::Utc::now().timestamp(),
                            });
                        }
                    }
                }
            }
        }

        symbols
    }

    fn extract_relationships(&self, tree: &Tree, content: &str, file_path: &str, symbols: &[Symbol]) -> Vec<Relationship> {
        let mut relationships = Vec::new();
        let root = tree.root_node();

        // Create a map of qualified names to symbol IDs for lookup
        let symbol_map: HashMap<&str, &Symbol> = symbols.iter()
            .map(|s| (s.qualified_name.as_str(), s))
            .collect();

        let mut cursor = root.walk();
        self.extract_relationships_from_tree(&mut cursor, content, file_path, &symbol_map, &mut relationships, Vec::new());

        relationships
    }

    fn extract_relationships_from_tree(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        content: &str,
        file_path: &str,
        symbol_map: &HashMap<&str, &Symbol>,
        relationships: &mut Vec<Relationship>,
        context_stack: Vec<String>,
    ) {
        let node = cursor.node();

        match node.kind() {
            "call_expression" => {
                if let Some(rel) = self.extract_call_relationship(node, content, file_path, symbol_map, &context_stack) {
                    relationships.push(rel);
                }
            }
            _ => {}
        }

        // Recurse
        if cursor.goto_first_child() {
            let mut new_context = context_stack.clone();
            if let "function_declaration" | "method_declaration" = node.kind() {
                if let Some(name) = self.get_node_text(node.child_by_field_name("name"), content) {
                    new_context.push(name);
                }
            }

            self.extract_relationships_from_tree(cursor, content, file_path, symbol_map, relationships, new_context);

            while cursor.goto_next_sibling() {
                let mut sibling_context = context_stack.clone();
                if let "function_declaration" | "method_declaration" = cursor.node().kind() {
                    if let Some(name) = self.get_node_text(cursor.node().child_by_field_name("name"), content) {
                        sibling_context.push(name);
                    }
                }
                self.extract_relationships_from_tree(cursor, content, file_path, symbol_map, relationships, sibling_context);
            }

            cursor.goto_parent();
        }
    }

    fn extract_call_relationship(
        &self,
        node: tree_sitter::Node,
        content: &str,
        file_path: &str,
        symbol_map: &HashMap<&str, &Symbol>,
        context_stack: &[String],
    ) -> Option<Relationship> {
        let function_node = node.child_by_field_name("function")?;
        let function_name = self.get_node_text(Some(function_node), content)?;

        // Find the symbol being called
        let called_symbol = symbol_map.values()
            .find(|s| s.qualified_name == function_name || s.qualified_name.ends_with(&format!(".{}", function_name)));

        if let Some(called_symbol) = called_symbol {
            // Only create relationship if we have a valid calling context
            if !context_stack.is_empty() {
                let caller_qualified_name = context_stack.join(".");
                if let Some(caller_symbol) = symbol_map.get(caller_qualified_name.as_str()) {
                    let location = self.node_location(node, file_path);

                    return Some(Relationship {
                        from_id: caller_symbol.id.clone(),
                        to_id: called_symbol.id.clone(),
                        kind: RelationshipKind::Calls,
                        location,
                        metadata: serde_json::json!({}),
                    });
                }
            }
        }
        None
    }

    fn get_node_text(&self, node: Option<tree_sitter::Node>, content: &str) -> Option<String> {
        node.map(|n| content[n.byte_range()].to_string())
    }

    fn node_location(&self, node: tree_sitter::Node, file_path: &str) -> Location {
        let start = node.start_position();
        let end = node.end_position();

        Location {
            file: file_path.to_string(),
            line: start.row as u32,
            column: start.column as u32,
            end_line: end.row as u32,
            end_column: end.column as u32,
        }
    }
}

impl crate::index::Parser for GoParser {
    fn can_parse(&self, file_path: &str) -> bool {
        file_path.ends_with(".go")
    }

    fn parse(&self, content: &str, file_path: &str) -> anyhow::Result<(Vec<Symbol>, Vec<Relationship>)> {
        let tree = self.parse_tree(content)?;
        let symbols = self.extract_symbols(&tree, content, file_path);
        let relationships = self.extract_relationships(&tree, content, file_path, &symbols);

        Ok((symbols, relationships))
    }

    fn extract_relationships_with_global_context(&self, content: &str, file_path: &str, global_symbol_map: &std::collections::HashMap<&str, &Symbol>) -> anyhow::Result<Vec<Relationship>> {
        let tree = self.parse_tree(content)?;
        let mut relationships = Vec::new();
        let root = tree.root_node();

        let mut cursor = root.walk();
        self.extract_relationships_from_tree(&mut cursor, content, file_path, global_symbol_map, &mut relationships, Vec::new());

        Ok(relationships)
    }
}

impl JavaParser {
    pub fn new() -> Self {
        Self
    }

    fn parse_tree(&self, content: &str) -> anyhow::Result<Tree> {
        let mut parser = TreeParser::new();
        parser.set_language(&tree_sitter_java::LANGUAGE.into())?;

        let tree = parser.parse(content, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse Java code"))?;

        Ok(tree)
    }

    fn extract_symbols(&self, tree: &Tree, content: &str, file_path: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let root = tree.root_node();

        // Walk the tree to find symbols
        let mut cursor = root.walk();
        self.walk_tree(&mut cursor, content, file_path, &mut symbols, Vec::new());

        symbols
    }

    fn walk_tree(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        content: &str,
        file_path: &str,
        symbols: &mut Vec<Symbol>,
        scope_stack: Vec<String>,
    ) {
        let node = cursor.node();

        match node.kind() {
            "class_declaration" => {
                if let Some(symbol) = self.extract_class(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "interface_declaration" => {
                if let Some(symbol) = self.extract_interface(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "method_declaration" => {
                if let Some(symbol) = self.extract_method(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "constructor_declaration" => {
                if let Some(symbol) = self.extract_constructor(node, content, file_path, &scope_stack) {
                    symbols.push(symbol);
                }
            }
            "field_declaration" => {
                let field_symbols = self.extract_field_declaration(node, content, file_path, &scope_stack);
                symbols.extend(field_symbols);
            }
            "local_variable_declaration" => {
                let var_symbols = self.extract_local_variable_declaration(node, content, file_path, &scope_stack);
                symbols.extend(var_symbols);
            }
            _ => {}
        }

        // Recurse into children
        if cursor.goto_first_child() {
            let mut new_scope = scope_stack.clone();
            if let "class_declaration" | "method_declaration" | "constructor_declaration" = node.kind() {
                if let Some(name) = self.get_node_text(node.child_by_field_name("name"), content) {
                    new_scope.push(name);
                }
            }

            self.walk_tree(cursor, content, file_path, symbols, new_scope);

            while cursor.goto_next_sibling() {
                self.walk_tree(cursor, content, file_path, symbols, scope_stack.clone());
            }

            cursor.goto_parent();
        }
    }

    fn extract_class(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", scope_stack.join("."), name)
        };

        let location = self.node_location(node, file_path);

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Class,
            name,
            qualified_name,
            location,
            signature: None,
            type_info: None,
            visibility: Visibility::Public, // Default visibility in Java
            language: "java".to_string(),
            metadata: serde_json::json!({}),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_interface(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", scope_stack.join("."), name)
        };

        let location = self.node_location(node, file_path);

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Type,
            name,
            qualified_name,
            location,
            signature: None,
            type_info: None,
            visibility: Visibility::Public,
            language: "java".to_string(),
            metadata: serde_json::json!({
                "interface": true
            }),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_method(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        let name_node = node.child_by_field_name("name")?;
        let name = self.get_node_text(Some(name_node), content)?;

        // Get class name from scope
        let class_name = scope_stack.last().unwrap_or(&"Unknown".to_string()).clone();

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", scope_stack.join("."), name)
        };

        let location = self.node_location(node, file_path);

        // Extract parameters
        let parameters_node = node.child_by_field_name("parameters");
        let mut parameters = Vec::new();
        if let Some(params) = parameters_node {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                if child.kind() == "formal_parameter" {
                    if let Some(type_node) = child.child_by_field_name("type") {
                        if let Some(param_type) = self.get_node_text(Some(type_node), content) {
                            parameters.push(param_type);
                        }
                    }
                }
            }
        }

        let signature = format!("{} {}({})", class_name, name, parameters.join(", "));

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Method,
            name,
            qualified_name,
            location,
            signature: Some(signature),
            type_info: Some(class_name),
            visibility: Visibility::Public,
            language: "java".to_string(),
            metadata: serde_json::json!({
                "parameters": parameters
            }),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_constructor(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Option<Symbol> {
        // Get class name from scope
        let class_name = scope_stack.last().unwrap_or(&"Unknown".to_string()).clone();
        let name = class_name.clone(); // Constructor name is the class name

        let qualified_name = if scope_stack.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", scope_stack.join("."), name)
        };

        let location = self.node_location(node, file_path);

        // Extract parameters
        let parameters_node = node.child_by_field_name("parameters");
        let mut parameters = Vec::new();
        if let Some(params) = parameters_node {
            let mut cursor = params.walk();
            for child in params.children(&mut cursor) {
                if child.kind() == "formal_parameter" {
                    if let Some(type_node) = child.child_by_field_name("type") {
                        if let Some(param_type) = self.get_node_text(Some(type_node), content) {
                            parameters.push(param_type);
                        }
                    }
                }
            }
        }

        let signature = format!("{}({})", class_name, parameters.join(", "));

        Some(Symbol {
            id: format!("{}:{}", file_path, qualified_name),
            kind: SymbolKind::Method,
            name,
            qualified_name,
            location,
            signature: Some(signature),
            type_info: Some(class_name),
            visibility: Visibility::Public,
            language: "java".to_string(),
            metadata: serde_json::json!({
                "constructor": true,
                "parameters": parameters
            }),
            content_hash: "".to_string(),
            last_indexed: chrono::Utc::now().timestamp(),
        })
    }

    fn extract_field_declaration(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Vec<Symbol> {
        let mut symbols = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Some(name) = self.get_node_text(Some(name_node), content) {
                        let qualified_name = if scope_stack.is_empty() {
                            name.clone()
                        } else {
                            format!("{}.{}", scope_stack.join("."), name)
                        };

                        let location = self.node_location(child, file_path);

                        symbols.push(Symbol {
                            id: format!("{}:{}", file_path, qualified_name),
                            kind: SymbolKind::Field,
                            name,
                            qualified_name,
                            location,
                            signature: None,
                            type_info: None,
                            visibility: Visibility::Public,
                            language: "java".to_string(),
                            metadata: serde_json::json!({
                                "field": true
                            }),
                            content_hash: "".to_string(),
                            last_indexed: chrono::Utc::now().timestamp(),
                        });
                    }
                }
            }
        }

        symbols
    }

    fn extract_local_variable_declaration(&self, node: tree_sitter::Node, content: &str, file_path: &str, scope_stack: &[String]) -> Vec<Symbol> {
        let mut symbols = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Some(name) = self.get_node_text(Some(name_node), content) {
                        let qualified_name = if scope_stack.is_empty() {
                            name.clone()
                        } else {
                            format!("{}.{}", scope_stack.join("."), name)
                        };

                        let location = self.node_location(child, file_path);

                        symbols.push(Symbol {
                            id: format!("{}:{}", file_path, qualified_name),
                            kind: SymbolKind::Variable,
                            name,
                            qualified_name,
                            location,
                            signature: None,
                            type_info: None,
                            visibility: Visibility::Public,
                            language: "java".to_string(),
                            metadata: serde_json::json!({
                                "local": true
                            }),
                            content_hash: "".to_string(),
                            last_indexed: chrono::Utc::now().timestamp(),
                        });
                    }
                }
            }
        }

        symbols
    }

    fn extract_relationships(&self, tree: &Tree, content: &str, file_path: &str, symbols: &[Symbol]) -> Vec<Relationship> {
        let mut relationships = Vec::new();
        let root = tree.root_node();

        // Create a map of qualified names to symbol IDs for lookup
        let symbol_map: HashMap<&str, &Symbol> = symbols.iter()
            .map(|s| (s.qualified_name.as_str(), s))
            .collect();

        let mut cursor = root.walk();
        self.extract_relationships_from_tree(&mut cursor, content, file_path, &symbol_map, &mut relationships, Vec::new());

        relationships
    }

    fn extract_relationships_from_tree(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        content: &str,
        file_path: &str,
        symbol_map: &HashMap<&str, &Symbol>,
        relationships: &mut Vec<Relationship>,
        context_stack: Vec<String>,
    ) {
        let node = cursor.node();

        match node.kind() {
            "method_invocation" => {
                if let Some(rel) = self.extract_method_invocation(node, content, file_path, symbol_map, &context_stack) {
                    relationships.push(rel);
                }
            }
            _ => {}
        }

        // Recurse
        if cursor.goto_first_child() {
            let mut new_context = context_stack.clone();
            if let "method_declaration" | "constructor_declaration" = node.kind() {
                if let Some(name) = self.get_node_text(node.child_by_field_name("name"), content) {
                    new_context.push(name);
                }
            }

            self.extract_relationships_from_tree(cursor, content, file_path, symbol_map, relationships, new_context);

            while cursor.goto_next_sibling() {
                let mut sibling_context = context_stack.clone();
                if let "method_declaration" | "constructor_declaration" = cursor.node().kind() {
                    if let Some(name) = self.get_node_text(cursor.node().child_by_field_name("name"), content) {
                        sibling_context.push(name);
                    }
                }
                self.extract_relationships_from_tree(cursor, content, file_path, symbol_map, relationships, sibling_context);
            }

            cursor.goto_parent();
        }
    }

    fn extract_method_invocation(
        &self,
        node: tree_sitter::Node,
        content: &str,
        file_path: &str,
        symbol_map: &HashMap<&str, &Symbol>,
        context_stack: &[String],
    ) -> Option<Relationship> {
        let name_node = node.child_by_field_name("name")?;
        let method_name = self.get_node_text(Some(name_node), content)?;

        // Look for the method in any class
        let called_symbol = symbol_map.values()
            .find(|s| s.kind == SymbolKind::Method && s.name == method_name);

        if let Some(called_symbol) = called_symbol {
            // Only create relationship if we have a valid calling context
            if !context_stack.is_empty() {
                let caller_qualified_name = context_stack.join(".");
                if let Some(caller_symbol) = symbol_map.get(caller_qualified_name.as_str()) {
                    let location = self.node_location(node, file_path);

                    return Some(Relationship {
                        from_id: caller_symbol.id.clone(),
                        to_id: called_symbol.id.clone(),
                        kind: RelationshipKind::Calls,
                        location,
                        metadata: serde_json::json!({}),
                    });
                }
            }
        }
        None
    }

    fn get_node_text(&self, node: Option<tree_sitter::Node>, content: &str) -> Option<String> {
        node.map(|n| content[n.byte_range()].to_string())
    }

    fn node_location(&self, node: tree_sitter::Node, file_path: &str) -> Location {
        let start = node.start_position();
        let end = node.end_position();

        Location {
            file: file_path.to_string(),
            line: start.row as u32,
            column: start.column as u32,
            end_line: end.row as u32,
            end_column: end.column as u32,
        }
    }
}

impl IntentParser {
    pub fn new() -> Self {
        Self
    }

    fn extract_symbols(&self, content: &str, file_path: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            let line = line.trim();

            // Parse context declarations
            if line.starts_with("context ") {
                if let Some(context_symbol) = self.extract_context_declaration(line, file_path, line_num + 1) {
                    symbols.push(context_symbol);
                }
            }
            // Parse field declarations (inside contexts)
            else if line.contains(": ") && !line.starts_with("fn ") && !line.starts_with("//") && !line.starts_with("///") {
                if let Some(field_symbol) = self.extract_field_declaration(line, file_path, line_num + 1) {
                    symbols.push(field_symbol);
                }
            }
            // Parse method declarations
            else if line.starts_with("fn ") {
                if let Some(method_symbol) = self.extract_method_declaration(line, file_path, line_num + 1) {
                    symbols.push(method_symbol);
                }
            }
        }

        symbols
    }

    fn extract_context_declaration(&self, line: &str, file_path: &str, line_num: usize) -> Option<Symbol> {
        // Match: context <Name> [persist "..."] [extends <Base>] [depends [...]] {
        let re = regex::Regex::new(r"context\s+(\w+)").ok()?;
        if let Some(captures) = re.captures(line) {
            let name = captures.get(1)?.as_str();

            let location = Location {
                file: file_path.to_string(),
                line: line_num as u32,
                column: line.find("context").unwrap_or(0) as u32,
                end_line: line_num as u32,
                end_column: (line.find("context").unwrap_or(0) + line.len()) as u32,
            };

            Some(Symbol {
                id: format!("{}:{}", file_path, name),
                kind: SymbolKind::Context,
                name: name.to_string(),
                qualified_name: name.to_string(),
                location,
                signature: Some(format!("context {}", name)),
                type_info: None,
                visibility: Visibility::Public,
                language: "intent".to_string(),
                metadata: serde_json::json!({
                    "context": true
                }),
                content_hash: "".to_string(),
                last_indexed: chrono::Utc::now().timestamp(),
            })
        } else {
            None
        }
    }

    fn extract_field_declaration(&self, line: &str, file_path: &str, line_num: usize) -> Option<Symbol> {
        // Match: <name>: <Type> [= <default>]
        let re = regex::Regex::new(r"(\w+)\s*:\s*([^=\s]+)").ok()?;
        if let Some(captures) = re.captures(line) {
            let name = captures.get(1)?.as_str();
            let type_info = captures.get(2)?.as_str();

            let location = Location {
                file: file_path.to_string(),
                line: line_num as u32,
                column: 0,
                end_line: line_num as u32,
                end_column: line.len() as u32,
            };

            Some(Symbol {
                id: format!("{}:{}", file_path, name),
                kind: SymbolKind::Field,
                name: name.to_string(),
                qualified_name: name.to_string(),
                location,
                signature: Some(format!("{}: {}", name, type_info)),
                type_info: Some(type_info.to_string()),
                visibility: Visibility::Public,
                language: "intent".to_string(),
                metadata: serde_json::json!({
                    "field": true
                }),
                content_hash: "".to_string(),
                last_indexed: chrono::Utc::now().timestamp(),
            })
        } else {
            None
        }
    }

    fn extract_method_declaration(&self, line: &str, file_path: &str, line_num: usize) -> Option<Symbol> {
        // Match: fn <name>(<params>) [-> <return_type>]
        let re = regex::Regex::new(r"fn\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*(\w+))?").ok()?;
        if let Some(captures) = re.captures(line) {
            let name = captures.get(1)?.as_str();
            let params = captures.get(2).map_or("", |m| m.as_str());
            let return_type = captures.get(3).map_or("void", |m| m.as_str());

            let signature = if return_type == "void" {
                format!("fn {}({})", name, params)
            } else {
                format!("fn {}({}) -> {}", name, params, return_type)
            };

            let location = Location {
                file: file_path.to_string(),
                line: line_num as u32,
                column: line.find("fn").unwrap_or(0) as u32,
                end_line: line_num as u32,
                end_column: (line.find("fn").unwrap_or(0) + line.len()) as u32,
            };

            Some(Symbol {
                id: format!("{}:{}", file_path, name),
                kind: SymbolKind::Function,
                name: name.to_string(),
                qualified_name: name.to_string(),
                location,
                signature: Some(signature),
                type_info: Some(return_type.to_string()),
                visibility: Visibility::Public,
                language: "intent".to_string(),
                metadata: serde_json::json!({
                    "function": true,
                    "parameters": params
                }),
                content_hash: "".to_string(),
                last_indexed: chrono::Utc::now().timestamp(),
            })
        } else {
            None
        }
    }

    fn extract_relationships(&self, _content: &str, _file_path: &str, _symbols: &[Symbol]) -> Vec<Relationship> {
        // For now, skip relationship extraction for Intent files
        // TODO: Implement proper relationship extraction with context tracking
        Vec::new()
    }

    fn extract_method_calls(&self, line: &str, file_path: &str, line_num: usize, symbol_map: &HashMap<&str, &Symbol>) -> Option<Vec<Relationship>> {
        let mut relationships = Vec::new();

        // Simple regex to find method calls: word( or word.word(
        let re = regex::Regex::new(r"(\w+(?:\.\w+)*)\s*\(").ok()?;

        for capture in re.captures_iter(line) {
            if let Some(method_ref) = capture.get(1) {
                let method_name = method_ref.as_str();

                // Try to find the method in our symbols
                if let Some(called_symbol) = symbol_map.get(method_name) {
                    // For now, we don't track the caller context in this simple parser
                    // In a real implementation, we'd need to track the current context/method
                    let location = Location {
                        file: file_path.to_string(),
                        line: line_num as u32,
                        column: method_ref.start() as u32,
                        end_line: line_num as u32,
                        end_column: method_ref.end() as u32,
                    };

                    relationships.push(Relationship {
                        from_id: format!("{}:unknown_caller", file_path), // Placeholder
                        to_id: called_symbol.id.clone(),
                        kind: RelationshipKind::Calls,
                        location,
                        metadata: serde_json::json!({}),
                    });
                }
            }
        }

        if relationships.is_empty() {
            None
        } else {
            Some(relationships)
        }
    }
}

impl crate::index::Parser for JavaParser {
    fn can_parse(&self, file_path: &str) -> bool {
        file_path.ends_with(".java")
    }

    fn parse(&self, content: &str, file_path: &str) -> anyhow::Result<(Vec<Symbol>, Vec<Relationship>)> {
        let tree = self.parse_tree(content)?;
        let symbols = self.extract_symbols(&tree, content, file_path);
        let relationships = self.extract_relationships(&tree, content, file_path, &symbols);

        Ok((symbols, relationships))
    }

    fn extract_relationships_with_global_context(&self, content: &str, file_path: &str, global_symbol_map: &std::collections::HashMap<&str, &Symbol>) -> anyhow::Result<Vec<Relationship>> {
        let tree = self.parse_tree(content)?;
        let mut relationships = Vec::new();
        let root = tree.root_node();

        let mut cursor = root.walk();
        self.extract_relationships_from_tree(&mut cursor, content, file_path, global_symbol_map, &mut relationships, Vec::new());

        Ok(relationships)
    }
}

impl crate::index::Parser for IntentParser {
    fn can_parse(&self, file_path: &str) -> bool {
        file_path.ends_with(".intent")
    }

    fn parse(&self, content: &str, file_path: &str) -> anyhow::Result<(Vec<Symbol>, Vec<Relationship>)> {
        let symbols = self.extract_symbols(content, file_path);
        let relationships = self.extract_relationships(content, file_path, &symbols);

        Ok((symbols, relationships))
    }

    fn extract_relationships_with_global_context(&self, _content: &str, _file_path: &str, _global_symbol_map: &std::collections::HashMap<&str, &Symbol>) -> anyhow::Result<Vec<Relationship>> {
        // For now, skip relationship extraction for Intent files
        // TODO: Implement proper relationship extraction with context tracking
        Ok(Vec::new())
    }
}

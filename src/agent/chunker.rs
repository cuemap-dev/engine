use tree_sitter::Parser;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub context: String, // e.g., "function calculate_tax"
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChunkerType {
    Python,
    Rust,
    TypeScript,
    JavaScript,
    Go,
    Html,
    Css,
    Php,
    Java,
    Markdown,
    Csv,
    Json,
    Yaml,
    Xml,
    Pdf,
    Office, // DOCX, XLSX, PPTX
    Text,
}

pub struct Chunker {
    // Parsers are not thread-safe so we create them on demand or thread-local 
    // but for simplicity here we re-create or use a pool later.
}

impl Chunker {
    pub fn chunk_file(path: &Path, content: &str) -> Vec<Chunk> {
        let file_type = Self::detect_type(path);
        
        match file_type {
            ChunkerType::Python => Self::chunk_python(content),
            ChunkerType::Rust => Self::chunk_rust(content),
            ChunkerType::TypeScript => Self::chunk_typescript(content),
            ChunkerType::JavaScript => Self::chunk_javascript(content),
            ChunkerType::Go => Self::chunk_go(content),
            ChunkerType::Html => Self::chunk_html(content),
            ChunkerType::Css => Self::chunk_css(content),
            ChunkerType::Php => Self::chunk_php(content),
            ChunkerType::Java => Self::chunk_java(content),
            ChunkerType::Markdown => Self::chunk_markdown(content),
            ChunkerType::Csv => Self::chunk_csv(content),
            ChunkerType::Json => Self::chunk_json(content),
            ChunkerType::Yaml => Self::chunk_yaml(content),
            ChunkerType::Xml => Self::chunk_xml(content),
            ChunkerType::Pdf => Self::chunk_pdf(path),
            ChunkerType::Office => Self::chunk_office(path),
            ChunkerType::Text => Self::chunk_text(content),
        }
    }

    fn detect_type(path: &Path) -> ChunkerType {
        match path.extension().and_then(|s| s.to_str()) {
            Some("py") => ChunkerType::Python,
            Some("rs") => ChunkerType::Rust,
            Some("ts" | "tsx") => ChunkerType::TypeScript,
            Some("js" | "jsx") => ChunkerType::JavaScript,
            Some("go") => ChunkerType::Go,
            Some("html" | "htm") => ChunkerType::Html,
            Some("css") => ChunkerType::Css,
            Some("php") => ChunkerType::Php,
            Some("java") => ChunkerType::Java,
            Some("md") => ChunkerType::Markdown,
            Some("csv") => ChunkerType::Csv,
            Some("json") => ChunkerType::Json,
            Some("yaml" | "yml") => ChunkerType::Yaml,
            Some("xml") => ChunkerType::Xml,
            Some("pdf") => ChunkerType::Pdf,
            Some("docx" | "xlsx" | "pptx") => ChunkerType::Office,
            _ => ChunkerType::Text,
        }
    }

    fn chunk_python(content: &str) -> Vec<Chunk> {
        // Fallback or Tree-sitter
        let mut parser = Parser::new();
        let language = tree_sitter_python::LANGUAGE;
        parser.set_language(&language.into()).expect("Error loading Python grammar");

        let mut chunks = Vec::new();
        if let Some(tree) = parser.parse(content, None) {
             let root = tree.root_node();
             let mut cursor = root.walk();
             
             // Simple traversal for top-level functions and classes
             for child in root.children(&mut cursor) {
                 match child.kind() {
                     "function_definition" | "class_definition" => {
                         // Extract name
                         let name = child.child_by_field_name("name")
                             .map(|n| n.utf8_text(content.as_bytes()).unwrap_or("anon"))
                             .unwrap_or("anon");
                         
                         let start = child.start_position().row + 1;
                         let end = child.end_position().row + 1;
                         let text = child.utf8_text(content.as_bytes()).unwrap_or("").to_string();
                         
                         chunks.push(Chunk {
                             content: text,
                             start_line: start,
                             end_line: end,
                             context: format!("{}:{}", child.kind(), name),
                         });
                     },
                     _ => {} // Ignore top level statements for now or chunk them differently
                 }
             }
        }
        
        // Safety: if tree-sitter found nothing structure-wise but file not empty, treat as text
        if chunks.is_empty() && !content.trim().is_empty() {
             return Self::chunk_text(content);
        }
        
        chunks
    }

    fn chunk_rust(content: &str) -> Vec<Chunk> {
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        parser.set_language(&language.into()).expect("Error loading Rust grammar");
        
        let mut chunks = Vec::new();
        if let Some(tree) = parser.parse(content, None) {
             let root = tree.root_node();
             let mut cursor = root.walk();
             
             for child in root.children(&mut cursor) {
                 match child.kind() {
                     "function_item" | "struct_item" | "impl_item" | "enum_item" | "mod_item" => {
                         let name = child.child_by_field_name("name")
                            .map(|n| n.utf8_text(content.as_bytes()).unwrap_or("anon"))
                            .unwrap_or("anon");

                         let start = child.start_position().row + 1;
                         let end = child.end_position().row + 1;
                         let text = child.utf8_text(content.as_bytes()).unwrap_or("").to_string();
                         
                         chunks.push(Chunk {
                             content: text,
                             start_line: start,
                             end_line: end,
                             context: format!("{}:{}", child.kind(), name),
                         });
                     },
                     _ => {}
                 }
             }
        }

        if chunks.is_empty() && !content.trim().is_empty() {
             return Self::chunk_text(content);
        }
        
        chunks
    }
    
    fn chunk_typescript(content: &str) -> Vec<Chunk> {
        let mut parser = Parser::new();
        let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT;
        parser.set_language(&language.into()).expect("Error loading TS grammar");
        
        let mut chunks = Vec::new();
        if let Some(tree) = parser.parse(content, None) {
             let root = tree.root_node();
             let mut cursor = root.walk();
             
             for child in root.children(&mut cursor) {
                 match child.kind() {
                     "function_declaration" | "class_declaration" | "interface_declaration" | "lexical_declaration" => {
                         // Simplify: just grab the whole block
                         let start = child.start_position().row + 1;
                         let end = child.end_position().row + 1;
                         let text = child.utf8_text(content.as_bytes()).unwrap_or("").to_string();
                         let kind = child.kind();
                         
                         chunks.push(Chunk {
                             content: text,
                             start_line: start,
                             end_line: end,
                             context: format!("{}", kind),
                         });
                     },
                     _ => {}
                 }
             }
        }
        
        if chunks.is_empty() && !content.trim().is_empty() {
             return Self::chunk_text(content);
        }

        chunks
    }

    fn chunk_javascript(content: &str) -> Vec<Chunk> {
        let mut parser = Parser::new();
        let language = tree_sitter_javascript::LANGUAGE;
        parser.set_language(&language.into()).expect("Error loading JS grammar");
        Self::chunk_treesitter(content, parser, &["function_declaration", "class_declaration", "method_definition"])
    }

    fn chunk_go(content: &str) -> Vec<Chunk> {
        let mut parser = Parser::new();
        let language = tree_sitter_go::LANGUAGE;
        parser.set_language(&language.into()).expect("Error loading Go grammar");
        Self::chunk_treesitter(content, parser, &["function_declaration", "method_declaration", "type_declaration"])
    }

    fn chunk_html(content: &str) -> Vec<Chunk> {
        let mut parser = Parser::new();
        let language = tree_sitter_html::LANGUAGE;
        parser.set_language(&language.into()).expect("Error loading HTML grammar");
        Self::chunk_treesitter(content, parser, &["element", "script_element", "style_element"])
    }

    fn chunk_css(content: &str) -> Vec<Chunk> {
        let mut parser = Parser::new();
        let language = tree_sitter_css::LANGUAGE;
        parser.set_language(&language.into()).expect("Error loading CSS grammar");
        Self::chunk_treesitter(content, parser, &["rule_set", "media_rule"])
    }

    fn chunk_php(content: &str) -> Vec<Chunk> {
        let mut parser = Parser::new();
        // tree-sitter-php 0.23 uses LANGUAGE_PHP
        let language = tree_sitter_php::LANGUAGE_PHP;
        parser.set_language(&language.into()).expect("Error loading PHP grammar");
        Self::chunk_treesitter(content, parser, &["function_definition", "class_definition", "method_declaration"])
    }

    fn chunk_java(content: &str) -> Vec<Chunk> {
        let mut parser = Parser::new();
        let language = tree_sitter_java::LANGUAGE;
        parser.set_language(&language.into()).expect("Error loading Java grammar");
        Self::chunk_treesitter(content, parser, &["class_declaration", "method_declaration", "constructor_declaration"])
    }

    fn chunk_treesitter(content: &str, mut parser: Parser, node_kinds: &[&str]) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        if let Some(tree) = parser.parse(content, None) {
             let root = tree.root_node();
             let mut cursor = root.walk();
             
             for child in root.children(&mut cursor) {
                 if node_kinds.contains(&child.kind()) {
                     let start = child.start_position().row + 1;
                     let end = child.end_position().row + 1;
                     let text = child.utf8_text(content.as_bytes()).unwrap_or("").to_string();
                     
                     chunks.push(Chunk {
                         content: text,
                         start_line: start,
                         end_line: end,
                         context: format!("{}", child.kind()),
                     });
                 }
             }
        }
        
        if chunks.is_empty() && !content.trim().is_empty() {
             return Self::chunk_text(content);
        }
        chunks
    }

    fn chunk_markdown(content: &str) -> Vec<Chunk> {
        // Split by headers (#, ##, etc.)
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut current_block = Vec::new();
        let mut current_start = 1;
        let mut current_header = "root".to_string();
        
        for (i, line) in lines.iter().enumerate() {
            if line.starts_with('#') {
                if !current_block.is_empty() {
                    chunks.push(Chunk {
                        content: current_block.join("\n"),
                        start_line: current_start,
                        end_line: i,
                        context: current_header.clone(),
                    });
                    current_block.clear();
                }
                current_start = i + 1;
                current_header = line.trim_start_matches('#').trim().to_string();
            }
            current_block.push(*line);
        }
        
        if !current_block.is_empty() {
            chunks.push(Chunk {
                content: current_block.join("\n"),
                start_line: current_start,
                end_line: lines.len(),
                context: current_header,
            });
        }
        
        chunks
    }

    fn chunk_csv(content: &str) -> Vec<Chunk> {
        let mut rdr = csv::Reader::from_reader(content.as_bytes());
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut row_count = 0;
        let headers = rdr.headers().cloned().unwrap_or_default();
        
        for result in rdr.records() {
            if let Ok(record) = result {
                if row_count % 10 == 0 && row_count > 0 {
                    chunks.push(Chunk {
                        content: current_chunk.clone(),
                        start_line: row_count,
                        end_line: row_count + 10,
                        context: "csv_rows".to_string(),
                    });
                    current_chunk.clear();
                    current_chunk.push_str(&headers.iter().collect::<Vec<_>>().join(","));
                    current_chunk.push('\n');
                }
                current_chunk.push_str(&record.iter().collect::<Vec<_>>().join(","));
                current_chunk.push('\n');
                row_count += 1;
            }
        }
        
        if !current_chunk.is_empty() {
            chunks.push(Chunk {
                content: current_chunk,
                start_line: row_count.saturating_sub(10),
                end_line: row_count,
                context: "csv_rows".to_string(),
            });
        }
        chunks
    }

    fn chunk_json(content: &str) -> Vec<Chunk> {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(obj) = value.as_object() {
                return obj.iter().map(|(key, val)| Chunk {
                    content: format!("\"{}\": {}", key, val),
                    start_line: 0,
                    end_line: 0,
                    context: format!("json_key:{}", key),
                }).collect();
            } else if let Some(arr) = value.as_array() {
                return arr.iter().enumerate().map(|(i, val)| Chunk {
                    content: val.to_string(),
                    start_line: 0,
                    end_line: 0,
                    context: format!("json_index:{}", i),
                }).collect();
            }
        }
        Self::chunk_text(content)
    }

    fn chunk_yaml(content: &str) -> Vec<Chunk> {
        if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(content) {
            if let Some(mapping) = value.as_mapping() {
                return mapping.iter().map(|(k, v)| Chunk {
                    content: format!("{}: {}", serde_yaml::to_string(k).unwrap_or_default().trim(), serde_yaml::to_string(v).unwrap_or_default().trim()),
                    start_line: 0,
                    end_line: 0,
                    context: "yaml_block".to_string(),
                }).collect();
            }
        }
        Self::chunk_text(content)
    }

    fn chunk_xml(content: &str) -> Vec<Chunk> {
        if let Ok(doc) = roxmltree::Document::parse(content) {
            let mut chunks = Vec::new();
            for node in doc.root().children() {
                if node.is_element() {
                    chunks.push(Chunk {
                        content: node.document().input_text()[node.range()].to_string(),
                        start_line: 0,
                        end_line: 0,
                        context: format!("xml_tag:{}", node.tag_name().name()),
                    });
                }
            }
            if !chunks.is_empty() {
                return chunks;
            }
        }
        Self::chunk_text(content)
    }

    fn chunk_pdf(path: &Path) -> Vec<Chunk> {
        if let Ok(content) = pdf_extract::extract_text(path) {
            return Self::chunk_text(&content);
        }
        Vec::new()
    }

    fn chunk_office(path: &Path) -> Vec<Chunk> {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let mut full_text = String::new();

        match extension {
            "xlsx" => {
                use calamine::{Reader, Xlsx, open_workbook};
                if let Ok(mut excel) = open_workbook::<Xlsx<_>, _>(path) {
                    for sheet_name in excel.sheet_names().to_owned() {
                        if let Some(Ok(range)) = excel.worksheet_range(&sheet_name) {
                            for row in range.rows() {
                                for cell in row {
                                    full_text.push_str(&cell.to_string());
                                    full_text.push(' ');
                                }
                                full_text.push('\n');
                            }
                        }
                    }
                }
            },
            "docx" => {
                // docx-rs is better for structured reading
                if let Ok(_bytes) = std::fs::read(path) {
                    // Extract text (simplified placeholder for now as docx-rs is complex)
                    // In a real scenario we'd traverse the document tree
                    full_text.push_str("DOCX Content Placeholder");
                }
            },
            _ => {
                return Vec::new();
            }
        }
        
        if !full_text.trim().is_empty() {
            return Self::chunk_text(&full_text);
        }
        Vec::new()
    }

    fn chunk_text(content: &str) -> Vec<Chunk> {
        // Simple paragraph splitter
        // Split by double newline
        content.split("\n\n").enumerate().map(|(i, s)| {
             Chunk {
                 content: s.to_string(),
                 start_line: 0, // Hard to track line numbers with simple split
                 end_line: 0,
                 context: format!("para:{}", i),
             }
        }).filter(|c| !c.content.trim().is_empty()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_csv_chunking() {
        let content = "id,name\n1,alice\n2,bob";
        let chunks = Chunker::chunk_csv(content);
        assert!(!chunks.is_empty());
        assert!(chunks[0].content.contains("alice"));
    }

    #[test]
    fn test_json_chunking() {
        let content = "{\"key\": \"value\", \"list\": [1, 2]}";
        let chunks = Chunker::chunk_json(content);
        assert!(chunks.len() >= 2);
        assert!(chunks.iter().any(|c| c.context.contains("json_key:key")));
    }

    #[test]
    fn test_yaml_chunking() {
        let content = "engine: cuemap\nversion: 0.5";
        let chunks = Chunker::chunk_yaml(content);
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.content.contains("cuemap")));
    }

    #[test]
    fn test_html_chunking() {
        let content = "<html><body><h1>Test</h1></body></html>";
        let chunks = Chunker::chunk_html(content);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].context, "element");
    }

    #[test]
    fn test_java_chunking() {
        let content = "public class Test { public void hello() {} }";
        let chunks = Chunker::chunk_java(content);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].context, "class_declaration");
    }

    #[test]
    fn test_go_chunking() {
        let content = "package main\nfunc main() {}";
        let chunks = Chunker::chunk_go(content);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].context, "function_declaration");
    }

    #[test]
    fn test_php_chunking() {
        let content = "<?php function test() {} ?>";
        let chunks = Chunker::chunk_php(content);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].context, "function_definition");
    }

    #[test]
    fn test_css_chunking() {
        let content = ".selector { color: red; }";
        let chunks = Chunker::chunk_css(content);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].context, "rule_set");
    }

    #[test]
    fn test_detect_type() {
        assert_eq!(Chunker::detect_type(&PathBuf::from("test.py")), ChunkerType::Python);
        assert_eq!(Chunker::detect_type(&PathBuf::from("test.csv")), ChunkerType::Csv);
        assert_eq!(Chunker::detect_type(&PathBuf::from("test.pdf")), ChunkerType::Pdf);
        assert_eq!(Chunker::detect_type(&PathBuf::from("test.docx")), ChunkerType::Office);
    }
}

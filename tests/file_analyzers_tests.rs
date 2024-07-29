use git_iris::file_analyzers::get_analyzer;
use git_iris::git::FileChange;

#[test]
fn test_rust_analyzer() {
    let analyzer = get_analyzer("test.rs");
    let change = FileChange {
        status: "M".to_string(),
        diff: r#"
+pub fn new_function() {
+    println!("Hello, world!");
+}
-struct OldStruct {
-    field: i32,
-}
+struct NewStruct {
+    field: String,
+}
        "#
        .to_string(),
    };

    let analysis = analyzer.analyze("test.rs", &change);
    println!("Rust Test Debug: Analysis results: {:?}", analysis);
    assert!(analysis.contains(&"Modified functions: new_function".to_string()));
    assert!(analysis.contains(&"Modified structs: OldStruct, NewStruct".to_string()));
}

#[test]
fn test_javascript_analyzer() {
    let analyzer = get_analyzer("test.js");
    let change = FileChange {
        status: "M".to_string(),
        diff: r#"
+function newFunction() {
+    console.log("Hello, world!");
+}
-class OldClass {
-    constructor() {}
-}
+class NewClass extends React.Component {
+    render() {
+        return <div>Hello</div>;
+    }
+}
+const FunctionalComponent = () => {
+    return <div>Functional</div>;
+}
+import { useState } from 'react';
        "#
        .to_string(),
    };

    let analysis = analyzer.analyze("test.js", &change);
    println!("JavaScript Test Debug: Analysis results: {:?}", analysis);
    assert!(analysis.contains(&"Modified functions: newFunction, FunctionalComponent".to_string()));
    assert!(analysis.contains(&"Modified classes: OldClass, NewClass".to_string()));
    assert!(analysis.contains(&"Import statements have been modified".to_string()));

    let react_components = analysis
        .iter()
        .find(|&s| s.starts_with("Modified React components:"))
        .unwrap();
    assert!(react_components.contains("NewClass"));
    assert!(react_components.contains("FunctionalComponent"));
}

#[test]
fn test_python_analyzer() {
    let analyzer = get_analyzer("test.py");
    let change = FileChange {
        status: "M".to_string(),
        diff: r#"
+def new_function():
+    print("Hello, world!")
-class OldClass:
-    pass
+class NewClass:
+    def __init__(self):
+        pass
+@decorator
+def decorated_function():
+    pass
+from module import something
        "#
        .to_string(),
    };

    let analysis = analyzer.analyze("test.py", &change);
    println!("Python Test Debug: Analysis results: {:?}", analysis);
    assert!(analysis.contains(&"Modified functions: new_function, decorated_function".to_string()));
    assert!(analysis.contains(&"Modified classes: OldClass, NewClass".to_string()));
    assert!(analysis.contains(&"Import statements have been modified".to_string()));
    assert!(analysis.contains(&"Modified decorators: decorator".to_string()));
}

#[test]
fn test_yaml_analyzer() {
    let analyzer = get_analyzer("test.yaml");
    let change = FileChange {
        status: "M".to_string(),
        diff: r#"
+new_key: value
-old_key: value
 list:
+  - new item
-  - old item
 nested:
+  inner_key: value
        "#
        .to_string(),
    };

    let analysis = analyzer.analyze("test.yaml", &change);
    println!("YAML Test Debug: Analysis results: {:?}", analysis);

    let top_level_keys_analysis = analysis
        .iter()
        .find(|&s| s.starts_with("Modified top-level keys:"))
        .expect("No top-level keys analysis found");
    assert!(top_level_keys_analysis.contains("new_key"));
    assert!(top_level_keys_analysis.contains("old_key"));
    assert!(!top_level_keys_analysis.contains("inner_key"));

    assert!(analysis.contains(&"List structures have been modified".to_string()));
    assert!(analysis.contains(&"Nested structures have been modified".to_string()));
}

#[test]
fn test_json_analyzer() {
    let analyzer = get_analyzer("test.json");
    let change = FileChange {
        status: "M".to_string(),
        diff: r#"
+  "new_key": "value",
-  "old_key": "value",
   "array": [
+    "new item",
-    "old item"
   ],
   "nested": {
+    "inner_key": "value"
   }
        "#
        .to_string(),
    };

    let analysis = analyzer.analyze("test.json", &change);
    println!("JSON Test Debug: Analysis results: {:?}", analysis);

    assert!(analysis
        .iter()
        .any(|s| s.starts_with("Modified top-level keys:")));
    assert!(analysis.contains(&"Array structures have been modified".to_string()));
    assert!(analysis.contains(&"Nested objects have been modified".to_string()));

    let top_level_keys_analysis = analysis
        .iter()
        .find(|&s| s.starts_with("Modified top-level keys:"))
        .unwrap();
    assert!(top_level_keys_analysis.contains("new_key"));
    assert!(top_level_keys_analysis.contains("old_key"));
    assert!(!top_level_keys_analysis.contains("inner_key"));
}

#[test]
fn test_markdown_analyzer() {
    let analyzer = get_analyzer("test.md");
    let change = FileChange {
        status: "M".to_string(),
        diff: r#"
+# New Header
-## Old Header
+ - New list item
- * Old list item
+```
+New code block
+```
+[New link](https://example.com)
-[Old link](https://old-example.com)
        "#
        .to_string(),
    };

    let analysis = analyzer.analyze("test.md", &change);
    println!("Markdown Test Debug: Analysis results: {:?}", analysis);
    assert!(analysis.contains(&"Modified headers: New Header, Old Header".to_string()));
    assert!(analysis.contains(&"List structures have been modified".to_string()));
    assert!(analysis.contains(&"Code blocks have been modified".to_string()));
    assert!(analysis.contains(&"Links have been modified".to_string()));
}

#[test]
fn test_default_analyzer() {
    let analyzer = get_analyzer("unknown.xyz");
    let change = FileChange {
        status: "M".to_string(),
        diff: "Some changes".to_string(),
    };

    let analysis = analyzer.analyze("unknown.xyz", &change);
    println!(
        "Default Analyzer Test Debug: Analysis results: {:?}",
        analysis
    );
    assert!(analysis.is_empty());
}

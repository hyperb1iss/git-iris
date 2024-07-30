use git_iris::context::{ChangeType, StagedFile};
use git_iris::file_analyzers::get_analyzer;

#[test]
fn test_rust_analyzer() {
    let analyzer = get_analyzer("test.rs");
    let change = StagedFile {
        path: "test.rs".to_string(),
        change_type: ChangeType::Modified,
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
        analysis: Vec::new(),
    };

    let analysis = analyzer.analyze("test.rs", &change);
    println!("Rust Test Debug: Analysis results: {:?}", analysis);
    assert!(analysis.contains(&"Modified functions: new_function".to_string()));
    assert!(analysis.contains(&"Modified structs: OldStruct, NewStruct".to_string()));
}

#[test]
fn test_javascript_analyzer() {
    let analyzer = get_analyzer("test.js");
    let change = StagedFile {
        path: "test.js".to_string(),
        change_type: ChangeType::Modified,
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
        analysis: Vec::new(),
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
    let change = StagedFile {
        path: "test.py".to_string(),
        change_type: ChangeType::Modified,
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
        analysis: Vec::new(),
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
    let change = StagedFile {
        path: "test.yaml".to_string(),
        change_type: ChangeType::Modified,
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
        analysis: Vec::new(),
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
    let change = StagedFile {
        path: "test.json".to_string(),
        change_type: ChangeType::Modified,
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
        analysis: Vec::new(),
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
    let change = StagedFile {
        path: "test.md".to_string(),
        change_type: ChangeType::Modified,
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
        analysis: Vec::new(),
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
    let change = StagedFile {
        path: "unknown.xyz".to_string(),
        change_type: ChangeType::Modified,
        diff: "Some changes".to_string(),
        analysis: Vec::new(),
    };

    let analysis = analyzer.analyze("unknown.xyz", &change);
    println!(
        "Default Analyzer Test Debug: Analysis results: {:?}",
        analysis
    );
    assert!(analysis.is_empty());
}

#[test]
fn test_java_analyzer() {
    let analyzer = get_analyzer("test.java");
    let change = StagedFile {
        path: "test.java".to_string(),
        change_type: ChangeType::Modified,
        diff: r#"
+public class NewClass {
+    public void newMethod() {
+        System.out.println("Hello, World!");
+    }
+}
-private class OldClass {
-    private void oldMethod() {
-        // Do nothing
-    }
-}
+import java.util.List;
-import java.util.ArrayList;
        "#
        .to_string(),
        analysis: Vec::new(),
    };

    let analysis = analyzer.analyze("test.java", &change);
    println!("Java analysis results: {:?}", analysis);

    // Check for modified classes
    let class_analysis = analysis
        .iter()
        .find(|&s| s.starts_with("Modified classes:"))
        .expect("No class analysis found");
    assert!(class_analysis.contains("NewClass"), "NewClass not detected");
    assert!(class_analysis.contains("OldClass"), "OldClass not detected");

    // Check for modified methods
    let method_analysis = analysis
        .iter()
        .find(|&s| s.starts_with("Modified methods:"))
        .expect("No method analysis found");
    assert!(
        method_analysis.contains("newMethod"),
        "newMethod not detected"
    );
    assert!(
        method_analysis.contains("oldMethod"),
        "oldMethod not detected"
    );

    // Check for import changes
    assert!(
        analysis.contains(&"Import statements have been modified".to_string()),
        "Failed to detect import changes. Analysis: {:?}",
        analysis
    );
}

#[test]
fn test_kotlin_analyzer() {
    let analyzer = get_analyzer("test.kt");
    let change = StagedFile {
        path: "test.kt".to_string(),
        change_type: ChangeType::Modified,
        diff: r#"
+class NewClass {
+    fun newFunction() {
+        println("Hello, Kotlin!")
+    }
+}
-object OldObject {
-    fun oldFunction() {
-        // Do nothing
-    }
-}
+import kotlin.collections.List
-import kotlin.collections.ArrayList
        "#
        .to_string(),
        analysis: Vec::new(),
    };

    let analysis = analyzer.analyze("test.kt", &change);
    println!("Kotlin analysis results: {:?}", analysis);

    // Helper function to check if any string in the analysis contains all expected substrings
    fn contains_all_substrings(analysis: &Vec<String>, substrings: &[&str]) -> bool {
        analysis
            .iter()
            .any(|s| substrings.iter().all(|&sub| s.contains(sub)))
    }

    // Check for the presence of the entire expected strings
    assert!(contains_all_substrings(
        &analysis,
        &["NewClass", "OldObject"]
    ));
    assert!(contains_all_substrings(
        &analysis,
        &["newFunction", "oldFunction"]
    ));
    assert!(analysis.contains(&"Import statements have been modified".to_string()));
}

#[test]
fn test_gradle_analyzer() {
    let analyzer = get_analyzer("build.gradle");
    let change = StagedFile {
        path: "build.gradle".to_string(),
        change_type: ChangeType::Modified,
        diff: r#"
+    implementation 'com.example:new-library:1.0.0'
-    implementation 'com.example:old-library:0.9.0'
+plugins {
+    id 'com.android.application'
+}
-apply plugin: 'java'
+task newTask {
+    doLast {
+        println 'Executing new task'
+    }
+}
        "#
        .to_string(),
        analysis: Vec::new(),
    };

    let analysis = analyzer.analyze("build.gradle", &change);
    assert!(analysis.contains(&"Dependencies have been modified".to_string()));
    assert!(analysis.contains(&"Plugins have been modified".to_string()));
    assert!(analysis.contains(&"Tasks have been modified".to_string()));
}

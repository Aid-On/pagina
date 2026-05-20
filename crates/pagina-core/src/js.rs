/// JavaScript execution via Boa engine.
///
/// Executes `<script>` blocks before layout, allowing dynamic content generation.
/// The DOM is exposed as a simplified `document` object.

use boa_engine::{Context, Source};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

/// Execute all `<script>` elements in the DOM.
/// Returns a list of text insertions that scripts requested via `document.write()`.
pub fn execute_scripts(dom: &RcDom) -> Vec<String> {
    let scripts = extract_scripts(&dom.document);
    if scripts.is_empty() {
        return Vec::new();
    }

    let mut context = Context::default();
    let mut writes: Vec<String> = Vec::new();

    // Provide a minimal `document.write()` implementation
    // We capture writes and return them for the caller to inject into the DOM.
    register_document_api(&mut context, &mut writes);

    for script in &scripts {
        let _ = context.eval(Source::from_bytes(script.as_bytes()));
    }

    writes
}

/// Extract text content from all `<script>` elements.
fn extract_scripts(handle: &Handle) -> Vec<String> {
    let mut scripts = Vec::new();
    collect_scripts(handle, &mut scripts);
    scripts
}

fn collect_scripts(handle: &Handle, scripts: &mut Vec<String>) {
    if let NodeData::Element { name, .. } = &handle.data {
        if name.local.as_ref() == "script" {
            let mut js = String::new();
            for child in handle.children.borrow().iter() {
                if let NodeData::Text { contents } = &child.data {
                    js.push_str(&contents.borrow());
                }
            }
            if !js.is_empty() {
                scripts.push(js);
            }
            return;
        }
    }
    for child in handle.children.borrow().iter() {
        collect_scripts(child, scripts);
    }
}

fn register_document_api(context: &mut Context, _writes: &mut Vec<String>) {
    // Create a minimal `document` object with basic methods.
    // For now we only support `document.title` as a simple string property.
    // Full DOM manipulation would require a much more complex bridge.

    let script = r#"
        var document = {
            title: '',
            _writes: [],
            write: function(s) { this._writes.push(String(s)); },
            writeln: function(s) { this._writes.push(String(s) + '\n'); },
            getElementById: function(id) { return null; },
            querySelector: function(sel) { return null; },
        };
        var window = { document: document };
        var console = {
            log: function() {},
            warn: function() {},
            error: function() {},
        };
    "#;

    let _ = context.eval(Source::from_bytes(script.as_bytes()));
}

/// After executing scripts, extract any `document.write()` output.
pub fn extract_document_writes(context: &mut Context) -> Vec<String> {
    let result = context.eval(Source::from_bytes(b"JSON.stringify(document._writes)"));
    match result {
        Ok(val) => {
            let s = val.to_string(context).ok().map(|js| js.to_std_string_escaped()).unwrap_or_default();
            // Parse JSON array
            parse_json_string_array(&s)
        }
        Err(_) => Vec::new(),
    }
}

fn parse_json_string_array(s: &str) -> Vec<String> {
    let s = s.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        return Vec::new();
    }
    let inner = &s[1..s.len() - 1];
    if inner.is_empty() {
        return Vec::new();
    }
    // Simple JSON string array parser
    let mut items = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escape = false;
    for ch in inner.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if ch == ',' && !in_string {
            items.push(std::mem::take(&mut current));
            continue;
        }
        if in_string {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        items.push(current);
    }
    items
}

/// High-level: execute scripts and return generated HTML fragments.
pub fn run_scripts(dom: &RcDom) -> Vec<String> {
    let scripts = extract_scripts(&dom.document);
    if scripts.is_empty() {
        return Vec::new();
    }

    let mut context = Context::default();
    register_document_api(&mut context, &mut Vec::new());

    for script in &scripts {
        let _ = context.eval(Source::from_bytes(script.as_bytes()));
    }

    extract_document_writes(&mut context)
}

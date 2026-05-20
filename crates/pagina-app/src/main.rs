use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

// ─── CLI ─────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "pagina-app", about = "Pagina document management")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Data directory (default: ~/.pagina)
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    /// Manage clause template components
    #[command(subcommand)]
    Component(ComponentCmd),

    /// Manage document templates (preset compositions)
    #[command(subcommand)]
    Template(TemplateCmd),

    /// Generate a PDF document
    Generate {
        /// Template name
        template: String,
        /// Output PDF path
        #[arg(short, long)]
        output: PathBuf,
        /// Party definitions: "甲=株式会社Example"
        #[arg(long = "party", value_parser = parse_party)]
        parties: Vec<(String, String)>,
        /// Parameter overrides: "payment.金額=500000"
        #[arg(long = "set", value_parser = parse_override)]
        overrides: Vec<(String, String, String)>,
        /// Contract date
        #[arg(long, default_value = "")]
        date: String,
        /// External font files
        #[arg(long = "font")]
        fonts: Vec<PathBuf>,
    },

    /// Manage generated documents
    #[command(subcommand)]
    Document(DocumentCmd),

    /// Initialize data directory with bundled templates
    Init,
}

#[derive(Subcommand)]
enum ComponentCmd {
    /// List all components
    List,
    /// Show component content and parameters
    Show { name: String },
    /// Create a new component
    New { name: String },
}

#[derive(Subcommand)]
enum TemplateCmd {
    /// List all templates
    List,
    /// Show template composition and parameters
    Show { name: String },
    /// Create a new template
    New { name: String },
}

#[derive(Subcommand)]
enum DocumentCmd {
    /// List generated documents
    List,
    /// Show document details
    Show { id: String },
}

fn parse_party(s: &str) -> Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .ok_or_else(|| format!("expected KEY=VALUE, got: {s}"))
}

fn parse_override(s: &str) -> Result<(String, String, String), String> {
    let (template_key, value) = s.split_once('=')
        .ok_or_else(|| format!("expected TEMPLATE.KEY=VALUE, got: {s}"))?;
    let (template, key) = template_key.split_once('.')
        .ok_or_else(|| format!("expected TEMPLATE.KEY=VALUE, got: {s}"))?;
    Ok((template.to_string(), key.to_string(), value.to_string()))
}

// ─── Data directory ──────────────────────────────────

struct AppData {
    root: PathBuf,
}

impl AppData {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn components_dir(&self) -> PathBuf { self.root.join("components") }
    fn templates_dir(&self) -> PathBuf { self.root.join("templates") }
    fn documents_dir(&self) -> PathBuf { self.root.join("documents") }
    fn fonts_dir(&self) -> PathBuf { self.root.join("fonts") }

    fn ensure_dirs(&self) {
        for dir in [self.components_dir(), self.templates_dir(), self.documents_dir(), self.fonts_dir()] {
            let _ = fs::create_dir_all(&dir);
        }
    }

    fn list_files(&self, dir: &Path, ext: &str) -> Vec<String> {
        let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
        entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.strip_suffix(ext).map(|s| s.to_string())
            })
            .collect()
    }
}

// ─── Document metadata ───────────────────────────────

#[derive(Serialize, Deserialize)]
struct DocumentMeta {
    id: String,
    template: String,
    date: String,
    created_at: String,
    parties: HashMap<String, String>,
    output_file: String,
}

// ─── Commands ────────────────────────────────────────

fn cmd_init(data: &AppData) {
    data.ensure_dirs();

    // Copy bundled components
    let bundled_dir = Path::new("templates/contract");
    if bundled_dir.exists() {
        copy_dir_contents(&bundled_dir.join("clauses"), &data.components_dir());
        copy_dir_contents(&bundled_dir.join("presets"), &data.templates_dir());
        if let Ok(fonts_toml) = fs::read_to_string(bundled_dir.join("fonts.toml")) {
            let _ = fs::write(data.root.join("fonts.toml"), fonts_toml);
        }
    }

    println!("Initialized {}", data.root.display());
    println!("  components/  {} items", data.list_files(&data.components_dir(), ".html").len());
    println!("  templates/   {} items", data.list_files(&data.templates_dir(), ".toml").len());
}

fn copy_dir_contents(src: &Path, dst: &Path) {
    let Ok(entries) = fs::read_dir(src) else { return };
    let _ = fs::create_dir_all(dst);
    for entry in entries.flatten() {
        let dest = dst.join(entry.file_name());
        let _ = fs::copy(entry.path(), dest);
    }
}

fn cmd_component_list(data: &AppData) {
    let components = data.list_files(&data.components_dir(), ".html");
    if components.is_empty() {
        println!("No components. Run `pagina-app init` first.");
        return;
    }
    println!("Components ({}):", components.len());
    for name in &components {
        // Extract parameters from template
        let path = data.components_dir().join(format!("{name}.html"));
        let params = extract_template_params(&path);
        if params.is_empty() {
            println!("  {name}");
        } else {
            println!("  {name}  params: {}", params.join(", "));
        }
    }
}

fn cmd_component_show(data: &AppData, name: &str) {
    let path = data.components_dir().join(format!("{name}.html"));
    match fs::read_to_string(&path) {
        Ok(content) => {
            let params = extract_template_params_from_str(&content);
            println!("=== Component: {name} ===");
            if !params.is_empty() {
                println!("Parameters: {}", params.join(", "));
            }
            println!("---");
            println!("{content}");
        }
        Err(_) => println!("Component not found: {name}"),
    }
}

fn cmd_component_new(data: &AppData, name: &str) {
    let path = data.components_dir().join(format!("{name}.html"));
    if path.exists() {
        println!("Component already exists: {name}");
        return;
    }
    let template = "<p>{{パラメータ名|デフォルト値}}</p>\n";
    let _ = fs::write(&path, template);
    println!("Created component: {}", path.display());
}

fn cmd_template_list(data: &AppData) {
    let templates = data.list_files(&data.templates_dir(), ".toml");
    if templates.is_empty() {
        println!("No templates. Run `pagina-app init` first.");
        return;
    }
    println!("Templates ({}):", templates.len());
    for name in &templates {
        let path = data.templates_dir().join(format!("{name}.toml"));
        let desc = read_template_name(&path).unwrap_or_default();
        println!("  {name}  {desc}");
    }
}

fn cmd_template_show(data: &AppData, name: &str) {
    let path = data.templates_dir().join(format!("{name}.toml"));
    let Ok(text) = fs::read_to_string(&path) else {
        println!("Template not found: {name}");
        return;
    };

    #[derive(Deserialize)]
    struct Preset { name: String, clauses: Vec<PresetClause> }
    #[derive(Deserialize)]
    struct PresetClause { title: String, template: String, #[serde(default)] defaults: HashMap<String, String> }

    let Ok(preset) = toml::from_str::<Preset>(&text) else {
        println!("Failed to parse template: {name}");
        return;
    };

    println!("=== Template: {} ===", preset.name);
    println!("{} clauses:", preset.clauses.len());
    for (i, c) in preset.clauses.iter().enumerate() {
        let params_str = if c.defaults.is_empty() {
            String::new()
        } else {
            let ps: Vec<String> = c.defaults.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            format!("  [{}]", ps.join(", "))
        };
        println!("  {}. {} ({}){}", i + 1, c.title, c.template, params_str);
    }
}

fn cmd_template_new(data: &AppData, name: &str) {
    let path = data.templates_dir().join(format!("{name}.toml"));
    if path.exists() {
        println!("Template already exists: {name}");
        return;
    }
    let template = format!("name = \"{name}\"\n\n[[clauses]]\ntitle = \"条項名\"\ntemplate = \"component-name\"\n[clauses.defaults]\n");
    let _ = fs::write(&path, template);
    println!("Created template: {}", path.display());
}

fn cmd_generate(
    data: &AppData,
    template: &str,
    output: &Path,
    parties_raw: &[(String, String)],
    overrides_raw: &[(String, String, String)],
    date: &str,
    font_paths: &[PathBuf],
) {
    // Build the document TOML dynamically
    let mut doc = String::new();
    doc.push_str("[document]\n");
    doc.push_str(&format!("title = \"\"\n"));
    doc.push_str(&format!("date = \"{date}\"\n"));
    doc.push_str(&format!("preset = \"{template}\"\n\n"));

    for (role, name) in parties_raw {
        doc.push_str(&format!("[parties.\"{role}\"]\nname = \"{name}\"\n\n"));
    }

    // Group overrides by template
    let mut override_map: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for (tmpl, key, val) in overrides_raw {
        override_map.entry(tmpl.clone()).or_default().push((key.clone(), val.clone()));
    }
    for (tmpl, kvs) in &override_map {
        doc.push_str(&format!("[overrides.\"{tmpl}\"]\n"));
        for (k, v) in kvs {
            doc.push_str(&format!("\"{k}\" = \"{v}\"\n"));
        }
        doc.push_str("\n");
    }

    doc.push_str("[signature]\nstyle = \"seal\"\n");

    // Write temp config
    let config_path = data.root.join(".tmp_generate.toml");
    let _ = fs::write(&config_path, &doc);

    // Generate using pagina-compose logic
    let html = compose_from_config(&config_path, &data.templates_dir(), &data.components_dir());

    let mut all_fonts: Vec<String> = font_paths.iter().map(|p| p.display().to_string()).collect();
    // Auto-download fonts if fonts.toml exists
    let auto_fonts = ensure_fonts_from_config(&data.root);
    all_fonts.extend(auto_fonts.iter().map(|p| p.display().to_string()));

    let font_refs: Vec<&str> = all_fonts.iter().map(|s| s.as_str()).collect();
    let pdf_bytes = pagina_core::convert_with_fonts(&html, &font_refs);

    let _ = fs::write(output, &pdf_bytes);
    let _ = fs::remove_file(&config_path);

    // Save document record
    let doc_id = format!("{}-{}", chrono_date(), template);
    let meta = DocumentMeta {
        id: doc_id.clone(),
        template: template.to_string(),
        date: date.to_string(),
        created_at: chrono_date(),
        parties: parties_raw.iter().cloned().collect(),
        output_file: output.display().to_string(),
    };
    let meta_dir = data.documents_dir().join(&doc_id);
    let _ = fs::create_dir_all(&meta_dir);
    let _ = fs::write(meta_dir.join("meta.toml"), toml::to_string_pretty(&meta).unwrap_or_default());

    println!("wrote {}", output.display());
    println!("document id: {doc_id}");
}

fn cmd_document_list(data: &AppData) {
    let Ok(entries) = fs::read_dir(data.documents_dir()) else {
        println!("No documents.");
        return;
    };
    let mut docs: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    docs.sort();
    docs.reverse();

    if docs.is_empty() {
        println!("No documents.");
        return;
    }

    println!("Documents ({}):", docs.len());
    for id in &docs {
        let meta_path = data.documents_dir().join(id).join("meta.toml");
        let info = fs::read_to_string(&meta_path)
            .ok()
            .and_then(|t| toml::from_str::<DocumentMeta>(&t).ok());
        if let Some(meta) = info {
            println!("  {}  template={}  output={}", id, meta.template, meta.output_file);
        } else {
            println!("  {id}");
        }
    }
}

fn cmd_document_show(data: &AppData, id: &str) {
    let meta_path = data.documents_dir().join(id).join("meta.toml");
    match fs::read_to_string(&meta_path) {
        Ok(text) => println!("{text}"),
        Err(_) => println!("Document not found: {id}"),
    }
}

// ─── Compose helpers (reuse pagina-compose logic) ────

fn compose_from_config(config_path: &Path, templates_dir: &Path, components_dir: &Path) -> String {
    let config_text = fs::read_to_string(config_path).unwrap_or_default();

    #[derive(Deserialize)]
    struct Doc { document: DocMeta, #[serde(default)] parties: HashMap<String, Party>, #[serde(default)] overrides: HashMap<String, HashMap<String, String>>, #[serde(default)] signature: Option<SigDef> }
    #[derive(Deserialize)]
    struct DocMeta { #[serde(default)] title: String, #[serde(default)] date: String, #[serde(default)] preset: String }
    #[derive(Deserialize)]
    struct Party { name: String, #[serde(default)] address: String, #[serde(default)] representative: String }
    #[derive(Deserialize)]
    struct SigDef { #[serde(default)] style: String }
    #[derive(Deserialize)]
    struct Preset { name: String, #[serde(default)] clauses: Vec<PresetClause> }
    #[derive(Deserialize, Clone)]
    struct PresetClause { title: String, #[serde(default)] template: String, #[serde(default)] defaults: HashMap<String, String> }

    let doc: Doc = toml::from_str(&config_text).unwrap_or_else(|e| {
        eprintln!("config parse error: {e}");
        std::process::exit(1);
    });

    // Load preset
    let preset_path = templates_dir.join(format!("{}.toml", doc.document.preset));
    let preset_text = fs::read_to_string(&preset_path).unwrap_or_default();
    let preset: Preset = toml::from_str(&preset_text).unwrap_or_else(|_| Preset { name: String::new(), clauses: Vec::new() });

    let title = if doc.document.title.is_empty() { &preset.name } else { &doc.document.title };

    let css = default_contract_css();
    let mut html = format!("<!DOCTYPE html>\n<html><head><style>\n{css}\n</style></head><body>\n");
    html.push_str(&format!("<h1>{title}</h1>\n"));

    if !doc.document.date.is_empty() {
        html.push_str(&format!("<p class=\"date\">{}</p>\n", doc.document.date));
    }

    // Preamble
    let mut parties: Vec<(&String, &Party)> = doc.parties.iter().collect();
    parties.sort_by(|(a, _), (b, _)| party_order(a).cmp(&party_order(b)));
    let names: Vec<String> = parties.iter()
        .map(|(role, p)| format!("{}（以下「{}」という）", p.name, role))
        .collect();
    html.push_str(&format!("<p class=\"preamble\">{}は、以下のとおり契約を締結する。</p>\n", names.join("と")));

    // Clauses
    for (i, pc) in preset.clauses.iter().enumerate() {
        let mut params = pc.defaults.clone();
        if let Some(ovr) = doc.overrides.get(&pc.template) {
            for (k, v) in ovr { params.insert(k.clone(), v.clone()); }
        }
        let body = load_component(components_dir, &pc.template)
            .unwrap_or_default();
        let body = replace_params(&body, &params);
        html.push_str(&format!("<div class=\"clause\">\n<h3>第{}条（{}）</h3>\n{}\n</div>\n", i + 1, pc.title, body));
    }

    // Signature
    let num = doc.parties.len();
    html.push_str(&format!("<div class=\"sig-area\">\n<p>以上のとおり合意が成立したので、本書面を{}通作成し、甲乙それぞれ1通を保持する。</p>\n", num));
    if !doc.document.date.is_empty() {
        html.push_str(&format!("<p class=\"sig-date\">{}</p>\n", doc.document.date));
    }
    let labels: Vec<&str> = parties.iter().map(|(r, _)| r.as_str()).collect();
    let sep = "\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}\u{3000}";
    html.push_str(&format!("<p class=\"sig-parties\">{}</p>\n</div>\n", labels.join(sep)));

    html.push_str("</body></html>");
    html
}

fn load_component(dir: &Path, name: &str) -> Option<String> {
    fs::read_to_string(dir.join(format!("{name}.html"))).ok()
}

fn replace_params(template: &str, params: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in params {
        result = result.replace(&format!("{{{{{key}}}}}"), value);
    }
    // Default values: {{key|default}}
    while let Some(start) = result.find("{{") {
        let rest = &result[start + 2..];
        let Some(end) = rest.find("}}") else { break };
        let token = &rest[..end];
        let replacement = if let Some((key, default)) = token.split_once('|') {
            params.get(key).map(|s| s.as_str()).unwrap_or(default)
        } else {
            params.get(token).map(|s| s.as_str()).unwrap_or(token)
        };
        result = format!("{}{}{}", &result[..start], replacement, &rest[end + 2..]);
    }
    result
}

fn party_order(name: &str) -> (u8, String) {
    let p = match name { "甲" => 0, "乙" => 1, "丙" => 2, "丁" => 3, _ => 10 };
    (p, name.to_string())
}

fn extract_template_params(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .map(|s| extract_template_params_from_str(&s))
        .unwrap_or_default()
}

fn extract_template_params_from_str(content: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("{{") {
        let inner = &rest[start + 2..];
        if let Some(end) = inner.find("}}") {
            let token = &inner[..end];
            let name = token.split('|').next().unwrap_or(token).trim();
            if !name.is_empty() && !params.contains(&name.to_string()) {
                params.push(name.to_string());
            }
            rest = &inner[end + 2..];
        } else {
            break;
        }
    }
    params
}

fn read_template_name(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("name = ") {
            return Some(name.trim_matches('"').to_string());
        }
    }
    None
}

fn ensure_fonts_from_config(root: &Path) -> Vec<PathBuf> {
    let fonts_toml = root.join("fonts.toml");
    let Ok(text) = fs::read_to_string(&fonts_toml) else { return Vec::new() };

    #[derive(Deserialize)]
    struct FC { #[serde(default)] fonts: Vec<FE> }
    #[derive(Deserialize)]
    struct FE { url: String, file: String, #[allow(dead_code)] family: String }

    let Ok(config) = toml::from_str::<FC>(&text) else { return Vec::new() };
    let cache = root.join("fonts");
    let _ = fs::create_dir_all(&cache);

    config.fonts.iter().filter_map(|entry| {
        let path = cache.join(&entry.file);
        if !path.exists() {
            eprintln!("Downloading font {}...", entry.file);
            let ok = std::process::Command::new("curl")
                .args(["-fsSL", "-o"]).arg(&path).arg(&entry.url)
                .status().map(|s| s.success()).unwrap_or(false);
            if !ok { return None; }
        }
        Some(path)
    }).collect()
}

fn default_contract_css() -> &'static str {
    r#"
@page { size: A4; margin: 25mm 20mm 30mm 20mm; }
@page :first { @top-center { content: none; } }
body { font-size: 10.5pt; line-height: 1.8; color: #222; font-family: "NotoSansCJKjp-Regular", Helvetica; }
h1 { font-size: 18pt; text-align: center; margin-top: 20mm; margin-bottom: 8mm; }
.date { text-align: right; margin-bottom: 5mm; }
.preamble { margin-bottom: 8mm; }
.clause { margin-bottom: 5mm; }
.clause h3 { font-size: 11pt; margin-bottom: 2mm; }
.clause p { margin-bottom: 2mm; }
.clause ol, .clause ul { margin-bottom: 2mm; }
.sig-area { margin-top: 12mm; }
.sig-date { margin-top: 8mm; }
.sig-parties { margin-top: 8mm; }
"#
}

fn chrono_date() -> String {
    "2026-05-20".to_string()
}

// ─── Main ────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let root = cli.data_dir.unwrap_or_else(|| {
        dirs_home().join(".pagina")
    });
    let data = AppData::new(root);

    match cli.command {
        Command::Init => cmd_init(&data),
        Command::Component(cmd) => match cmd {
            ComponentCmd::List => cmd_component_list(&data),
            ComponentCmd::Show { name } => cmd_component_show(&data, &name),
            ComponentCmd::New { name } => cmd_component_new(&data, &name),
        },
        Command::Template(cmd) => match cmd {
            TemplateCmd::List => cmd_template_list(&data),
            TemplateCmd::Show { name } => cmd_template_show(&data, &name),
            TemplateCmd::New { name } => cmd_template_new(&data, &name),
        },
        Command::Generate { template, output, parties, overrides, date, fonts } => {
            cmd_generate(&data, &template, &output, &parties, &overrides, &date, &fonts);
        },
        Command::Document(cmd) => match cmd {
            DocumentCmd::List => cmd_document_list(&data),
            DocumentCmd::Show { id } => cmd_document_show(&data, &id),
        },
    }
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"))
}

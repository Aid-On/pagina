use std::fs;
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "pagina", about = "HTML + CSS Paged Media → PDF")]
struct Cli {
    /// Input HTML file
    input: PathBuf,

    /// Output PDF file (default: <input>.pdf)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let html = fs::read_to_string(&cli.input).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {e}", cli.input.display());
        std::process::exit(1);
    });

    let pdf_bytes = pagina_core::convert(&html);

    let output = cli.output.unwrap_or_else(|| cli.input.with_extension("pdf"));
    fs::write(&output, &pdf_bytes).unwrap_or_else(|e| {
        eprintln!("Error writing {}: {e}", output.display());
        std::process::exit(1);
    });

    eprintln!("wrote {}", output.display());
}

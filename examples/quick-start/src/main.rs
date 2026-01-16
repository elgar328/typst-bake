use typst_bake::{IntoDict, IntoValue};

#[derive(IntoValue, IntoDict)]
struct Inputs {
    templates_original: usize,
    templates_compressed: usize,
    templates_count: usize,
    fonts_original: usize,
    fonts_compressed: usize,
    fonts_count: usize,
    packages_original: usize,
    packages_compressed: usize,
    packages_count: usize,
    total_original: usize,
    total_compressed: usize,
}

fn save_pdf(data: &[u8], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::write(out_dir.join(filename), data)?;
    println!("Generated {} ({} bytes)", filename, data.len());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = typst_bake::document!("main.typ");

    // Display compression statistics
    println!();
    doc.stats().display();
    println!();

    let stats = doc.stats();
    let inputs = Inputs {
        templates_original: stats.templates.original_size,
        templates_compressed: stats.templates.compressed_size,
        templates_count: stats.templates.file_count,
        fonts_original: stats.fonts.original_size,
        fonts_compressed: stats.fonts.compressed_size,
        fonts_count: stats.fonts.file_count,
        packages_original: stats.packages.total_original,
        packages_compressed: stats.packages.total_compressed,
        packages_count: stats.packages.packages.len(),
        total_original: stats.total_original(),
        total_compressed: stats.total_compressed(),
    };

    let pdf = doc.with_inputs(inputs).to_pdf()?;

    save_pdf(&pdf, "output.pdf")
}

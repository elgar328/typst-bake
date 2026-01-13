fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf = typst_bake::document!("main.typ")
        .with_font(include_bytes!("../fonts/SourceSerif4-Regular.ttf"))
        .with_font(include_bytes!("../fonts/STIXTwoMath-Regular.otf"))
        .to_pdf()?;

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::write(out_dir.join("output.pdf"), &pdf)?;
    println!("Generated package-example.pdf ({} bytes)", pdf.len());

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf = typst_bake::document!("main.typ")
        .with_font(include_bytes!("../fonts/Inter_18pt-Light.ttf"))
        .with_font(include_bytes!("../fonts/Inter_28pt-Bold.ttf"))
        .with_font(include_bytes!("../fonts/STIXTwoMath-Regular.otf"))
        .with_font(include_bytes!("../fonts/JetBrainsMono-Regular.otf"))
        .to_pdf()?;

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::write(out_dir.join("output.pdf"), &pdf)?;
    println!("Generated output.pdf ({} bytes)", pdf.len());

    Ok(())
}

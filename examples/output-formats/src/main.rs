fn main() -> Result<(), Box<dyn std::error::Error>> {
    use typst_bake::{PdfConfig, PdfStandard, PdfTimestamp};

    // Apply PDF export options: untagged (smaller file, bookmarks preserved) and
    // conforming to PDF/A-2b. PDF/A requires a document date; a fixed date keeps
    // this example's output reproducible across builds.
    let doc = typst_bake::document!("main.typ").with_pdf_config(PdfConfig {
        tagged: false,
        standard: PdfStandard::A2b,
        timestamp: Some(PdfTimestamp::utc(2026, 1, 1, 0, 0, 0).expect("valid date")),
        ..Default::default()
    });

    // PDF - single file for all pages (with the options above)
    let pdf = doc.to_pdf()?;
    save_file(&pdf, "output.pdf")?;

    // SVG - one file per page
    let svgs = doc.to_svg()?;
    for (i, svg) in svgs.iter().enumerate() {
        save_file(svg.as_bytes(), &format!("output_{}.svg", i + 1))?;
    }

    // PNG - one file per page (144 DPI)
    let pngs = doc.to_png(144.0)?;
    for (i, png) in pngs.iter().enumerate() {
        save_file(png, &format!("output_{}.png", i + 1))?;
    }

    Ok(())
}

fn save_file(data: &[u8], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::write(out_dir.join(filename), data)?;
    println!("Generated {} ({} bytes)", filename, data.len());
    Ok(())
}

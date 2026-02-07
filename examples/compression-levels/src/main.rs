fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = typst_bake::document!("main.typ");
    println!();
    print!("{}", doc.stats());
    println!();

    let pdf = doc.to_pdf()?;
    save_pdf(&pdf, "output.pdf")
}

fn save_pdf(data: &[u8], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::write(out_dir.join(filename), data)?;
    println!("Generated {} ({} bytes)", filename, data.len());
    Ok(())
}

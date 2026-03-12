fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://elgar328.github.io/typst-bake/with-package.pdf";
    println!("Downloading {url}");

    let pdf_bytes: Vec<u8> = ureq::get(url).call()?.body_mut().read_to_vec()?;
    println!("Downloaded {} bytes", pdf_bytes.len());

    let pdf = typst_bake::document!("main.typ")
        .add_file("downloaded.pdf", pdf_bytes)?
        .to_pdf()?;

    save_pdf(&pdf, "output.pdf")
}

fn save_pdf(data: &[u8], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::write(out_dir.join(filename), data)?;
    println!("Generated {} ({} bytes)", filename, data.len());
    Ok(())
}

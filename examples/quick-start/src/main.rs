use typst_bake::{IntoDict, IntoValue};

#[derive(IntoValue, IntoDict)]
struct Inputs {
    fonts_size: u64,
    files_size: u64,
}

fn dir_size(dir: &str) -> u64 {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

fn save_pdf(data: &[u8], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::write(out_dir.join(filename), data)?;
    println!("Generated {} ({} bytes)", filename, data.len());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let inputs = Inputs {
        fonts_size: dir_size(&format!("{}/fonts", manifest_dir)),
        files_size: dir_size(&format!("{}/templates", manifest_dir)),
    };

    let pdf = typst_bake::document!("main.typ")
        .with_inputs(inputs.into_dict())
        .to_pdf()?;

    save_pdf(&pdf, "output.pdf")
}

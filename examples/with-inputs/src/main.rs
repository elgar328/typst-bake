use chrono::Local;
use typst_bake::{IntoDict, IntoValue};

#[derive(IntoValue, IntoDict)]
struct Inputs {
    number: String,
    date: String,
    customer: String,
    items: Vec<Item>,
    total: f64,
}

#[derive(IntoValue)]
struct Item {
    description: String,
    quantity: i32,
    price: f64,
    amount: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let items = vec![
        Item {
            description: "Web Development".into(),
            quantity: 40,
            price: 75.0,
            amount: 3000.0,
        },
        Item {
            description: "UI/UX Design".into(),
            quantity: 20,
            price: 85.0,
            amount: 1700.0,
        },
        Item {
            description: "Server Setup".into(),
            quantity: 8,
            price: 100.0,
            amount: 800.0,
        },
    ];

    let total = items.iter().map(|i| i.amount).sum();

    let now = Local::now();
    let invoice = Inputs {
        number: format!("INV-{}-001", now.format("%Y")),
        date: now.format("%Y-%m-%d").to_string(),
        customer: "Acme Corporation".into(),
        items,
        total,
    };

    let pdf = typst_bake::document!("main.typ")
        .with_inputs(invoice.into_dict())
        .to_pdf()?;
    save_pdf(&pdf, "output.pdf")
}

fn save_pdf(data: &[u8], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::write(out_dir.join(filename), data)?;
    println!("Generated {} ({} bytes)", filename, data.len());
    Ok(())
}

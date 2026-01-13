use chrono::Local;
use derive_typst_intoval::{IntoDict, IntoValue};
use typst::foundations::IntoValue as _;

#[derive(IntoValue, IntoDict)]
struct ShoppingList {
    title: String,
    date: String,
    items: Vec<Item>,
}

#[derive(IntoValue)]
struct Item {
    name: String,
    quantity: i32,
    category: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let items = vec![
        Item {
            name: "Milk".into(),
            quantity: 2,
            category: "Dairy".into(),
        },
        Item {
            name: "Eggs".into(),
            quantity: 12,
            category: "Dairy".into(),
        },
        Item {
            name: "Bread".into(),
            quantity: 1,
            category: "Bakery".into(),
        },
        Item {
            name: "Apples".into(),
            quantity: 6,
            category: "Fruits".into(),
        },
        Item {
            name: "Chicken".into(),
            quantity: 1,
            category: "Meat".into(),
        },
    ];

    let shopping_list = ShoppingList {
        title: "Weekly Groceries".into(),
        date: Local::now().format("%Y-%m-%d").to_string(),
        items,
    };

    let pdf = typst_bake::document!("main.typ")
        .with_inputs(shopping_list.into_dict())
        .to_pdf()?;

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::write(out_dir.join("output.pdf"), &pdf)?;
    println!("Generated output.pdf ({} bytes)", pdf.len());

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    // Phase 1: Generate outputs from numbers.typ

    let doc = typst_bake::document!("numbers.typ");

    // All pages as PDF
    let all_pdf = doc.to_pdf()?;
    assert!(all_pdf.starts_with(b"%PDF"), "Invalid PDF header");
    println!("all_pdf: {} bytes", all_pdf.len());

    // Verify page count
    let count = doc.page_count()?;
    assert_eq!(count, 5, "Expected 5 pages, got {count}");
    println!("page_count: {count}");

    // All pages as SVG
    let all_svgs = doc.to_svg()?;
    assert_eq!(all_svgs.len(), 5, "Expected 5 SVGs");
    for svg in &all_svgs {
        assert!(svg.contains("<svg"), "Invalid SVG content");
    }
    println!("all_svgs: {} pages", all_svgs.len());

    // All pages as PNG
    let all_pngs = doc.to_png(144.0)?;
    assert_eq!(all_pngs.len(), 5, "Expected 5 PNGs");
    for png in &all_pngs {
        assert!(
            png.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
            "Invalid PNG header"
        );
    }
    println!("all_pngs: {} pages", all_pngs.len());

    // Single page selection — page 1 (PDF)
    let page1_pdf = doc.select_pages([0]).to_pdf()?;
    assert!(page1_pdf.starts_with(b"%PDF"), "Invalid PDF header");
    println!("page1_pdf: ok");

    // Single page selection — page 2 (PNG)
    let page2_png = doc.select_pages([1]).to_png(144.0)?;
    assert_eq!(page2_png.len(), 1);
    assert!(
        page2_png[0].starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "Invalid PNG header"
    );
    println!("page2_png: ok");

    // Single page selection — page 3 (SVG)
    let page3_svg = doc.select_pages([2]).to_svg()?;
    assert_eq!(page3_svg.len(), 1);
    assert!(page3_svg[0].contains("<svg"), "Invalid SVG content");
    println!("page3_svg: ok");

    // Odd pages (0, 2, 4) → pages 1, 3, 5 (PDF)
    let odd_pdf = doc.select_pages([0, 2, 4]).to_pdf()?;
    assert!(odd_pdf.starts_with(b"%PDF"), "Invalid PDF header");
    println!("odd_pdf: ok");

    // Odd pages (0, 2, 4) → pages 1, 3, 5 (PNG)
    let odd_pngs = doc.select_pages([0, 2, 4]).to_png(144.0)?;
    assert_eq!(odd_pngs.len(), 3, "Expected 3 PNGs for odd pages");
    for png in &odd_pngs {
        assert!(
            png.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
            "Invalid PNG header"
        );
    }
    println!("odd_pngs: {} pages", odd_pngs.len());

    // Dedup test: [4, 0, 0, 2] should produce same PDF as [0, 2, 4]
    let dedup_pdf = doc.select_pages([4, 0, 0, 2]).to_pdf()?;
    assert_eq!(
        odd_pdf, dedup_pdf,
        "Dedup PDF should be identical to odd PDF"
    );
    println!("dedup_pdf: identical to odd_pdf");

    // Range: pages 2-4 (indices 1..4) as PDF
    let range_pdf = doc.select_pages(1..4).to_pdf()?;
    assert!(range_pdf.starts_with(b"%PDF"), "Invalid PDF header");
    println!("range_pdf: ok");

    // Error cases
    assert!(
        doc.select_pages([]).to_pdf().is_err(),
        "Empty selection should error"
    );
    assert!(
        doc.select_pages([99]).to_pdf().is_err(),
        "Out of range should error"
    );
    println!("error cases: ok");

    println!("All assertions passed!");

    // Phase 2: Generate visual report using main.typ

    let report = typst_bake::document!("main.typ")
        .add_file("outputs/all.pdf", all_pdf)?
        .add_file("outputs/page1.pdf", page1_pdf)?
        .add_file("outputs/page2.png", page2_png[0].clone())?
        .add_file("outputs/page3.svg", page3_svg[0].as_bytes().to_vec())?
        .add_file("outputs/odd.pdf", odd_pdf)?
        .add_file("outputs/odd_1.png", odd_pngs[0].clone())?
        .add_file("outputs/odd_2.png", odd_pngs[1].clone())?
        .add_file("outputs/odd_3.png", odd_pngs[2].clone())?
        .add_file("outputs/dedup.pdf", dedup_pdf)?
        .add_file("outputs/range.pdf", range_pdf)?
        .to_pdf()?;

    std::fs::write(out_dir.join("output.pdf"), &report)?;
    println!("Generated output.pdf ({} bytes)", report.len());

    Ok(())
}

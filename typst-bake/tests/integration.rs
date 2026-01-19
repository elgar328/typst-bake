//! Integration tests for typst-bake document generation.
//!
//! These tests verify the full pipeline by compiling actual Typst documents.

use std::path::PathBuf;
use std::process::Command;

/// Get the workspace root directory
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points to typst-bake crate, parent is workspace root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Run example and verify PDF generation
fn run_example_and_verify_pdf(example_name: &str, pdf_rel_path: &str) {
    let workspace = workspace_root();
    let pdf_path = workspace.join(pdf_rel_path);

    // 1. Delete existing file (remove previous run results)
    let _ = std::fs::remove_file(&pdf_path);
    assert!(!pdf_path.exists(), "Failed to delete old PDF");

    // 2. Run example from workspace root
    let output = Command::new("cargo")
        .args(["run", "-p", example_name])
        .current_dir(&workspace)
        .output()
        .expect("Failed to run example");

    assert!(
        output.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 3. Verify newly generated PDF
    assert!(pdf_path.exists(), "PDF was not generated");
    let pdf = std::fs::read(&pdf_path).expect("Failed to read PDF");
    assert!(!pdf.is_empty(), "PDF is empty");
    assert!(pdf.starts_with(b"%PDF"), "Invalid PDF header");
}

#[test]
fn test_quick_start_generates_pdf() {
    run_example_and_verify_pdf("example-quick-start", "examples/quick-start/output.pdf");
}

#[test]
fn test_font_guide_generates_pdf() {
    run_example_and_verify_pdf("example-font-guide", "examples/font-guide/output.pdf");
}

#[test]
fn test_with_inputs_generates_pdf() {
    run_example_and_verify_pdf("example-with-inputs", "examples/with-inputs/output.pdf");
}

#[test]
fn test_with_package_generates_pdf() {
    run_example_and_verify_pdf("example-with-package", "examples/with-package/output.pdf");
}

#[test]
fn test_output_formats_generates_all() {
    let workspace = workspace_root();
    let example_dir = workspace.join("examples/output-formats");

    // Files to verify
    let pdf_path = example_dir.join("output.pdf");
    let svg_paths = [
        example_dir.join("output_1.svg"),
        example_dir.join("output_2.svg"),
    ];
    let png_paths = [
        example_dir.join("output_1.png"),
        example_dir.join("output_2.png"),
    ];

    // 1. Delete existing files
    let _ = std::fs::remove_file(&pdf_path);
    for path in &svg_paths {
        let _ = std::fs::remove_file(path);
    }
    for path in &png_paths {
        let _ = std::fs::remove_file(path);
    }

    // 2. Run example from workspace root
    let output = Command::new("cargo")
        .args(["run", "-p", "example-output-formats"])
        .current_dir(&workspace)
        .output()
        .expect("Failed to run example");

    assert!(
        output.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // 3. Verify PDF
    assert!(pdf_path.exists(), "PDF was not generated");
    let pdf = std::fs::read(&pdf_path).expect("Failed to read PDF");
    assert!(!pdf.is_empty(), "PDF is empty");
    assert!(pdf.starts_with(b"%PDF"), "Invalid PDF header");

    // 4. Verify SVGs
    for path in &svg_paths {
        assert!(path.exists(), "SVG was not generated: {:?}", path);
        let svg = std::fs::read_to_string(path).expect("Failed to read SVG");
        assert!(!svg.is_empty(), "SVG is empty");
        assert!(svg.contains("<svg"), "Invalid SVG content");
    }

    // 5. Verify PNGs
    for path in &png_paths {
        assert!(path.exists(), "PNG was not generated: {:?}", path);
        let png = std::fs::read(path).expect("Failed to read PNG");
        assert!(!png.is_empty(), "PNG is empty");
        assert!(
            png.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
            "Invalid PNG header"
        );
    }
}

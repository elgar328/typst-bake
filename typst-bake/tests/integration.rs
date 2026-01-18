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

use std::fs::{self, create_dir_all};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag for visualization generation
pub static VIZ_GENERATION_ENABLED: AtomicBool = AtomicBool::new(true);

/// Set whether visualization generation is enabled
pub fn set_viz_enabled(enabled: bool) {
    VIZ_GENERATION_ENABLED.store(enabled, Ordering::SeqCst);
}

/// Check whether visualization generation is enabled
pub fn viz_enabled() -> bool {
    VIZ_GENERATION_ENABLED.load(Ordering::SeqCst)
}

/// Save GraphViz DOT files to disk and generate visualizations
///
/// This function:
/// 1. Creates the output directory and subdirectory if they don't exist
/// 2. Saves the DOT file
/// 3. Runs the GraphViz 'dot' command to generate PNG, SVG, and PDF visualizations
/// 4. Optionally opens the generated PNG files for viewing
///
/// # Arguments
/// * `dot_content` - GraphViz DOT language content as a string
/// * `name` - Base name for the generated files
/// * `viz_type` - Type of visualization (e.g., "network", "petri")
/// * `open_files` - Whether to open the generated PNG files for viewing
///
/// Returns a Result with the paths to the generated files or an error message
pub fn save_graphviz(
    dot_content: &str,
    out_dir: &str,
    viz_type: &str,
    open_files: bool,
) -> Result<Vec<String>, String> {
    let out_path = Path::new(out_dir);

    // *** Create directory if it doesn't exist ***
    if !out_path.exists() {
        match create_dir_all(out_path) {
            Ok(_) => (),
            Err(e) => return Err(format!("Failed to create output directory: {}", e)),
        }
    }

    let mut generated_files = Vec::new();

    // Save full visualization
    let dot_path = out_path.join(format!("{}.dot", viz_type));
    let png_path = out_path.join(format!("{}.png", viz_type));
    let svg_path = out_path.join(format!("{}.svg", viz_type));
    let pdf_path = out_path.join(format!("{}.pdf", viz_type));

    match fs::write(&dot_path, dot_content) {
        Ok(_) => {
            generated_files.push(dot_path.to_string_lossy().to_string());

            // Generate PNG
            match Command::new("dot")
                .args(["-Tpng", "-o", &png_path.to_string_lossy()])
                .arg(&dot_path)
                .output()
            {
                Ok(output) => {
                    // Check if the command executed successfully (exit code 0)
                    if output.status.success() {
                        // Verify the file was created
                        if png_path.exists() {
                            generated_files.push(png_path.to_string_lossy().to_string());
                        } else {
                            println!("Warning: dot command executed but PNG file was not created");
                            if !output.stderr.is_empty() {
                                println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
                            }
                        }
                    } else {
                        // Command failed with non-zero exit code
                        println!(
                            "Warning: GraphViz dot command failed with exit code {:?}: {}",
                            output.status.code(),
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "Warning: Failed to generate visualization PNG: {}. \
                        Is GraphViz installed? Try installing with 'brew install graphviz' on macOS or \
                        'apt-get install graphviz' on Linux.",
                        e
                    );
                }
            }

            // Generate SVG (better for web viewing)
            match Command::new("dot")
                .args(["-Tsvg", "-o", &svg_path.to_string_lossy()])
                .arg(&dot_path)
                .output()
            {
                Ok(output) => {
                    if output.status.success() && svg_path.exists() {
                        generated_files.push(svg_path.to_string_lossy().to_string());
                    } else if !output.status.success() {
                        println!(
                            "Warning: Failed to generate SVG: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                }
                Err(e) => {
                    println!("Warning: Failed to execute dot for SVG: {}", e);
                }
            }

            // Generate PDF (better for printing)
            match Command::new("dot")
                .args(["-Tpdf", "-o", &pdf_path.to_string_lossy()])
                .arg(&dot_path)
                .output()
            {
                Ok(output) => {
                    if output.status.success() && pdf_path.exists() {
                        generated_files.push(pdf_path.to_string_lossy().to_string());
                    } else if !output.status.success() {
                        println!(
                            "Warning: Failed to generate PDF: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                }
                Err(e) => {
                    println!("Warning: Failed to execute dot for PDF: {}", e);
                }
            }
        }
        Err(e) => return Err(format!("Failed to write DOT file: {}", e)),
    }

    // Try to open the PNG files for viewing (platform-specific)
    if open_files {
        // Try to open the PNG if it exists
        if png_path.exists() {
            #[cfg(target_os = "macos")]
            match Command::new("open").arg(&png_path).spawn() {
                Ok(_) => {}
                Err(e) => println!("Warning: Could not open PNG: {}", e),
            }

            #[cfg(target_os = "linux")]
            match Command::new("xdg-open").arg(&png_path).spawn() {
                Ok(_) => {}
                Err(e) => println!("Warning: Could not open PNG: {}", e),
            }

            #[cfg(target_os = "windows")]
            match Command::new("cmd")
                .args(["/C", "start", &png_path.to_string_lossy()])
                .spawn()
            {
                Ok(_) => {}
                Err(e) => println!("Warning: Could not open PNG: {}", e),
            }
        } else {
            println!("Warning: PNG file does not exist: {}", png_path.display());
        }
    }

    Ok(generated_files)
}

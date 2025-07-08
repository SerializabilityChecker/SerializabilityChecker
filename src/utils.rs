//! Utility functions shared across the codebase
//!
//! This module provides common string processing, file handling, and formatting
//! utilities used throughout the serializability checker.

/// String processing utilities
pub mod string {
    /// Escape a string for use as a GraphViz node ID
    ///
    /// GraphViz node IDs must be alphanumeric or underscores. This function
    /// replaces all other characters with underscores and ensures the result
    /// is a valid identifier.
    pub fn escape_for_graphviz_id(s: &str) -> String {
           s.chars()
            .map(|c| match c {
                c if c.is_alphanumeric() || c == '_' => c,
                '+' => 'P',   // make plus distinct
                '-' => 'M',   // make minus distinct
                _   => '_',
            })
            .collect()
    }

    /// Sanitize a string for use in filenames and external tools
    ///
    /// Replaces non-alphanumeric characters with underscores to create
    /// safe filenames and identifiers for external tools like SMPT.
    pub fn sanitize(s: &str) -> String {
        s.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect()
    }

    /// Escape HTML special characters
    ///
    /// Converts HTML special characters to their entity equivalents
    /// for safe inclusion in HTML debug reports.
    pub fn html_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
}

/// File and directory utilities
pub mod file {
    use std::fs;
    use std::path::Path;

    /// Ensure a directory exists, creating it if necessary
    ///
    /// Creates the directory and any necessary parent directories.
    /// Returns an error if the directory cannot be created.
    pub fn ensure_dir_exists(path: &str) -> Result<(), std::io::Error> {
        fs::create_dir_all(path)
    }

    /// Safely write content to a file
    ///
    /// Creates the parent directory if it doesn't exist and writes
    /// the content to the specified file path.
    pub fn safe_write_file(file_path: &str, content: &str) -> Result<(), std::io::Error> {
        if let Some(parent) = Path::new(file_path).parent() {
            ensure_dir_exists(&parent.to_string_lossy())?;
        }
        fs::write(file_path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_for_graphviz_id() {
        assert_eq!(string::escape_for_graphviz_id("hello-world"), "hello_world");
        assert_eq!(string::escape_for_graphviz_id("test.file"), "test_file");
        assert_eq!(
            string::escape_for_graphviz_id("already_valid"),
            "already_valid"
        );
        assert_eq!(string::escape_for_graphviz_id("123abc"), "123abc");
        assert_eq!(
            string::escape_for_graphviz_id("special!@#$%"),
            "special_____"
        );
    }

    #[test]
    fn test_sanitize() {
        assert_eq!(string::sanitize("hello-world"), "hello_world");
        assert_eq!(string::sanitize("test@example.com"), "test_example_com");
        assert_eq!(string::sanitize("already_valid"), "already_valid");
        assert_eq!(string::sanitize("123"), "123");
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(string::html_escape("<div>"), "&lt;div&gt;");
        assert_eq!(string::html_escape("Tom & Jerry"), "Tom &amp; Jerry");
        assert_eq!(string::html_escape("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(
            string::html_escape("'apostrophe'"),
            "&#x27;apostrophe&#x27;"
        );
    }

    #[test]
    fn test_ensure_dir_exists() {
        // Test with a temp directory
        let temp_dir = std::env::temp_dir().join("ser_test_dir");
        let temp_path = temp_dir.to_string_lossy();

        // Clean up first if it exists
        let _ = std::fs::remove_dir_all(&*temp_path);

        // Should create successfully
        assert!(file::ensure_dir_exists(&temp_path).is_ok());

        // Should be idempotent
        assert!(file::ensure_dir_exists(&temp_path).is_ok());

        // Clean up
        let _ = std::fs::remove_dir_all(&*temp_path);
    }
}

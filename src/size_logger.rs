use crate::kleene::SMART_ORDER;
use crate::reachability::BIDIRECTIONAL_PRUNING_ENABLED;
use crate::semilinear::{GENERATE_LESS, REMOVE_REDUNDANT};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::{fs, fs::OpenOptions, io::Write, path::Path};

// Global hash table for storing string-to-string mappings
lazy_static::lazy_static! {
    static ref GLOBAL_LOGGER: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

#[derive(Serialize)]
pub struct PetriNetSize {
    // A string representing the program name (benchmark)
    pub program_name: String,
    /// Identifier so you know which disjunct / iteration this came from
    pub disjunct_id: usize,
    /// "pre_pruning" or "post_pruning"
    pub stage: &'static str,
    /// Number of places in the net at this point
    pub num_places: usize,
    /// Number of transitions in the net at this point
    pub num_transitions: usize,
}

/// Append a record to a CSV file (no headers, so you can keep appending)
pub fn log_petri_size_csv(path: &Path, entry: &PetriNetSize) -> Result<(), std::io::Error> {
    // Decide if we need to write headers: either file doesn't exist yet,
    // or it exists but is zero‐length.
    let need_header = match path.metadata() {
        Ok(meta) => meta.len() == 0,
        Err(_) => true,
    };

    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false) // we'll manage headers ourselves
        .from_writer(file);

    if need_header {
        // write a top row naming each column
        wtr.write_record([
            "bidirectional_pruning ON",
            "remove_redundant ON",
            "generate_less ON",
            "smart_order ON",
            "benchmark",
            "index",
            "stage",
            "num_places",
            "num_transitions"])?;
    }

    let mut record = Vec::new();

    // read each flag and push "1"/"0"
    let bidir_pruning = BIDIRECTIONAL_PRUNING_ENABLED.load(Ordering::Relaxed);
    let remove_redundant = REMOVE_REDUNDANT.load(Ordering::Relaxed);
    let generate_less = GENERATE_LESS.load(Ordering::Relaxed);
    let smart_order = SMART_ORDER.load(Ordering::Relaxed);
    record.push(if bidir_pruning { "1" } else { "0" }.to_string());
    record.push(if remove_redundant { "1" } else { "0" }.to_string());
    record.push(if generate_less { "1" } else { "0" }.to_string());
    record.push(if smart_order { "1" } else { "0" }.to_string());

    record.push(entry.program_name.to_string());
    record.push(entry.disjunct_id.to_string());
    record.push(entry.stage.to_string());
    record.push(entry.num_places.to_string());
    record.push(entry.num_transitions.to_string());

    // write all 9 columns in one shot
    wtr.write_record(&record)?;
    wtr.flush()?;

    // ─── ALSO log into global optimization_experiments/petri_size/petri_size_stats.csv ─────
    let experiments_dir = Path::new("optimization_experiments/petri_size");
    fs::create_dir_all(&experiments_dir)?;
    let global_path = experiments_dir.join("petri_size_stats.csv");
    let need_header = match global_path.metadata() {
        Ok(meta) => meta.len() == 0,
        Err(_) => true,
    };
    let file = OpenOptions::new().create(true).append(true).open(&global_path)?;
    let mut exp_wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(file);
    if need_header {
        exp_wtr.write_record(&[
            "bidirectional_pruning ON",
            "remove_redundant ON",
            "generate_less ON",
            "smart_order ON",
            "benchmark",
            "index",
            "stage",
            "num_places",
            "num_transitions",
        ])?;
    }
    exp_wtr.write_record(&record)?;
    exp_wtr.flush()?;

    Ok(())
}

/// Append a record to a JSON‐lines file (one JSON object per line)
pub fn log_size_json(path: &Path, entry: &PetriNetSize) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let json = serde_json::to_string(entry)?;
    writeln!(file, "{}", json)?;
    Ok(())
}

/// Log entry for SemilinearSet statistics
#[derive(Serialize)]
pub struct SemilinearStats {
    // Name of the benchmark
    pub program_name: String,
    /// Number of linear-set components
    pub num_components: usize,
    /// Number of period vectors per component
    pub periods_per_component: Vec<usize>,
}

/// Append a record to a CSV file for SemilinearSet statistics
pub fn log_semilinear_size_csv(path: &Path, entry: &SemilinearStats) -> Result<(), std::io::Error> {
    // Ensure output directory exists
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    let file = OpenOptions::new().create(true).append(true).open(path)?;

    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(file);

    let need_header = match path.metadata() {
        Ok(meta) => meta.len() == 0,
        Err(_) => true,
    };
    if need_header {
        wtr.write_record([
            "bidirectional_pruning ON",
            "remove_redundant ON",
            "generate_less ON",
            "smart_order ON",
            "benchmark",
            "num_components",
            "max_periods_per_component",
            "avg_periods_per_component",
        ])?
    }

    let mut record = Vec::new();

    // read each flag and push "1"/"0"
    let bidir_pruning = BIDIRECTIONAL_PRUNING_ENABLED.load(Ordering::Relaxed);
    let remove_redundant = REMOVE_REDUNDANT.load(Ordering::Relaxed);
    let generate_less = GENERATE_LESS.load(Ordering::Relaxed);
    let smart_order = SMART_ORDER.load(Ordering::Relaxed);
    record.push(if bidir_pruning { "1" } else { "0" }.to_string());
    record.push(if remove_redundant { "1" } else { "0" }.to_string());
    record.push(if generate_less { "1" } else { "0" }.to_string());
    record.push(if smart_order { "1" } else { "0" }.to_string());

    record.push(entry.program_name.to_string());
    record.push(entry.num_components.to_string());

    let max_period = entry
        .periods_per_component
        .iter()
        .copied()
        .max()
        .unwrap_or(0);
    record.push(max_period.to_string());

    let avg_period = if entry.periods_per_component.is_empty() {
        0.0
    } else {
        let sum: usize = entry.periods_per_component.iter().sum();
        sum as f64 / (entry.periods_per_component.len() as f64)
    };
    record.push(format!("{:.2}", avg_period)); // 2-decimal precision

    wtr.write_record(record)?;
    wtr.flush()?;
    Ok(())
}

/// Store a key-value pair in the global logger
/// 
/// This function stores a string key-value pair in a global hash table that persists
/// throughout the program execution. The data can be retrieved later and written to CSV files.
/// 
/// # Arguments
/// * `key` - The key to store the value under
/// * `value` - The value to store
/// 
/// # Example
/// ```
/// use crate::size_logger::log_global_value;
/// 
/// log_global_value("program_name".to_string(), "my_program".to_string());
/// log_global_value("runtime_ms".to_string(), "1234".to_string());
/// ```
pub fn log_global_value(key: String, value: String) {
    if let Ok(mut logger) = GLOBAL_LOGGER.lock() {
        logger.insert(key, value);
    }
}

/// Get a value from the global logger
/// 
/// Retrieves a value stored under the given key in the global logger.
/// 
/// # Arguments
/// * `key` - The key to look up
/// 
/// # Returns
/// * `Some(value)` if the key exists, `None` otherwise
/// 
/// # Example
/// ```
/// use crate::size_logger::{log_global_value, get_global_value};
/// 
/// log_global_value("status".to_string(), "success".to_string());
/// assert_eq!(get_global_value("status"), Some("success".to_string()));
/// assert_eq!(get_global_value("nonexistent"), None);
/// ```
pub fn get_global_value(key: &str) -> Option<String> {
    if let Ok(logger) = GLOBAL_LOGGER.lock() {
        logger.get(key).cloned()
    } else {
        None
    }
}

/// Clear all entries from the global logger
/// 
/// Removes all key-value pairs from the global logger, resetting it to an empty state.
/// 
/// # Example
/// ```
/// use crate::size_logger::{log_global_value, clear_global_logger, get_global_value};
/// 
/// log_global_value("key1".to_string(), "value1".to_string());
/// assert_eq!(get_global_value("key1"), Some("value1".to_string()));
/// 
/// clear_global_logger();
/// assert_eq!(get_global_value("key1"), None);
/// ```
pub fn clear_global_logger() {
    if let Ok(mut logger) = GLOBAL_LOGGER.lock() {
        logger.clear();
    }
}

/// Write the current global logger contents as a row to a CSV file
/// 
/// This function writes all key-value pairs from the global logger as a single row
/// to the specified CSV file. If the file doesn't exist, it will be created with
/// headers based on the keys in the logger (sorted alphabetically).
/// 
/// # Arguments
/// * `path` - The path to the CSV file to write to
/// 
/// # Returns
/// * `Ok(())` on success, `Err` on failure
/// 
/// # Example
/// ```
/// use crate::size_logger::{log_global_value, write_global_logger_to_csv};
/// use std::path::Path;
/// 
/// log_global_value("program".to_string(), "test_program".to_string());
/// log_global_value("runtime".to_string(), "123.45".to_string());
/// 
/// let result = write_global_logger_to_csv(Path::new("output.csv"));
/// assert!(result.is_ok());
/// ```
pub fn write_global_logger_to_csv(path: &Path) -> Result<(), std::io::Error> {
    // Ensure output directory exists
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    // Get current logger contents
    let logger_data = if let Ok(logger) = GLOBAL_LOGGER.lock() {
        logger.clone()
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to acquire global logger lock",
        ));
    };

    // Check if file exists and is empty
    let need_header = match path.metadata() {
        Ok(meta) => meta.len() == 0,
        Err(_) => true,
    };

    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(file);

    if need_header {
        // Write headers based on the keys in the logger
        let mut headers: Vec<String> = logger_data.keys().cloned().collect();
        headers.sort(); // Sort for consistent ordering
        wtr.write_record(&headers)?;
    }

    // Write the values in the same order as headers
    let mut headers: Vec<String> = logger_data.keys().cloned().collect();
    headers.sort(); // Sort for consistent ordering
    let values: Vec<String> = headers
        .iter()
        .map(|key| logger_data.get(key).unwrap_or(&String::new()).clone())
        .collect();

    wtr.write_record(&values)?;
    wtr.flush()?;

    Ok(())
}

/// Write the current global logger contents as a row to a CSV file with custom headers
/// If the file doesn't exist, it will be created with the provided headers
/// 
/// This function writes key-value pairs from the global logger as a single row
/// to the specified CSV file, using the provided headers to determine the column order.
/// If a key from the headers doesn't exist in the logger, an empty string will be written.
/// 
/// # Arguments
/// * `path` - The path to the CSV file to write to
/// * `headers` - The column headers to use, which also determine the order of values
/// 
/// # Returns
/// * `Ok(())` on success, `Err` on failure
/// 
/// # Example
/// ```
/// use crate::size_logger::{log_global_value, write_global_logger_to_csv_with_headers};
/// use std::path::Path;
/// 
/// log_global_value("program".to_string(), "test_program".to_string());
/// log_global_value("runtime".to_string(), "123.45".to_string());
/// log_global_value("status".to_string(), "success".to_string());
/// 
/// let headers = vec!["program".to_string(), "status".to_string(), "runtime".to_string()];
/// let result = write_global_logger_to_csv_with_headers(Path::new("output.csv"), &headers);
/// assert!(result.is_ok());
/// ```
pub fn write_global_logger_to_csv_with_headers(
    path: &Path,
    headers: &[String],
) -> Result<(), std::io::Error> {
    // Ensure output directory exists
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    // Get current logger contents
    let logger_data = if let Ok(logger) = GLOBAL_LOGGER.lock() {
        logger.clone()
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to acquire global logger lock",
        ));
    };

    // Check if file exists and is empty
    let need_header = match path.metadata() {
        Ok(meta) => meta.len() == 0,
        Err(_) => true,
    };

    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(file);

    if need_header {
        wtr.write_record(headers)?;
    }

    // Write the values in the order of the provided headers
    let values: Vec<String> = headers
        .iter()
        .map(|key| logger_data.get(key).unwrap_or(&String::new()).clone())
        .collect();

    wtr.write_record(&values)?;
    wtr.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_logger() {
        // Clear any existing data
        clear_global_logger();

        // Store some values
        log_global_value("program".to_string(), "test_program".to_string());
        log_global_value("runtime".to_string(), "123.45".to_string());
        log_global_value("status".to_string(), "success".to_string());

        // Retrieve a value
        assert_eq!(get_global_value("program"), Some("test_program".to_string()));
        assert_eq!(get_global_value("runtime"), Some("123.45".to_string()));
        assert_eq!(get_global_value("nonexistent"), None);

        // Test CSV writing
        let test_path = Path::new("test_global_logger.csv");
        let result = write_global_logger_to_csv(test_path);
        assert!(result.is_ok());

        // Test CSV writing with custom headers
        let test_path2 = Path::new("test_global_logger_custom.csv");
        let headers = vec!["program".to_string(), "status".to_string(), "runtime".to_string()];
        let result2 = write_global_logger_to_csv_with_headers(test_path2, &headers);
        assert!(result2.is_ok());

        // Clean up test files
        let _ = fs::remove_file(test_path);
        let _ = fs::remove_file(test_path2);
    }
}

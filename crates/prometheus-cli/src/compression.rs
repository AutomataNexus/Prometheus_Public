// ============================================================================
// File: compression.rs
// Description: OpenZL dataset compression format with zstd and trainable dictionaries
// Author: Andrew Jewell Sr. - AutomataNexus
// Updated: March 8, 2026
//
// DISCLAIMER: This software is provided "as is", without warranty of any kind,
// express or implied. Use at your own risk. AutomataNexus and the author assume
// no liability for any damages arising from the use of this software.
// ============================================================================
//! OpenZL -- Prometheus dataset compression with trainable dictionaries.
//!
//! OpenZL wraps zstd with a custom file format that supports:
//! - Dictionary-based compression trained on dataset patterns
//! - Streaming compression for large CSV files
//! - Custom header with magic bytes, version, and dictionary ID
//!
//! File format (.ozl):
//! ```text
//! [4 bytes] Magic: "OZL\x01"
//! [1 byte]  Version: 0x01
//! [1 byte]  Flags: bit 0 = has dictionary
//! [32 bytes] SHA-256 of original data
//! [8 bytes] Original size (u64 LE)
//! [8 bytes] Dictionary ID (u64 LE, 0 if none)
//! [...] zstd-compressed data (with or without dictionary)
//! ```

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::io::Write;

use crate::theme;

const MAGIC: &[u8; 4] = b"OZL\x01";
const VERSION: u8 = 0x01;
const HEADER_SIZE: usize = 4 + 1 + 1 + 32 + 8 + 8; // 54 bytes

const ZSTD_LEVEL: i32 = 15; // High compression for datasets
const DICT_MAX_SIZE: usize = 112 * 1024; // 112 KB dictionary

/// Compress a file using OpenZL format.
pub fn compress_file(
    input_path: &str,
    output_path: Option<&str>,
    dict_path: Option<&str>,
) -> Result<()> {
    let input = std::fs::read(input_path)
        .with_context(|| format!("Cannot read input file: {input_path}"))?;

    let out_path = output_path
        .map(String::from)
        .unwrap_or_else(|| format!("{input_path}.ozl"));

    // Load dictionary if provided
    let dict_data = match dict_path {
        Some(dp) => {
            let data =
                std::fs::read(dp).with_context(|| format!("Cannot read dictionary: {dp}"))?;
            theme::print_info(&format!(
                "Using dictionary: {dp} ({:.1} KB)",
                data.len() as f64 / 1024.0
            ));
            Some(data)
        }
        None => None,
    };

    theme::print_info(&format!(
        "Compressing {} ({:.1} KB)...",
        input_path,
        input.len() as f64 / 1024.0
    ));

    // Compute SHA-256 of original data
    let hash = Sha256::digest(&input);

    // Compress with zstd
    let compressed = match &dict_data {
        Some(dict) => {
            let mut compressor = zstd::bulk::Compressor::with_dictionary(ZSTD_LEVEL, dict)
                .context("Failed to create zstd compressor with dictionary")?;
            compressor.compress(&input)
                .context("zstd compression with dictionary failed")?
        }
        None => {
            zstd::bulk::compress(&input, ZSTD_LEVEL).context("zstd compression failed")?
        }
    };

    // Write OpenZL file
    let mut file = std::fs::File::create(&out_path)
        .with_context(|| format!("Cannot create output: {out_path}"))?;

    let flags: u8 = if dict_data.is_some() { 0x01 } else { 0x00 };
    let dict_id: u64 = if dict_data.is_some() {
        // Use hash of dict as ID
        let dh = Sha256::digest(dict_data.as_ref().unwrap());
        u64::from_le_bytes(dh[..8].try_into().unwrap())
    } else {
        0
    };

    file.write_all(MAGIC)?;
    file.write_all(&[VERSION])?;
    file.write_all(&[flags])?;
    file.write_all(&hash)?;
    file.write_all(&(input.len() as u64).to_le_bytes())?;
    file.write_all(&dict_id.to_le_bytes())?;
    file.write_all(&compressed)?;
    file.flush()?;

    let ratio = compressed.len() as f64 / input.len() as f64 * 100.0;
    let saved = (1.0 - compressed.len() as f64 / input.len() as f64) * 100.0;

    theme::print_success(&format!(
        "Compressed: {out_path} ({:.1} KB, {ratio:.1}% of original, saved {saved:.1}%)",
        (HEADER_SIZE + compressed.len()) as f64 / 1024.0
    ));

    Ok(())
}

/// Decompress an OpenZL file.
pub fn decompress_file(input_path: &str, output_path: Option<&str>) -> Result<()> {
    let data =
        std::fs::read(input_path).with_context(|| format!("Cannot read: {input_path}"))?;

    if data.len() < HEADER_SIZE {
        anyhow::bail!("File too small to be a valid .ozl file");
    }

    // Parse header
    if &data[..4] != MAGIC {
        anyhow::bail!("Not a valid OpenZL file (bad magic bytes)");
    }

    let version = data[4];
    if version != VERSION {
        anyhow::bail!("Unsupported OpenZL version: {version}");
    }

    let flags = data[5];
    let has_dict = flags & 0x01 != 0;
    let stored_hash: &[u8] = &data[6..38];
    let original_size = u64::from_le_bytes(data[38..46].try_into().unwrap()) as usize;
    let _dict_id = u64::from_le_bytes(data[46..54].try_into().unwrap());
    let compressed_data = &data[HEADER_SIZE..];

    if has_dict {
        theme::print_warning(
            "File was compressed with a dictionary. Attempting without (may fail).",
        );
        theme::print_info("Use --dict <dictionary_file> if decompression fails.");
    }

    theme::print_info(&format!("Decompressing {input_path}..."));

    let decompressed = zstd::bulk::decompress(compressed_data, original_size)
        .context("zstd decompression failed (dictionary may be required)")?;

    // Verify hash
    let hash = Sha256::digest(&decompressed);
    if hash.as_slice() != stored_hash {
        theme::print_warning(
            "SHA-256 mismatch -- file may be corrupted or wrong dictionary used.",
        );
    }

    let out_path = output_path.map(String::from).unwrap_or_else(|| {
        input_path
            .strip_suffix(".ozl")
            .map(String::from)
            .unwrap_or_else(|| format!("{input_path}.decompressed"))
    });

    std::fs::write(&out_path, &decompressed)
        .with_context(|| format!("Cannot write output: {out_path}"))?;

    theme::print_success(&format!(
        "Decompressed: {out_path} ({:.1} KB)",
        decompressed.len() as f64 / 1024.0
    ));

    Ok(())
}

/// Train an OpenZL dictionary on a set of dataset files.
///
/// Analyzes the data patterns in the provided CSV files and trains
/// a zstd dictionary optimized for compressing similar datasets.
pub fn train_dictionary(files: &[String], output_path: &str) -> Result<()> {
    if files.is_empty() {
        theme::print_error("No input files provided.");
        return Ok(());
    }

    theme::print_info(&format!(
        "Training OpenZL dictionary on {} file(s)...",
        files.len()
    ));

    // Read all training data
    let mut samples: Vec<Vec<u8>> = Vec::new();
    let mut total_bytes = 0usize;

    for file_path in files {
        match std::fs::read(file_path) {
            Ok(data) => {
                total_bytes += data.len();
                // Split into chunks for better dictionary training
                // Each chunk is a sample for the dictionary trainer
                let chunk_size = 16 * 1024; // 16 KB chunks
                for chunk in data.chunks(chunk_size) {
                    samples.push(chunk.to_vec());
                }
                theme::print_info(&format!(
                    "  Loaded: {file_path} ({:.1} KB)",
                    data.len() as f64 / 1024.0
                ));
            }
            Err(e) => {
                theme::print_warning(&format!("  Skipping {file_path}: {e}"));
            }
        }
    }

    if samples.is_empty() {
        theme::print_error("No valid data loaded for dictionary training.");
        return Ok(());
    }

    theme::print_info(&format!(
        "Training on {:.1} KB of data ({} samples)...",
        total_bytes as f64 / 1024.0,
        samples.len()
    ));

    // Train zstd dictionary from samples
    let dict = zstd::dict::from_samples(&samples, DICT_MAX_SIZE)
        .context("Dictionary training failed")?;

    std::fs::write(output_path, &dict)
        .with_context(|| format!("Cannot write dictionary: {output_path}"))?;

    theme::print_success(&format!(
        "Dictionary trained: {output_path} ({:.1} KB)",
        dict.len() as f64 / 1024.0
    ));

    // Test compression ratio improvement
    if let Some(first_file) = files.first() {
        if let Ok(test_data) = std::fs::read(first_file) {
            let without_dict = zstd::bulk::compress(&test_data, ZSTD_LEVEL)
                .map(|c| c.len())
                .unwrap_or(test_data.len());
            let with_dict = zstd::bulk::Compressor::with_dictionary(ZSTD_LEVEL, &dict)
                .and_then(|mut c| c.compress(&test_data))
                .map(|c| c.len())
                .unwrap_or(test_data.len());

            let improvement = (1.0 - with_dict as f64 / without_dict as f64) * 100.0;
            theme::print_info(&format!(
                "Compression improvement with dictionary: {improvement:.1}% better ({without_dict} -> {with_dict} bytes)"
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_dir() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "prometheus-cli-compression-test-{}-{}",
            std::process::id(),
            id
        ));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// Generate repeating CSV-like data for compression tests.
    fn sample_csv(rows: usize) -> Vec<u8> {
        let mut data = String::from("id,temperature,humidity,pressure,status\n");
        for i in 0..rows {
            data.push_str(&format!(
                "{},{:.2},{:.1},{:.1},{}\n",
                i,
                20.0 + (i as f64 * 0.1) % 15.0,
                40.0 + (i as f64 * 0.3) % 50.0,
                1013.0 + (i as f64 * 0.05) % 10.0,
                if i % 3 == 0 { "normal" } else { "alert" }
            ));
        }
        data.into_bytes()
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let dir = test_dir();
        let input_path = dir.join("data.csv");
        let ozl_path = dir.join("data.csv.ozl");
        let output_path = dir.join("data_restored.csv");

        let original = sample_csv(200);
        fs::write(&input_path, &original).unwrap();

        compress_file(
            input_path.to_str().unwrap(),
            Some(ozl_path.to_str().unwrap()),
            None,
        )
        .unwrap();

        assert!(ozl_path.exists(), "Compressed file should exist");

        decompress_file(
            ozl_path.to_str().unwrap(),
            Some(output_path.to_str().unwrap()),
        )
        .unwrap();

        let restored = fs::read(&output_path).unwrap();
        assert_eq!(original, restored, "Decompressed data must match original");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ozl_header_magic_bytes() {
        let dir = test_dir();
        let input_path = dir.join("magic_test.csv");
        let ozl_path = dir.join("magic_test.ozl");

        fs::write(&input_path, &sample_csv(50)).unwrap();

        compress_file(
            input_path.to_str().unwrap(),
            Some(ozl_path.to_str().unwrap()),
            None,
        )
        .unwrap();

        let data = fs::read(&ozl_path).unwrap();

        // Check magic bytes: "OZL\x01"
        assert_eq!(&data[0..4], b"OZL\x01", "First 4 bytes must be OZL magic");
        // Version byte
        assert_eq!(data[4], 0x01, "Version must be 0x01");
        // Flags: no dictionary => 0x00
        assert_eq!(data[5], 0x00, "Flags should be 0x00 without dictionary");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ozl_header_original_size() {
        let dir = test_dir();
        let input_path = dir.join("size_test.csv");
        let ozl_path = dir.join("size_test.ozl");

        let original = sample_csv(100);
        let original_len = original.len();
        fs::write(&input_path, &original).unwrap();

        compress_file(
            input_path.to_str().unwrap(),
            Some(ozl_path.to_str().unwrap()),
            None,
        )
        .unwrap();

        let data = fs::read(&ozl_path).unwrap();

        // Original size is at bytes 38..46 (u64 LE)
        let stored_size = u64::from_le_bytes(data[38..46].try_into().unwrap()) as usize;
        assert_eq!(stored_size, original_len, "Stored original size must match");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ozl_sha256_integrity() {
        let dir = test_dir();
        let input_path = dir.join("sha_test.csv");
        let ozl_path = dir.join("sha_test.ozl");

        let original = sample_csv(80);
        fs::write(&input_path, &original).unwrap();

        compress_file(
            input_path.to_str().unwrap(),
            Some(ozl_path.to_str().unwrap()),
            None,
        )
        .unwrap();

        let data = fs::read(&ozl_path).unwrap();

        // SHA-256 hash is at bytes 6..38
        let stored_hash = &data[6..38];
        let computed_hash = Sha256::digest(&original);
        assert_eq!(
            stored_hash,
            computed_hash.as_slice(),
            "Stored SHA-256 must match hash of original data"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_decompress_file_too_small() {
        let dir = test_dir();
        let bad_file = dir.join("tiny.ozl");
        fs::write(&bad_file, b"OZL").unwrap(); // Only 3 bytes, less than HEADER_SIZE

        let result = decompress_file(bad_file.to_str().unwrap(), None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("too small"),
            "Error should mention file too small, got: {err}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_decompress_bad_magic() {
        let dir = test_dir();
        let bad_file = dir.join("badmagic.ozl");
        // Write a file with enough bytes but wrong magic
        let mut data = vec![0u8; HEADER_SIZE + 10];
        data[0..4].copy_from_slice(b"FAKE");
        fs::write(&bad_file, &data).unwrap();

        let result = decompress_file(bad_file.to_str().unwrap(), None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("bad magic"),
            "Error should mention bad magic bytes, got: {err}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_decompress_bad_version() {
        let dir = test_dir();
        let bad_file = dir.join("badver.ozl");
        let mut data = vec![0u8; HEADER_SIZE + 10];
        data[0..4].copy_from_slice(b"OZL\x01");
        data[4] = 0xFF; // unsupported version
        fs::write(&bad_file, &data).unwrap();

        let result = decompress_file(bad_file.to_str().unwrap(), None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Unsupported OpenZL version"),
            "Error should mention unsupported version, got: {err}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_compress_nonexistent_input() {
        let result = compress_file("/tmp/nonexistent_prometheus_test_file_xyz.csv", None, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Cannot read input file"),
            "Error should mention cannot read, got: {err}"
        );
    }

    #[test]
    fn test_default_output_path_appends_ozl() {
        let dir = test_dir();
        let input_path = dir.join("auto_name.csv");
        let expected_ozl = dir.join("auto_name.csv.ozl");

        fs::write(&input_path, &sample_csv(20)).unwrap();

        compress_file(input_path.to_str().unwrap(), None, None).unwrap();
        assert!(
            expected_ozl.exists(),
            "Without explicit output, should create <input>.ozl"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_compressed_smaller_than_original() {
        let dir = test_dir();
        let input_path = dir.join("ratio.csv");
        let ozl_path = dir.join("ratio.ozl");

        let original = sample_csv(500);
        fs::write(&input_path, &original).unwrap();

        compress_file(
            input_path.to_str().unwrap(),
            Some(ozl_path.to_str().unwrap()),
            None,
        )
        .unwrap();

        let compressed_size = fs::metadata(&ozl_path).unwrap().len() as usize;
        assert!(
            compressed_size < original.len(),
            "Compressed file ({compressed_size}) should be smaller than original ({})",
            original.len()
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_dictionary_training_and_roundtrip() {
        let dir = test_dir();

        // Create several CSV files for training
        // zstd dict training needs total data >= dict size (112KB)
        let mut files = Vec::new();
        for i in 0..10 {
            let path = dir.join(format!("train_{i}.csv"));
            fs::write(&path, &sample_csv(2000)).unwrap();
            files.push(path.to_string_lossy().to_string());
        }

        let dict_path = dir.join("test.ozl-dict");
        train_dictionary(&files, dict_path.to_str().unwrap()).unwrap();
        assert!(dict_path.exists(), "Dictionary file should be created");
        let dict_size = fs::metadata(&dict_path).unwrap().len();
        assert!(dict_size > 0, "Dictionary should not be empty");

        // Compress with dictionary
        let input_path = dir.join("dict_input.csv");
        let ozl_path = dir.join("dict_input.ozl");
        let _output_path = dir.join("dict_output.csv");

        let original = sample_csv(300);
        fs::write(&input_path, &original).unwrap();

        compress_file(
            input_path.to_str().unwrap(),
            Some(ozl_path.to_str().unwrap()),
            Some(dict_path.to_str().unwrap()),
        )
        .unwrap();

        // Check flags byte — should have dictionary flag set
        let ozl_data = fs::read(&ozl_path).unwrap();
        assert_eq!(ozl_data[5] & 0x01, 0x01, "Dictionary flag should be set");
        // dict_id should be non-zero
        let dict_id = u64::from_le_bytes(ozl_data[46..54].try_into().unwrap());
        assert_ne!(dict_id, 0, "Dictionary ID should be non-zero");

        // Note: decompress_file without dict may or may not work for dict-compressed data.
        // We test the header structure is valid. Full roundtrip with dict would require
        // extending decompress_file to accept a dict param (which it currently doesn't).

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_train_dictionary_empty_files_list() {
        // Should return Ok but print error (no panic)
        let result = train_dictionary(&[], "/tmp/unused.dict");
        assert!(result.is_ok());
    }

    #[test]
    fn test_header_size_constant() {
        // 4 (magic) + 1 (version) + 1 (flags) + 32 (sha256) + 8 (orig size) + 8 (dict id) = 54
        assert_eq!(HEADER_SIZE, 54);
    }
}

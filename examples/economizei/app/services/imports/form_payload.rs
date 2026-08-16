//! Builds upload payloads from HTML/JSON import forms.

use super::ImportFileInput;

pub fn disambiguate_filenames(files: Vec<(String, Vec<u8>)>) -> Vec<(String, Vec<u8>)> {
    let mut seen: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    files
        .into_iter()
        .map(|(name, bytes)| {
            let count = seen.entry(name.clone()).or_insert(0);
            *count += 1;
            if *count == 1 {
                (name, bytes)
            } else {
                (disambiguate_name(&name, *count), bytes)
            }
        })
        .collect()
}

fn disambiguate_name(name: &str, occurrence: u32) -> String {
    let (base, ext) = split_filename(name);
    format!("{base} ({occurrence}){ext}")
}

fn split_filename(name: &str) -> (String, String) {
    match name.rsplit_once('.') {
        Some((base, ext)) if !base.is_empty() && !ext.is_empty() => {
            (base.to_string(), format!(".{ext}"))
        }
        _ => (name.to_string(), String::new()),
    }
}

pub fn payloads_from_inputs(
    files: &[ImportFileInput],
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut payloads = Vec::new();
    for (index, file) in files.iter().enumerate() {
        if let Some(payload) = decode_file_input(file, index)? {
            payloads.push(payload);
        }
    }
    Ok(disambiguate_filenames(payloads))
}

pub fn payloads_from_parallel_arrays(
    original_filenames: &[String],
    csv_contents: &[String],
) -> Result<Vec<(String, Vec<u8>)>, String> {
    if original_filenames.len() != csv_contents.len() {
        return Err(crate::services::i18n::t(
            "imports.errors.file_field_mismatch",
        ));
    }

    let mut payloads = Vec::new();
    for (index, content) in csv_contents.iter().enumerate() {
        if content.trim().is_empty() {
            continue;
        }
        let original_filename = original_filenames
            .get(index)
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("import-{}.csv", index + 1));
        payloads.push((original_filename, content.as_bytes().to_vec()));
    }

    Ok(disambiguate_filenames(payloads))
}

fn decode_file_input(
    file: &ImportFileInput,
    index: usize,
) -> Result<Option<(String, Vec<u8>)>, String> {
    let original_filename = file
        .original_filename
        .as_ref()
        .filter(|name| !name.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| format!("import-{}.csv", index + 1));

    if let Some(base64_content) = file
        .content_base64
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        let file_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            base64_content.trim(),
        )
        .map_err(|e| {
            crate::services::i18n::t("imports.errors.invalid_base64")
                .replace("{filename}", &original_filename)
                .replace("{message}", &e.to_string())
        })?;
        return Ok(Some((original_filename, file_bytes)));
    }

    if let Some(content) = file
        .csv_content
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(Some((original_filename, content.as_bytes().to_vec())));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disambiguate_filenames_adds_suffix_for_duplicates() {
        let files = vec![
            ("report.csv".into(), b"a".to_vec()),
            ("report.csv".into(), b"b".to_vec()),
            ("other.csv".into(), b"c".to_vec()),
        ];
        let out = disambiguate_filenames(files);
        assert_eq!(out[0].0, "report.csv");
        assert_eq!(out[1].0, "report (2).csv");
        assert_eq!(out[2].0, "other.csv");
    }

    #[test]
    fn parallel_arrays_require_matching_lengths() {
        let err = payloads_from_parallel_arrays(
            &["a.csv".into(), "b.csv".into()],
            &["content".into()],
        )
        .unwrap_err();
        assert!(err.contains("file_field") || err.contains("mismatch") || err.contains("arquivo"));
    }

    #[test]
    fn parallel_arrays_pair_by_index() {
        let out = payloads_from_parallel_arrays(
            &["a.csv".into(), "b.csv".into()],
            &["AAA".into(), "BBB".into()],
        )
        .unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], ("a.csv".into(), b"AAA".to_vec()));
        assert_eq!(out[1], ("b.csv".into(), b"BBB".to_vec()));
    }
}

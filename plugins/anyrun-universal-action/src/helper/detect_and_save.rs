use dirs::home_dir;
use std::fs;
use std::path::{Path, PathBuf};

pub fn detect_and_save(content: &str) {
    let extension = detect_format(content);
    let desktop_path = home_dir()
        .expect("Không tìm thấy thư mục Home")
        .join("Desktop");

    let final_path = get_available_path(&desktop_path, "output", extension);

    fs::write(&final_path, content).unwrap();
    println!("Đã lưu thành công vào: {:?}", final_path);
}

fn detect_format(content: &str) -> &str {
    let trimmed = content.trim();

    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        "json"
    } else if trimmed.starts_with("<?xml") || (trimmed.starts_with('<') && trimmed.ends_with('>')) {
        "xml"
    } else if trimmed.starts_with("<?php") {
        "php"
    } else if trimmed.starts_with("#!") || trimmed.contains("echo ") {
        "sh"
    } else if content.contains("fn ") && content.contains("let ") && content.contains("match ") {
        "rs"
    } else if content.contains("interface ") || content.contains("type ") && content.contains(':') {
        "ts"
    } else if content.contains("function ")
        || content.contains("var ")
        || content.contains("const ")
    {
        "js"
    } else if content.lines().next().unwrap_or("").contains(',') {
        "csv"
    } else {
        "md"
    }
}

fn get_available_path(dir: &Path, base_name: &str, ext: &str) -> PathBuf {
    let mut path = dir.join(format!("{}.{}", base_name, ext));
    if !path.exists() {
        return path;
    }

    let mut counter = 1;
    loop {
        let new_name = format!("{}{:02}.{}", base_name, counter, ext);
        path = dir.join(new_name);
        if !path.exists() {
            return path;
        }
        counter += 1;
    }
}

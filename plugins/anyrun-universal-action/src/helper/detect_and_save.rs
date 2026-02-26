use crate::action::model::InputCategory;
use crate::helper::model::MagikaOutput;
use dirs::home_dir;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn detect_and_save(content: &str, filetype: InputCategory) {
    let extension = &InputCategory::get_extension(&filetype);
    let desktop_path = home_dir()
        .expect("Cannot find Home directory")
        .join("Desktop");

    let final_path = get_available_path(&desktop_path, "output", extension);

    fs::write(&final_path, content).unwrap();
    println!("Saved sucessfully to: {:?}", final_path);
}

pub fn call_magika(content: &str) -> Option<(String, String, f64)> {
    let mut child = Command::new("magika")
        .args(["--json", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes()).ok();
    }

    let output = child.wait_with_output().ok()?;
    let results: Vec<MagikaOutput> = serde_json::from_slice(&output.stdout).unwrap_or_default();

    results.first().map(|res| {
        (
            res.result.value.output.group.clone(),
            res.result.value.output.label.clone(),
            res.result.value.score,
        )
    })
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

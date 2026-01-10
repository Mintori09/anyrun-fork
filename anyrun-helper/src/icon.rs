use serde::Deserialize;
use std::{borrow::Cow, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub enum SystemIcon {
    Settings,
    UserPassword,
    NetworkStatus,
    Battery,
    Bluetooth,
    Display,
    Sound,

    GoHome,
    GoBack,
    GoNext,
    Menu,

    DocumentSave,
    EditCopy,
    EditPaste,
    EditCut,
    EditDelete,
    ViewRefresh,
    Search,
    ZoomIn,
    ZoomOut,
    MailSend,

    Calculator,
    WebBrowser,
    Dictionary,
    Terminal,
    SystemRun,
    Symbol,
    Language,
    Monitor,
    Url,

    #[default]
    FileText,
    FileImage,
    FileVideo,
    FileAudio,
    FileArchive,
    FileCode,
    FilePdf,
    FileExcel,
    FileWord,
    FilePowerpoint,
    Folder,
    FolderRemote,

    Rust,
    JavaScript,
    TypeScript,
    Python,
    C,
    Cpp,
    Go,
    PHP,
    Lua,
    Shell,
    Nix,
    Json,
    Yaml,
    Toml,
    Html,
    Css,
    Obsidian,
    Rclone,
    Config,
    Firefox,
    Custom(Cow<'static, str>),
}

impl SystemIcon {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Settings => "preferences-system",
            Self::UserPassword => "preferences-desktop-user-password",
            Self::NetworkStatus => "network-workgroup",
            Self::Battery => "battery-good",
            Self::Bluetooth => "bluetooth",
            Self::Display => "video-display",
            Self::Sound => "audio-speakers",

            Self::GoHome => "go-home",
            Self::GoBack => "go-previous",
            Self::GoNext => "go-next",
            Self::Menu => "open-menu",

            Self::DocumentSave => "document-save",
            Self::EditCopy => "edit-copy",
            Self::EditPaste => "edit-paste",
            Self::EditCut => "edit-cut",
            Self::EditDelete => "edit-delete",
            Self::ViewRefresh => "view-refresh",
            Self::Search => "system-search",
            Self::ZoomIn => "zoom-in",
            Self::ZoomOut => "zoom-out",
            Self::MailSend => "mail-send",

            Self::Calculator => "accessories-calculator",
            Self::WebBrowser => "internet-web-browser",
            Self::Dictionary => "accessories-dictionary",
            Self::Terminal => "utilities-terminal",
            Self::SystemRun => "system-run",
            Self::Symbol => "character-set",
            Self::Language => "preferences-desktop-locale",
            Self::Monitor => "preferences-desktop-display",

            Self::FileText => "text-x-generic",
            Self::FileImage => "image-x-generic",
            Self::FileVideo => "video-x-generic",
            Self::FileAudio => "audio-x-generic",
            Self::FileArchive => "package-x-generic",
            Self::FileCode => "text-x-script",
            Self::FilePdf => "application-pdf",
            Self::FileExcel => "x-office-spreadsheet",
            Self::FileWord => "x-office-document",
            Self::FilePowerpoint => "x-office-presentation",
            Self::Folder => "folder",
            Self::FolderRemote => "folder-remote",
            Self::Url => "text-html",

            Self::Rust => "text-rust",
            Self::JavaScript => "text-javascript",
            Self::TypeScript => "text-typescript",
            Self::Python => "text-x-python",
            Self::C => "text-x-csrc",
            Self::Cpp => "text-x-c++src",
            Self::Go => "text-x-go",
            Self::PHP => "application-x-php",
            Self::Lua => "text-x-lua",
            Self::Shell => "text-x-shellscript",
            Self::Nix => "text-x-nix",
            Self::Json => "application-json",
            Self::Yaml => "text-x-yaml",
            Self::Toml => "text-x-toml",
            Self::Html => "text-html",
            Self::Css => "text-css",
            Self::Obsidian => "obsidian",
            Self::Rclone => "rclone-browser",
            Self::Config => "system-config-display",
            Self::Firefox => "firefox",

            _ => "text-x-generic",
        }
    }

    pub fn custom<S: Into<Cow<'static, str>>>(name: S) -> Self {
        Self::Custom(name.into())
    }

    pub fn from_ext(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "js" => Self::JavaScript,
            "ts" => Self::TypeScript,
            "py" => Self::Python,
            "c" | "h" => Self::C,
            "cpp" | "hpp" | "cc" => Self::Cpp,
            "go" => Self::Go,
            "php" => Self::PHP,
            "lua" => Self::Lua,
            "sh" | "bash" | "zsh" => Self::Shell,
            "nix" => Self::Nix,
            "pdf" => Self::FilePdf,
            "doc" | "docx" | "odt" => Self::FileWord,
            "xls" | "xlsx" | "csv" | "ods" => Self::FileExcel,
            "ppt" | "pptx" | "odp" => Self::FilePowerpoint,
            "png" | "jpg" | "jpeg" | "svg" | "webp" | "ico" => Self::FileImage,
            "mp4" | "mkv" | "avi" | "mov" | "webm" => Self::FileVideo,
            "mp3" | "flac" | "wav" | "ogg" | "m4a" => Self::FileAudio,
            "zip" | "tar" | "gz" | "7z" | "rar" | "xz" => Self::FileArchive,
            _ => Self::Folder,
        }
    }
}

pub fn home_dir() -> Option<PathBuf> {
    #[cfg(target_family = "windows")]
    {
        if let Some(v) = env::var_os("USERPROFILE") {
            return Some(PathBuf::from(v));
        }
        let drive = env::var_os("HOMEDRIVE");
        let path = env::var_os("HOMEPATH");
        if let (Some(d), Some(p)) = (drive, path) {
            return Some(PathBuf::from(PathBuf::from(d).join(p)));
        }
    }

    #[cfg(target_family = "unix")]
    {
        use std::env;

        if let Some(home) = env::var_os("HOME") {
            return Some(PathBuf::from(home));
        }
    }

    None
}

use std::path::Path;

use std::process::Command;

pub fn get_icon_path(input: &str) -> String {
    let identifier = input
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("default");

    if !input.contains("://") {
        return identifier.to_string();
    }

    let Ok(home) = std::env::var("HOME") else {
        return "system-search".to_string();
    };

    let cache_dir = format!("{}/.config/anyrun/anyrun-favicons", home);
    let icon_path = format!("{}/{}.png", cache_dir, identifier);

    if Path::new(&icon_path).exists() {
        return icon_path;
    }

    download_favicon_async(&cache_dir, &icon_path, identifier);

    "web-browser".to_string()
}

fn download_favicon_async(cache_dir: &str, dest_path: &str, domain: &str) {
    let _ = std::fs::create_dir_all(cache_dir);

    let dest = dest_path.to_string();
    let url = format!(
        "https://t3.gstatic.com/faviconV2?client=SOCIAL&type=FAVICON&fallback_opts=TYPE,SIZE,URL&url=https://{}&size=64",
        domain
    );

    std::thread::spawn(move || {
        let _ = Command::new("curl")
            .arg("-L")
            .arg("-A")
            .arg("Mozilla/5.0")
            .arg("-s")
            .arg("-o")
            .arg(dest)
            .arg(url)
            .output();
    });
}

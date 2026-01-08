use serde::Deserialize;
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub enum SystemIcon {
    // --- NHÓM HỆ THỐNG & CÀI ĐẶT ---
    Settings,
    UserPassword,
    NetworkStatus,
    Battery,
    Bluetooth,
    Display,
    Sound,

    // --- NHÓM ĐIỀU HƯỚNG ---
    GoHome,
    GoBack,
    GoNext,
    Menu,

    // --- NHÓM HÀNH ĐỘNG (Actions) ---
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

    // --- NHÓM PLUGIN ĐẶC THÙ (Anyrun specific) ---
    Calculator,
    WebBrowser,
    Dictionary,
    Terminal,
    SystemRun,
    Symbol,
    Language,
    Monitor,
    Url,

    // --- NHÓM FILE TYPE & MIME ---
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
    FolderRemote, // Thư mục mạng/cloud

    // --- NHÓM NGÔN NGỮ LẬP TRÌNH CHI TIẾT ---
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
    // --- TÙY CHỈNH ---
    Custom(Cow<'static, str>),
}

impl SystemIcon {
    pub fn as_str(&self) -> &str {
        match self {
            // Hệ thống
            Self::Settings => "preferences-system",
            Self::UserPassword => "preferences-desktop-user-password",
            Self::NetworkStatus => "network-workgroup",
            Self::Battery => "battery-good",
            Self::Bluetooth => "bluetooth",
            Self::Display => "video-display",
            Self::Sound => "audio-speakers",

            // Điều hướng
            Self::GoHome => "go-home",
            Self::GoBack => "go-previous",
            Self::GoNext => "go-next",
            Self::Menu => "open-menu",

            // Hành động
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

            // Plugin
            Self::Calculator => "accessories-calculator",
            Self::WebBrowser => "internet-web-browser",
            Self::Dictionary => "accessories-dictionary",
            Self::Terminal => "utilities-terminal",
            Self::SystemRun => "system-run",
            Self::Symbol => "character-set",
            Self::Language => "preferences-desktop-locale",
            Self::Monitor => "preferences-desktop-display",

            // File Types
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

            // Custom
            _ => "text-x-generic",
        }
    }

    pub fn custom<S: Into<Cow<'static, str>>>(name: S) -> Self {
        Self::Custom(name.into())
    }

    pub fn from_ext(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            // Code
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
            // Documents
            "pdf" => Self::FilePdf,
            "doc" | "docx" | "odt" => Self::FileWord,
            "xls" | "xlsx" | "csv" | "ods" => Self::FileExcel,
            "ppt" | "pptx" | "odp" => Self::FilePowerpoint,
            // Media
            "png" | "jpg" | "jpeg" | "svg" | "webp" | "ico" => Self::FileImage,
            "mp4" | "mkv" | "avi" | "mov" | "webm" => Self::FileVideo,
            "mp3" | "flac" | "wav" | "ogg" | "m4a" => Self::FileAudio,
            // Archives
            "zip" | "tar" | "gz" | "7z" | "rar" | "xz" => Self::FileArchive,
            // Default
            _ => Self::Folder,
        }
    }
}

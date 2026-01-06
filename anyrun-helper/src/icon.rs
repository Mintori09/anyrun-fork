use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    Calculator, // Dùng cho plugin anyrun-calc
    WebBrowser, // Dùng cho plugin websearch
    Dictionary, // Dùng cho plugin dictionary
    Terminal,   // Dùng cho plugin shell/stdin
    SystemRun,  // Dùng cho plugin nix-run/applications
    Symbol,     // Dùng cho plugin symbols
    Language,   // Dùng cho plugin translate
    Monitor,    // Dùng cho plugin randr (cấu hình màn hình)

    // --- NHÓM FILE TYPE & MIME ---
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

            // Custom
            Self::Custom(name) => name,
        }
    }

    /// Khởi tạo icon tùy chỉnh nhanh
    pub fn custom<S: Into<Cow<'static, str>>>(name: S) -> Self {
        Self::Custom(name.into())
    }

    /// Mở rộng hàm đoán Icon với nhiều định dạng hơn
    pub fn from_ext(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            // Code
            "rs" | "js" | "ts" | "py" | "c" | "cpp" | "h" | "go" | "nix" | "lua" => Self::FileCode,
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

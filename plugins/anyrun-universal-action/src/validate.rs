pub fn is_youtube(clip: &str) -> bool {
    clip.contains("youtube.com") || clip.contains("youtu.be")
}

pub fn is_shorten_path(clip: &str) -> bool {
    clip.contains("$HOME") || clip.contains("~")
}

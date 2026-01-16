pub fn is_youtube(clip: &str) -> bool {
    clip.contains("youtube.com") || clip.contains("youtu.be")
}

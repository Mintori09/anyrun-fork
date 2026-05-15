use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub app_id: Option<String>,
    pub workspace: Option<String>,
    pub icon: Option<String>,
    pub app_name: Option<String>,
}

pub trait WindowBackend: Send + Sync {
    fn list_windows(&self) -> Vec<WindowInfo>;
    fn focus_window(&self, id: &str) -> Result<(), String>;
    fn name(&self) -> &'static str;
}

impl WindowBackend for Box<dyn WindowBackend> {
    fn list_windows(&self) -> Vec<WindowInfo> {
        self.as_ref().list_windows()
    }
    fn focus_window(&self, id: &str) -> Result<(), String> {
        self.as_ref().focus_window(id)
    }
    fn name(&self) -> &'static str {
        self.as_ref().name()
    }
}

pub struct CachedBackend<T: WindowBackend> {
    inner: T,
    cache: Mutex<Option<(std::time::Instant, Vec<WindowInfo>)>>,
    ttl: std::time::Duration,
}

impl<T: WindowBackend> CachedBackend<T> {
    pub fn new(inner: T, ttl_secs: u64) -> Self {
        Self {
            inner,
            cache: Mutex::new(None),
            ttl: std::time::Duration::from_secs(ttl_secs),
        }
    }
}

impl<T: WindowBackend> WindowBackend for CachedBackend<T> {
    fn list_windows(&self) -> Vec<WindowInfo> {
        let should_refresh = {
            let cache = self.cache.lock().unwrap();
            match cache.as_ref() {
                Some((ts, _)) if ts.elapsed() < self.ttl => {
                    return cache.as_ref().unwrap().1.clone();
                }
                _ => true,
            }
        };

        if should_refresh {
            let windows = self.inner.list_windows();
            let mut cache = self.cache.lock().unwrap();
            *cache = Some((std::time::Instant::now(), windows.clone()));
            windows
        } else {
            let cache = self.cache.lock().unwrap();
            cache.as_ref().unwrap().1.clone()
        }
    }

    fn focus_window(&self, id: &str) -> Result<(), String> {
        self.inner.focus_window(id)
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }
}

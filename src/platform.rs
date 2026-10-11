//! The few things that differ between the desktop build and the browser:
//! where saves live, how a file reaches the player, and launch options
//! (command-line environment natively, URL query parameters on the web).

#[derive(Clone, Debug, Default)]
pub struct Launch {
    /// Play the story by itself (`CPL_AUTOPLAY=1` / `?autoplay`).
    pub autoplay: bool,
    /// Seconds per dialogue line and puzzle step while autoplaying (0 = flat out).
    pub pace: f32,
    /// Walking speed multiplier while autoplaying.
    pub speed: f32,
    /// Seed phrase for a reproducible new game.
    pub seed: Option<String>,
    /// Write screenshots here at milestones (native only).
    pub shots: Option<String>,
    /// Also take one every this many seconds.
    pub shot_every: Option<f32>,
    /// Quit once the credits have rolled (native only, for tests).
    pub exit_at_end: bool,
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::path::PathBuf;

    use super::Launch;

    pub fn launch() -> Launch {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let autoplay = var("CPL_AUTOPLAY").is_some_and(|v| v != "0");
        Launch {
            autoplay,
            pace: var("CPL_PACE").and_then(|v| v.parse().ok()).unwrap_or(0.0),
            speed: var("CPL_SPEED").and_then(|v| v.parse().ok()).unwrap_or(1.0),
            seed: var("CPL_SEED"),
            shots: var("CPL_SHOTS"),
            shot_every: var("CPL_SHOT_EVERY").and_then(|v| v.parse().ok()),
            exit_at_end: var("CPL_EXIT_AT_END").is_some_and(|v| v != "0"),
        }
    }

    fn data_dir() -> PathBuf {
        if let Ok(d) = std::env::var("CPL_DATA_DIR") {
            return PathBuf::from(d);
        }
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
        if cfg!(target_os = "macos") {
            home.join("Library/Application Support/Cypherpunk Legacy")
        } else if cfg!(target_os = "windows") {
            std::env::var("APPDATA").map(PathBuf::from).unwrap_or(home).join("Cypherpunk Legacy")
        } else {
            std::env::var("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|_| home.join(".local/share")).join("cypherpunk-legacy")
        }
    }

    fn read(name: &str) -> Option<String> {
        std::fs::read_to_string(data_dir().join(name)).ok()
    }

    fn write(name: &str, contents: &str) -> bool {
        let dir = data_dir();
        if std::fs::create_dir_all(&dir).is_err() {
            return false;
        }
        // write-then-rename, so a crash never leaves half a save
        let tmp = dir.join(format!("{name}.tmp"));
        std::fs::write(&tmp, contents).is_ok() && std::fs::rename(&tmp, dir.join(name)).is_ok()
    }

    pub fn load_save() -> Option<String> {
        read("save.json")
    }

    pub fn store_save(json: &str) -> bool {
        write("save.json", json)
    }

    pub fn load_settings() -> Option<String> {
        read("settings.json")
    }

    pub fn store_settings(json: &str) -> bool {
        write("settings.json", json)
    }

    /// Save a file for the player; returns where it went.
    pub fn download(name: &str, contents: &str) -> Option<String> {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).map(PathBuf::from).ok();
        let downloads = home.map(|h| h.join("Downloads")).filter(|d| d.is_dir()).unwrap_or_else(data_dir);
        std::fs::create_dir_all(&downloads).ok()?;
        let path = downloads.join(name);
        std::fs::write(&path, contents).ok()?;
        Some(path.display().to_string())
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use wasm_bindgen::JsCast;

    use super::Launch;

    const SAVE_KEY: &str = "cpl3.save";
    const SETTINGS_KEY: &str = "cpl3.settings";

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok().flatten()
    }

    fn param(name: &str) -> Option<String> {
        let search = web_sys::window()?.location().search().ok()?;
        let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
        params.get(name)
    }

    pub fn launch() -> Launch {
        let autoplay = param("autoplay");
        let demo = autoplay.as_deref() == Some("demo");
        Launch {
            autoplay: autoplay.is_some(),
            pace: param("pace").and_then(|v| v.parse().ok()).unwrap_or(if demo { 1.4 } else { 0.0 }),
            speed: param("speed").and_then(|v| v.parse().ok()).unwrap_or(1.0),
            seed: param("seed"),
            shots: None,
            shot_every: None,
            exit_at_end: false,
        }
    }

    pub fn load_save() -> Option<String> {
        storage()?.get_item(SAVE_KEY).ok().flatten()
    }

    pub fn store_save(json: &str) -> bool {
        storage().is_some_and(|s| s.set_item(SAVE_KEY, json).is_ok())
    }

    pub fn load_settings() -> Option<String> {
        storage()?.get_item(SETTINGS_KEY).ok().flatten()
    }

    pub fn store_settings(json: &str) -> bool {
        storage().is_some_and(|s| s.set_item(SETTINGS_KEY, json).is_ok())
    }

    pub fn download(name: &str, contents: &str) -> Option<String> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let parts = js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(contents));
        let props = web_sys::BlobPropertyBag::new();
        props.set_type("text/plain;charset=utf-8");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &props).ok()?;
        let url = web_sys::Url::create_object_url_with_blob(&blob).ok()?;
        let a: web_sys::HtmlAnchorElement = document.create_element("a").ok()?.dyn_into().ok()?;
        a.set_href(&url);
        a.set_download(name);
        document.body()?.append_child(&a).ok()?;
        a.click();
        a.remove();
        let _ = web_sys::Url::revoke_object_url(&url);
        Some(name.to_string())
    }
}

pub use imp::*;

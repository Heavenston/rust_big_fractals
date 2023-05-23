pub mod utils;

use std::{path::{Path, PathBuf}, process::Stdio};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub available_levels: Vec<u32>,
    pub format: String,
    pub render_command: Option<String>,
}

pub struct FormattedBigImage {
    folder: PathBuf,
    manifest: Manifest,
}

impl FormattedBigImage {
    pub async fn load_folder(
        path: impl AsRef<Path>
    ) -> Self {
        let manifest_content = tokio::fs::read_to_string(
            path.as_ref().join("manifest.json")
        ).await.unwrap();
        let manifest: Manifest = serde_json::from_str(&manifest_content)
            .unwrap();

        Self {
            folder: path.as_ref().into(),
            manifest,
        }
    }

    pub fn is_level_available(&self, level: u32) -> bool {
        self.manifest.available_levels.len() == 0 ||
        self.manifest.available_levels.contains(&level)
    }

    pub fn max_level_available(&self) -> Option<u32> {
        self.manifest.available_levels.iter().copied().max()
    }

    async fn render_section(&self, level: u32, x: u32, y: u32) {
        let command = self.manifest.render_command.as_ref()
            .expect("Invalid call to render section")
            .replace("%LEVEL%", &level.to_string())
            .replace("%X%", &x.to_string())
            .replace("%Y%", &y.to_string())
            .replace("%FORMAT%", &self.manifest.format);
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::null())
            .current_dir(&self.folder)
            .spawn().expect("Could not run render process")
            .wait().await.expect("Error while running render process");
    }

    pub async fn load(&self, level: u32, x: u32, y: u32) -> Option<image::RgbaImage> {
        log::trace!("Trying to load section {level}_{x}x{y}");
        let path = self.folder.join(
            &format!("{level}_{x}x{y}.{}", self.manifest.format)
        );
        let image = image::open(&path).ok().map(|x| x.to_rgba8());

        if let Some(i) = image
        { Some(i) }
        else if self.manifest.render_command.is_some() {
            log::trace!("Could not open image file, will try to render it");
            self.render_section(level, x, y).await;
            image::open(&path).ok().map(|x| x.to_rgba8())
        }
        else {
            log::trace!("Could not open image file");
            None
        }
    }
}


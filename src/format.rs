pub mod utils;

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub available_levels: Vec<u32>,
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
            manifest
        }
    }

    pub fn is_level_available(&self, level: u32) -> bool {
        self.manifest.available_levels.contains(&level)
    }

    pub fn max_level_available(&self) -> Option<u32> {
        self.manifest.available_levels.iter().copied().max()
    }

    pub async fn load(&self, level: u32, x: u32, y: u32) -> Option<image::RgbaImage> {
        image::open(self.folder.join(&format!("{level}_{x}x{y}.webp"))).ok()
            .map(|x| x.to_rgba8())
    }
}


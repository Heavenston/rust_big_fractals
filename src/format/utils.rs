use image::GenericImage;
use std::{path::Path, collections::{HashSet, HashMap}, sync::{Arc, Mutex}};

pub async fn extrapolate_levels(path: impl AsRef<Path>) {
    let path = path.as_ref();
    let mut files = tokio::fs::read_dir(path).await.unwrap();

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    struct Section {
        level: u32,
        x: u32,
        y: u32,
    }

    let mut levels = HashMap::<u32, HashSet<Section>>::new();
    while let Some(entry) = files.next_entry().await.unwrap() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        let Some((name, "webp")) = file_name.split_once('.')
            else { continue };
        let Some((level_str, pos_str)) = name.split_once('_')
            else { continue };
        let Some((pos_x_str, pos_y_str)) = pos_str.split_once('x')
            else { continue };
        let Ok(level) = level_str.parse::<u32>()
            else { continue };
        let Ok(x) = pos_x_str.parse::<u32>()
            else { continue };
        let Ok(y) = pos_y_str.parse::<u32>()
            else { continue };
        let s = Section { level, x, y };
        levels.entry(level).or_default()
            .insert(s);
    }

    let deepest_level = levels.keys().copied().max();
    println!("Deepest level is {deepest_level:?}");
    let mut current_filling_level = deepest_level.unwrap_or(0);
    while current_filling_level >= 2 {
        current_filling_level /= 2;
        println!("Filling level {current_filling_level}");

        let level = &*levels.entry(current_filling_level).or_default();
        rayon::scope(|s| {
            for sx in 0..current_filling_level {
                for sy in 0..current_filling_level {
                    s.spawn(move |_| {
                        let filling_section = Section { level: current_filling_level, x: sx, y: sy };
                        if level.contains(&filling_section) { return };
                        println!("{sx}x{sy} is missing");

                        let mut reconstructed = image::RgbaImage::new(4096, 4096);

                        let sub_level = current_filling_level * 2;

                        println!("Reading from sub level");
                        for dx in 0..2 {
                            for dy in 0..2 {
                                let nsx = sx * 2 + dx;
                                let nsy = sy * 2 + dy;
                                println!("{sub_level}_{nsx}x{nsy}.webp");
                                if let Ok(si) = image::open(path.join(&format!("{sub_level}_{nsx}x{nsy}.webp"))) {
                                    reconstructed.copy_from(&si, 2048 * dx, 2048 * dy)
                                        .unwrap();
                                }
                            }
                        }

                        println!("Resizing");
                        let resized = image::imageops::resize(
                            &reconstructed, 2048, 2048, image::imageops::Lanczos3);

                        println!("Saving");
                        resized.save(path.join(&format!("{current_filling_level}_{sx}x{sy}.webp")))
                            .unwrap();
                    });
                }
            }
        });
    }

    let manifest = crate::format::Manifest {
        available_levels: (1..deepest_level.unwrap_or(0)).collect(),
    };
    let manifest_json = serde_json::to_string(&manifest).unwrap();
    tokio::fs::write(path.join("manifest.json"), &manifest_json).await.unwrap();
}


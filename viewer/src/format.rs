pub mod utils;

use std::{path::{Path, PathBuf}, process::Stdio, sync::{Arc, Mutex}};

use tokio::sync::{ Notify, oneshot };

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub available_levels: Vec<u32>,
    pub format: String,
    pub render_command: Option<String>,
}

type RenderTask = (u32, u32, u32, oneshot::Sender<()>);

pub struct FormattedBigImage {
    folder: PathBuf,
    manifest: Manifest,

    render_queue: Arc<Mutex<Vec<RenderTask>>>,
    render_notify: Arc<Notify>,
}

impl FormattedBigImage {
    pub async fn load_folder(
        path: impl AsRef<Path>
    ) -> Self {
        let path = path.as_ref();

        let manifest_content = tokio::fs::read_to_string(
            path.join("manifest.json")
        ).await.unwrap();
        let manifest: Manifest = serde_json::from_str(&manifest_content)
            .unwrap();

        let render_queue = Arc::new(Mutex::new(Vec::<RenderTask>::new()));
        let render_notify = Arc::new(Notify::new());

        tokio::spawn({
            let render_queue = Arc::clone(&render_queue);
            let render_notify = Arc::clone(&render_notify);
            let manifest = manifest.clone();
            let folder = path.to_owned();

            async move {
                loop {
                    let task = render_queue.lock().unwrap().pop();
                    let Some((level, x, y, sender)) = task else {
                        render_notify.notified().await;
                        continue;
                    };
                    log::trace!("Begining render task of {level}_{x}x{y}");

                    let command = manifest.render_command.as_ref()
                        .expect("Invalid call to render section")
                        .replace("%LEVEL%", &level.to_string())
                        .replace("%X%", &x.to_string())
                        .replace("%Y%", &y.to_string())
                        .replace("%FORMAT%", &manifest.format);
                    #[cfg(target_os = "linux")]
                    let mut c = {
                        let mut c = tokio::process::Command::new("sh");
                        c.arg("-c");
                        c
                    };
                    #[cfg(target_os = "windows")]
                    let mut c = {
                        let mut c = tokio::process::Command::new("powershell.exe");
                        c.arg("-Command");
                        c
                    };

                    c
                        .arg(command)
                        .stdout(Stdio::null())
                        .current_dir(&folder)
                        .spawn().expect("Could not run render process")
                        .wait().await.expect("Error while running render process");

                    sender.send(()).unwrap();
                }
                
            }
        });

        Self {
            folder: path.to_owned(),
            manifest,

            render_queue,
            render_notify
        }
    }

    pub fn is_level_available(&self, level: u32) -> bool {
        self.manifest.available_levels.len() == 0 ||
        self.manifest.available_levels.contains(&level)
    }

    pub fn max_level_available(&self) -> Option<u32> {
        self.manifest.available_levels.iter().copied().max()
    }

    pub fn render_queue_length(&self) -> usize {
        self.render_queue.lock().unwrap().len()
    }

    async fn render_section(&self, level: u32, x: u32, y: u32) {
        let (sender, receiver) = oneshot::channel::<()>();
        self.render_queue.lock().unwrap().push((
            level, x, y, sender
        ));
        self.render_notify.notify_one();
        let () = receiver.await.unwrap();
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


#![allow(dead_code)]
use big_image_viewer::*;

fn main() {
    tokio::runtime::Builder::new_current_thread()
        .build().expect("Could not create runtime")
        .block_on(async move {
            app::start_app().await;
            // format::utils::extrapolate_levels("images").await;
        });
}

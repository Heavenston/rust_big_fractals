use std::path::Path;

use anyhow::bail;

#[async_recursion::async_recursion]
pub async fn preproces_file(path: &Path) -> anyhow::Result<String> {
    let s = tokio::fs::read_to_string(path).await?;
    return Ok(preprocess(path, &s).await?);
}

#[async_recursion::async_recursion]
pub async fn preprocess(path: &Path, content: &str) -> anyhow::Result<String> {
    let mut out = String::new();

    for line in content.lines() {
        if !line.starts_with('#')
        { out += line; out += "\n"; continue; }

        if line.starts_with("#include \"") && line.trim_end().ends_with('"') {
            let file_to_include = line
                .strip_prefix("#include \"").unwrap()
                .trim_end().strip_suffix("\"")
                .unwrap();
            let ipath = path.with_file_name("").join(file_to_include);
            out += "\n";
            out += &preproces_file(&ipath).await?;
            out += "\n";
        }
        else {
            bail!("Invalid preprocessor macro")
        }
    }

    Ok(out)
}


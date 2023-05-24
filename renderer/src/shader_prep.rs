use std::{path::Path, collections::HashMap};

use anyhow::{ bail, anyhow};

#[derive(Debug, Clone, Default)]
struct PreprocessContext {
    vars: HashMap<String, String>,
}

#[async_recursion::async_recursion]
pub async fn preproces_file(path: &Path) -> anyhow::Result<String> {
    let s = &tokio::fs::read_to_string(path).await?;
    return Ok(preprocess(path, &s, &mut Default::default()).await?);
}

#[async_recursion::async_recursion]
async fn preprocess(
    path: &Path,
    content: &str,
    context: &mut PreprocessContext
) -> anyhow::Result<String> {
    let mut out = String::new();

    for line in content.lines() {
        if !line.starts_with("//#") {
            out += context.vars.iter().fold(line.to_string(), |a, (k, v)| {
                a.replace(k, v)
            }).as_str();
            out += "\n";
            continue;
        }

        if line.starts_with("//#include \"") && line.trim_end().ends_with('"') {
            let file_to_include = line
                .strip_prefix("//#include \"").unwrap()
                .trim_end().strip_suffix("\"")
                .unwrap();
            let ipath = path.with_file_name("").join(file_to_include);
            out += "\n";
            out += &preprocess(&ipath, &tokio::fs::read_to_string(&ipath).await?, context).await?;
            out += "\n";
        }
        else if line.starts_with("//#define ") || line.starts_with("//#default ") {
            // Wdym its ugly ??
            let rest = line
                .strip_prefix("//#define ").unwrap_or(line)
                .strip_prefix("//#default ").unwrap_or(
                    line.strip_prefix("//#define ").unwrap_or(line)
                );
            let (variable_name, value) = rest.trim().split_once(' ')
                .ok_or(anyhow!("Invalid preprocessor macro"))?;
            if line.starts_with("//#default ") && context.vars.contains_key(variable_name)
            { continue; }
            context.vars.insert(variable_name.into(), value.into());
        }
        else {
            bail!("Invalid preprocessor macro")
        }
    }

    Ok(out)
}


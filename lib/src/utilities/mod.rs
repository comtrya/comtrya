use which;

pub fn get_binary_path(binary: &str) -> Result<String, anyhow::Error> {
    let binary = which::which(String::from(binary))?
        .to_string_lossy()
        .to_string();

    Ok(binary)
}

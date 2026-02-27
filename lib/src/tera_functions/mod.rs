use tera::{Function, Result, Tera, Value};

pub struct ReadFileContents;

impl Function for ReadFileContents {
    fn call(&self, args: &std::collections::HashMap<String, Value>) -> Result<Value> {
        match args.get("path") {
            Some(value) => match value.as_str() {
                Some(path) => match std::fs::read_to_string(path) {
                    Ok(content) => Ok(content.trim().into()),
                    Err(err) => Err(err.into()),
                },

                None => Err(format!(
                    "Path: '{value}'. Error: Cannot convert argument 'path' to str"
                )
                .into()),
            },

            None => Err("Argument 'path' not set".into()),
        }
    }
}

pub fn register_functions(tera: &mut Tera) {
    tera.register_function("read_file_contents", ReadFileContents);
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Write;
    use tera::{Context, Tera};

    #[test]
    fn can_read_from_file() -> anyhow::Result<()> {
        let mut tera = Tera::default();
        tera.register_function("read_file_contents", ReadFileContents);

        let mut file = tempfile::NamedTempFile::new()?;

        let file_content = r#"
FKBR
KUCI
SXOE

"#;

        write!(file.as_file_mut(), "{file_content}")?;

        let template = format!(
            "{{{{ read_file_contents(path=\"{}\") }}}}",
            file.path().display()
        );

        let content = tera.render_str(&template, &Context::new())?;

        let expected_file_content = r#"FKBR
KUCI
SXOE"#;

        assert_eq!(expected_file_content, content);

        Ok(())
    }
}

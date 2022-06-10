use std::io::BufRead;
use std::io::BufReader;
use tera::{Function, Result, Tera, Value};

pub struct ReadFileContents;

impl Function for ReadFileContents {
    fn call(&self, args: &std::collections::HashMap<String, Value>) -> Result<Value> {
        match args.get("path") {
            Some(value) => match value.as_str() {
                Some(path) => match std::fs::File::open(path) {
                    Ok(file) => {
                        // This avoids reading the newline at the end of the file
                        // https://doc.rust-lang.org/std/fs/fn.read_to_string.html reads the newline
                        let contents = BufReader::new(file).lines().flatten().collect::<String>();

                        Ok(contents.into())
                    }
                    Err(err) => Err(err.into()),
                },

                None => Err(format!(
                    "Path: '{}'. Error: Cannot convert argument 'path' to str",
                    value
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

        write!(file.as_file_mut(), "FKBR KUCI SXOE")?;

        let template = format!(
            "{{{{ read_file_contents(path=\"{}\") }}}}",
            file.path().display()
        );

        let content = tera.render_str(&template, &Context::new())?;

        assert_eq!("FKBR KUCI SXOE", content);

        Ok(())
    }
}

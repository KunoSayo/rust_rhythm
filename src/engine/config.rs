use anyhow::{self, Ok};
use toml_edit::Document;

#[allow(unused)]
#[derive(Default, Debug, Clone)]
pub struct Config {
    toml: Document,
    dirty: bool,
}

#[allow(unused)]
impl Config {
    pub fn load(data: &str) -> anyhow::Result<Self> {
        let toml = data.parse::<Document>();
        Ok(Self { toml: toml?, dirty: false })
    }

    pub fn reload(&mut self, data: &str) -> anyhow::Result<()> {
        self.toml = data.parse()?;
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn toml(&self) -> &Document {
        &self.toml
    }

    pub fn toml_mut(&mut self) -> &mut Document {
        self.dirty = true;
        &mut self.toml
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.toml.get(key).and_then(|x| x.as_str())
    }
}
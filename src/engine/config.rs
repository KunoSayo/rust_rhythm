use anyhow::{self, Ok};
use toml_edit::{value, DocumentMut};

#[allow(unused)]
#[derive(Default, Debug, Clone)]
pub struct Config {
    toml: DocumentMut,
    dirty: bool,
}

#[allow(unused)]
impl Config {
    pub fn load(data: &str) -> anyhow::Result<Self> {
        let toml = data.parse::<DocumentMut>();
        Ok(Self { toml: toml?, dirty: false })
    }

    pub fn reload(&mut self, data: &str) -> anyhow::Result<()> {
        self.toml = data.parse()?;
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn toml(&self) -> &DocumentMut {
        &self.toml
    }

    pub fn toml_mut(&mut self) -> &mut DocumentMut {
        self.dirty = true;
        &mut self.toml
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.toml.get(key).and_then(|x| x.as_str())
    }

    pub fn get_f32_def(&mut self, key: &str, def: f32) -> f32 {
        self.toml.get(key).and_then(|x| x.as_float())
            .unwrap_or_else(|| {
                self.dirty = true;
                self.toml_mut().insert(key, value(def as f64));

                def as f64
            }) as f32
    }

    pub fn check_save(&mut self) {
        if self.is_dirty() {
            std::fs::write("cfg.toml", self.toml.to_string());
            self.dirty = false;
        }
    }
}
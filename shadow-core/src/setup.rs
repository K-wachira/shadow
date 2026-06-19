use crate::config::Config;
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
pub struct ShadowPaths {
    pub db: PathBuf,
    pub mind: PathBuf,
    pub config: PathBuf,
    pub mind_skill: PathBuf,
    pub log: PathBuf,
    pub identity: PathBuf,
}

impl ShadowPaths {
    pub fn new() -> Self {
        let root = dirs::home_dir()
            .expect("could not resolve home directory")
            .join(".shadow");
        Self {
            db: root.join("db/shadow.db"),
            mind: root.join("mind/shadow_mind.json5"),
            mind_skill: root.join("skill/shadow_mind_skill.md"),
            config: root.join("config.toml"),
            log: root.join("shadow.log"),
            identity: root.join("identity"),
        }
    }
}

pub fn run_setup() -> color_eyre::Result<(Config, ShadowPaths)> {
    let paths = ShadowPaths::new();
    fs::create_dir_all(paths.db.parent().unwrap())?;
    fs::create_dir_all(paths.mind.parent().unwrap())?;
    fs::create_dir_all(paths.mind_skill.parent().unwrap())?;
    fs::create_dir_all(&paths.identity)?;

    if !paths.config.exists() {
        fs::write(&paths.config, toml::to_string_pretty(&Config::default())?)?;
    }
    let config: Config = toml::from_str(&fs::read_to_string(&paths.config)?)?;

    // The person-model file is created on first (sealed) save — no plaintext
    // stub is written (INV-9).
    if !paths.mind_skill.exists() {
        fs::write(&paths.mind_skill, "{\n  ## shadow skill\n}")?;
    }
    if !paths.log.exists() {
        fs::File::create(&paths.log)?;
    }

    Ok((config, paths))
}

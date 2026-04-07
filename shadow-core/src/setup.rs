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
        }
    }
}

pub fn run_setup() -> color_eyre::Result<(Config, ShadowPaths)> {
    let paths = ShadowPaths::new();
    fs::create_dir_all(paths.db.parent().unwrap())?;
    fs::create_dir_all(paths.mind.parent().unwrap())?;
    fs::create_dir_all(paths.mind_skill.parent().unwrap())?;

    if !paths.config.exists() {
        fs::write(&paths.config, toml::to_string_pretty(&Config::default())?)?;
    }
    let config: Config = toml::from_str(&fs::read_to_string(&paths.config)?)?;

    if !paths.mind.exists() {
        fs::write(&paths.mind, "{\n  // shadow.mind\n}")?;
    }
    if !paths.mind_skill.exists() {
        fs::write(&paths.mind_skill, "{\n  ## shadow skill\n}")?;
    }
    if !paths.log.exists() {
        fs::File::create(&paths.log)?;
    }

    Ok((config, paths))
}
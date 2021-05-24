use anyhow::Result;

#[derive(Debug)]
pub struct Config {
    pub user: String,
    pub host: String,
}

pub struct Remote(pub String);

pub trait Backend {
    fn read_config(&self) -> Result<Option<Config>>;
    fn write_config(&self, config: &Config) -> Result<()>;
    fn fetch_remote_refs(&self, config: &Config, remote: &Remote) -> Result<()>;
}

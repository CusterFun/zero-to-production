//! src/configuration.rs
#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabasesSettings,
    pub application_port: u16,
}

#[derive(serde::Deserialize)]
pub struct DatabasesSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabasesSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("configuration"))
        .build()?;

    settings.try_deserialize()
}
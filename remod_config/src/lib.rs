use serde::Deserialize;
use serde_json::Result;
#[derive(Debug, Deserialize)]
pub struct Config {
    /// the root directory to use for scanning
    #[serde(default)]
    pub root_dir: String,
    /// A glob pattern matcher of files to include for scanning
    #[serde(default)]
    pub glob: String,
    /// Optional boolean for the parser to use typescript config
    #[serde(default)]
    pub typescript: Option<bool>,
    /// The display name prefix to use for the components when display names command is run
    #[serde(default)]
    pub display_name_prefix: Option<String>,
    /// The file extension pattern to use when creating story files
    #[serde(default)]
    pub story_file_ext: Option<String>,
    /// Ignore the pattern matched in glob
    #[serde(default)]
    pub ignore: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root_dir: String::from("./"),
            glob: String::from("**/*/*.tsx"),
            typescript: Default::default(),
            display_name_prefix: Default::default(),
            ignore: Default::default(),
            story_file_ext: Some(String::from(".stories.tsx")),
        }
    }
}
impl Config {
    pub fn parse_config(&self, contents: String) -> Config {
        let config: Result<Config> = serde_json::from_str(contents.as_str());
        match config {
            Ok(cfg) => cfg,
            Err(_) => Config::default(),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }

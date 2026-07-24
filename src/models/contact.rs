use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relationship {
    Friend,
    Colleague,
    Client,
    Mentor,
    Mentee,
    Acquaintance,
    Family,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Active,
    Dormant,
    LostTouch,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub pronouns: String,

    // Contact info
    #[serde(default)]
    pub email: Vec<String>,
    #[serde(default)]
    pub phone: Vec<String>,
    #[serde(default)]
    pub address: Vec<String>,

    // Professional
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub industry: String,
    #[serde(default)]
    pub linkedin: String,

    // Social
    #[serde(default)]
    pub twitter: String,
    #[serde(default)]
    pub facebook: String,
    #[serde(default)]
    pub instagram: String,
    #[serde(default)]
    pub github: String,
    #[serde(default)]
    pub website: String,

    // Personal
    #[serde(default)]
    pub birthday: Option<NaiveDate>,
    #[serde(default)]
    pub interests: Vec<String>,
    #[serde(default)]
    pub family: Vec<String>,

    // Relationship
    #[serde(default)]
    pub how_we_met: String,
    #[serde(default)]
    pub met_date: Option<NaiveDate>,
    #[serde(default)]
    pub introduced_by: String,
    #[serde(default)]
    pub relationship: Option<Relationship>,
    #[serde(default)]
    pub tags: Vec<String>,

    // CRM
    #[serde(default)]
    pub status: Option<Status>,
    #[serde(default)]
    pub follow_up_cadence: String,
    #[serde(default)]
    pub last_contacted: Option<NaiveDate>,
    #[serde(default)]
    pub next_follow_up: Option<NaiveDate>,
    #[serde(default)]
    pub priority: Option<Priority>,

    // Source
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub source_id: String,
    #[serde(default)]
    pub etag: String,
}

impl Contact {
    pub fn slug(&self) -> String {
        self.name
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect()
    }
}

/// A contact file = parsed frontmatter + raw markdown body
#[derive(Debug)]
pub struct ContactFile {
    pub contact: Contact,
    pub body: String,
    pub path: std::path::PathBuf,
    /// Raw YAML text between --- delimiters, preserving comments and field order
    pub raw_frontmatter: String,
}

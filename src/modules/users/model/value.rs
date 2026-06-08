use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum UserRole {
    Client,
    Worker,
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Client => write!(f, "Client"),
            Self::Worker => write!(f, "Worker"),
        }
    }
}

impl FromStr for UserRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Client" => Ok(Self::Client),
            "Worker" => Ok(Self::Worker),
            _ => Err(format!("unsupported user role: {value}")),
        }
    }
}

use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum OrderStatus {
    Pending,
    Accepted,
    Completed,
    Cancelled,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Accepted => write!(f, "Accepted"),
            Self::Completed => write!(f, "Completed"),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl FromStr for OrderStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Pending" => Ok(Self::Pending),
            "Accepted" => Ok(Self::Accepted),
            "Completed" => Ok(Self::Completed),
            "Cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("unsupported order status: {value}")),
        }
    }
}

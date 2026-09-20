use crate::models::FracClass;
use chrono::NaiveDate;
use serde::Serialize;

/// What the lawn owner should do about a disease right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ManagementAction {
    NoAction,
    /// Cultural practices only; watch the outlook.
    Monitor,
    /// A cultural fix is the primary remedy (e.g. nitrogen for red thread).
    Cultural,
    ApplyPreventative,
    /// A logged fungicide application still covers this disease.
    Protected,
}

/// Relative field efficacy, summarizing university extension ratings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Efficacy {
    Fair,
    Good,
    Excellent,
}

#[derive(Debug, Clone, Serialize)]
pub struct FungicideOption {
    pub frac_class: FracClass,
    pub class_label: String,
    pub examples: Vec<String>,
    pub efficacy: Efficacy,
    pub note: Option<String>,
    /// Most recent single-site class applied this season — rotate away from it.
    pub last_used: bool,
    /// The rotation-aware pick for the next application.
    pub recommended: bool,
}

/// A preventative or curative spray program.
#[derive(Debug, Clone, Serialize)]
pub struct FungicideProgram {
    pub when: String,
    pub interval: String,
    pub guidance: Vec<String>,
    pub options: Vec<FungicideOption>,
}

/// Coverage from the most recent logged fungicide that is effective on this disease.
#[derive(Debug, Clone, Serialize)]
pub struct ProtectionStatus {
    pub product: String,
    pub class_label: String,
    pub applied_on: NaiveDate,
    pub protected_through: NaiveDate,
    pub days_remaining: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiseaseManagement {
    pub action: ManagementAction,
    pub headline: String,
    pub cultural: Vec<String>,
    pub preventative: FungicideProgram,
    pub curative: FungicideProgram,
    pub protection: Option<ProtectionStatus>,
    pub notes: Vec<String>,
}

/// A logged fungicide application, resolved to its FRAC class where recognizable.
#[derive(Debug, Clone, PartialEq)]
pub struct FungicideRecord {
    pub date: NaiveDate,
    pub product: String,
    pub class: Option<FracClass>,
}

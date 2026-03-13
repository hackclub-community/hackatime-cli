use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct UserProfile {
    pub id: Option<u64>,
    pub github_username: Option<String>,
    pub emails: Option<Vec<String>>,
}

impl UserProfile {
    pub fn display_name(&self) -> String {
        self.github_username
            .clone()
            .or_else(|| {
                self.emails
                    .as_ref()
                    .and_then(|emails| emails.first().cloned())
                    .and_then(|email| email.split('@').next().map(ToOwned::to_owned))
            })
            .unwrap_or_else(|| "hackatime".to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DurationResponse {
    pub total_seconds: Option<f64>,
    pub text: Option<String>,
    pub human_readable_total: Option<String>,
    #[serde(flatten)]
    pub extra: Value,
}

impl DurationResponse {
    pub fn display(&self) -> String {
        self.text
            .clone()
            .or_else(|| self.human_readable_total.clone())
            .or_else(|| {
                self.total_seconds.map(|seconds| {
                    let hours = seconds / 3600.0;
                    format!("{hours:.1} hrs")
                })
            })
            .or_else(|| {
                self.extra
                    .get("total")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_else(|| "Unavailable".to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Heartbeat {
    pub project: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreakResponse {
    pub streak_days: Option<u64>,
}

impl StreakResponse {
    pub fn display(&self) -> String {
        self.streak_days
            .map(|days| format!("{days} days"))
            .unwrap_or_else(|| "Unavailable".to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectSummary {
    pub name: Option<String>,
    pub text: Option<String>,
    pub total_seconds: Option<f64>,
}

impl ProjectSummary {
    pub fn display_time(&self) -> String {
        self.text
            .clone()
            .or_else(|| {
                self.total_seconds.map(|seconds| {
                    let hours = seconds / 3600.0;
                    format!("{hours:.1} hrs")
                })
            })
            .unwrap_or_else(|| "Unavailable".to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectsResponse {
    pub projects: Vec<ProjectSummary>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserStatsResponse {
    pub data: UserStatsSummary,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserStatsSummary {
    pub languages: Option<Vec<LanguageStat>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LanguageStat {
    pub name: Option<String>,
    pub total_seconds: Option<f64>,
    pub percent: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct DashboardData {
    pub title: String,
    pub layout: DashboardLayout,
    pub stats: Vec<StatLine>,
    pub languages_title: Option<String>,
    pub languages: Vec<LanguageLine>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DashboardLayout {
    Standard,
    Fetch,
}

#[derive(Debug, Clone)]
pub struct StatLine {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct LanguageLine {
    pub name: String,
    pub percent: f64,
    pub hours_text: String,
}

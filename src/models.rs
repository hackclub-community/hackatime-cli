use chrono::NaiveDate;
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
    pub last_heartbeat: Option<String>,
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
pub struct HeartbeatSpansResponse {
    pub spans: Vec<HeartbeatSpan>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeartbeatSpan {
    #[serde(alias = "start")]
    pub start_time: Option<f64>,
    #[serde(alias = "end")]
    pub end_time: Option<f64>,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserStatsResponse {
    pub data: UserStatsSummary,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserStatsSummary {
    pub total_seconds: Option<f64>,
    pub human_readable_total: Option<String>,
    pub streak: Option<u64>,
    pub languages: Option<Vec<LanguageStat>>,
    pub projects: Option<Vec<ProjectStat>>,
}

impl UserStatsSummary {
    pub fn display_total(&self) -> String {
        self.human_readable_total
            .clone()
            .or_else(|| {
                self.total_seconds.map(|seconds| {
                    let hours = seconds / 3600.0;
                    format!("{hours:.1} hrs")
                })
            })
            .unwrap_or_else(|| "Unavailable".to_string())
    }

    pub fn display_streak(&self) -> String {
        self.streak
            .map(|days| format!("{days} days"))
            .unwrap_or_else(|| "Unavailable".to_string())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LanguageStat {
    pub name: Option<String>,
    pub total_seconds: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectStat {
    pub name: Option<String>,
    pub total_seconds: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct DashboardData {
    pub title: String,
    pub layout: DashboardLayout,
    pub stats: Vec<StatLine>,
    pub languages_title: Option<String>,
    pub languages: Vec<LanguageLine>,
    pub project_graphs_title: Option<String>,
    pub project_graphs: Vec<ProjectGraphLine>,
    pub activity_graph: Option<ActivityGraph>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DashboardLayout {
    Standard,
    Fetch,
    Projects,
    Graph,
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

#[derive(Debug, Clone)]
pub struct ProjectGraphLine {
    pub name: String,
    pub total_seconds: f64,
    pub hours_text: String,
    pub languages: Vec<ProjectLanguageSegment>,
}

#[derive(Debug, Clone)]
pub struct ProjectLanguageSegment {
    pub name: String,
    pub percent: f64,
}

#[derive(Debug, Clone)]
pub struct ActivityGraph {
    pub weeks: Vec<ActivityWeek>,
    pub today: NaiveDate,
    pub display_end: NaiveDate,
    pub total_hours_text: String,
    pub active_days: usize,
    pub best_day_text: String,
}

#[derive(Debug, Clone)]
pub struct ActivityWeek {
    pub days: Vec<ActivityDay>,
}

#[derive(Debug, Clone)]
pub struct ActivityDay {
    pub date: NaiveDate,
    pub seconds: f64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FetchTheme {
    Red,
    Blue,
    Green,
    Yellow,
    Pink,
    Cyan,
    Noir,
}

impl FetchTheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Blue => "blue",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Pink => "pink",
            Self::Cyan => "cyan",
            Self::Noir => "noir",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "red" => Some(Self::Red),
            "blue" => Some(Self::Blue),
            "green" => Some(Self::Green),
            "yellow" => Some(Self::Yellow),
            "pink" => Some(Self::Pink),
            "cyan" => Some(Self::Cyan),
            "noir" => Some(Self::Noir),
            _ => None,
        }
    }
}
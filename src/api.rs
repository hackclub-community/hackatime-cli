use anyhow::{Context, Result};
use chrono::{Datelike, Days, Local, NaiveDate};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::models::{
    DashboardData, DashboardLayout, DurationResponse, Heartbeat, LanguageLine, ProjectsResponse,
    StatLine, StreakResponse, UserProfile, UserStatsResponse,
};

const API_BASE_URL: &str = "https://hackatime.hackclub.com/api/v1";
const NOT_AVAILABLE: &str = "N/A";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportMode {
    Summary,
    Fetch,
    Current,
    Day,
    Week,
    Month,
    Year,
    Lifetime,
}

#[derive(Clone, Copy)]
enum TimeRange {
    Day,
    Week,
    Month,
    Year,
    Lifetime,
}

#[derive(Clone)]
pub struct HackatimeClient {
    client: Client,
    access_token: String,
}

impl HackatimeClient {
    pub fn new(access_token: String) -> Self {
        Self {
            client: Client::new(),
            access_token,
        }
    }

    pub async fn fetch_dashboard(&self, mode: ReportMode) -> Result<DashboardData> {
        match mode {
            ReportMode::Summary => self.fetch_summary_report().await,
            ReportMode::Fetch => self.fetch_fetch_report().await,
            ReportMode::Current => self.fetch_current_project_report().await,
            ReportMode::Day => self.fetch_range_report(TimeRange::Day).await,
            ReportMode::Week => self.fetch_range_report(TimeRange::Week).await,
            ReportMode::Month => self.fetch_range_report(TimeRange::Month).await,
            ReportMode::Year => self.fetch_range_report(TimeRange::Year).await,
            ReportMode::Lifetime => self.fetch_range_report(TimeRange::Lifetime).await,
        }
    }

    pub async fn fetch_named_project_report(&self, project_name: &str) -> Result<DashboardData> {
        self.fetch_project_report(project_name).await
    }

    async fn fetch_summary_report(&self) -> Result<DashboardData> {
        let (profile, total_today, total_week, total_month, total_year, total_lifetime) = tokio::try_join!(
            self.get::<UserProfile>("/authenticated/me"),
            self.fetch_hours_for_range(TimeRange::Day),
            self.fetch_hours_for_range(TimeRange::Week),
            self.fetch_hours_for_range(TimeRange::Month),
            self.fetch_hours_for_range(TimeRange::Year),
            self.fetch_hours_for_range(TimeRange::Lifetime),
        )?;

        let languages = if let Some(user_id) = profile.id {
            self.fetch_language_breakdown(user_id, TimeRange::Lifetime, None)
                .await?
        } else {
            Vec::new()
        };

        Ok(DashboardData {
            title: "Hackatime Stats".to_string(),
            layout: DashboardLayout::Standard,
            stats: vec![
                StatLine {
                    label: "Total Hours Today".to_string(),
                    value: total_today.display(),
                },
                StatLine {
                    label: "Total Hours This Week".to_string(),
                    value: total_week.display(),
                },
                StatLine {
                    label: "Total Hours This Month".to_string(),
                    value: total_month.display(),
                },
                StatLine {
                    label: "Total Hours This Year".to_string(),
                    value: total_year.display(),
                },
                StatLine {
                    label: "Total Hours Lifetime".to_string(),
                    value: total_lifetime.display(),
                },
            ],
            languages_title: Some("Languages Lifetime".to_string()),
            languages,
        })
    }

    async fn fetch_fetch_report(&self) -> Result<DashboardData> {
        let (
            profile,
            latest_heartbeat,
            streak,
            top_project,
            total_today,
            total_week,
            total_month,
            total_year,
            total_lifetime,
        ) = tokio::try_join!(
            self.get::<UserProfile>("/authenticated/me"),
            self.get_optional::<Heartbeat>("/authenticated/heartbeats/latest"),
            self.get_optional::<StreakResponse>("/authenticated/streak"),
            self.fetch_top_project(),
            self.fetch_hours_for_range(TimeRange::Day),
            self.fetch_hours_for_range(TimeRange::Week),
            self.fetch_hours_for_range(TimeRange::Month),
            self.fetch_hours_for_range(TimeRange::Year),
            self.fetch_hours_for_range(TimeRange::Lifetime),
        )?;

        let languages = if let Some(user_id) = profile.id {
            self.fetch_language_breakdown(user_id, TimeRange::Lifetime, None)
                .await?
        } else {
            Vec::new()
        };

        let current_project = latest_heartbeat
            .and_then(|heartbeat| heartbeat.project)
            .unwrap_or_else(|| "Unavailable".to_string());
        let current_streak = streak
            .map(|streak| streak.display())
            .unwrap_or_else(|| "Unavailable".to_string());
        let fetch_title = profile.display_name();

        let stats = vec![
            StatLine {
                label: "Current Project".to_string(),
                value: current_project,
            },
            StatLine {
                label: "Current Streak".to_string(),
                value: current_streak,
            },
            StatLine {
                label: "Top Project".to_string(),
                value: top_project,
            },
            StatLine {
                label: "Today".to_string(),
                value: total_today.display(),
            },
            StatLine {
                label: "This Week".to_string(),
                value: total_week.display(),
            },
            StatLine {
                label: "This Month".to_string(),
                value: total_month.display(),
            },
            StatLine {
                label: "This Year".to_string(),
                value: total_year.display(),
            },
            StatLine {
                label: "Lifetime".to_string(),
                value: total_lifetime.display(),
            },
        ];

        Ok(DashboardData {
            title: fetch_title,
            layout: DashboardLayout::Fetch,
            stats,
            languages_title: Some("Languages Lifetime".to_string()),
            languages,
        })
    }

    async fn fetch_range_report(&self, range: TimeRange) -> Result<DashboardData> {
        let (profile, total) = tokio::try_join!(
            self.get::<UserProfile>("/authenticated/me"),
            self.fetch_hours_for_range(range),
        )?;

        let languages = if let Some(user_id) = profile.id {
            self.fetch_language_breakdown(user_id, range, None).await?
        } else {
            Vec::new()
        };

        Ok(DashboardData {
            title: "Hackatime Stats".to_string(),
            layout: DashboardLayout::Standard,
            stats: vec![StatLine {
                label: range.total_label().to_string(),
                value: total.display(),
            }],
            languages_title: Some(range.languages_label().to_string()),
            languages,
        })
    }

    async fn fetch_current_project_report(&self) -> Result<DashboardData> {
        let latest_heartbeat = self
            .get_optional::<Heartbeat>("/authenticated/heartbeats/latest")
            .await?;

        let Some(project_name) = latest_heartbeat.and_then(|heartbeat| heartbeat.project) else {
            return Ok(DashboardData {
                title: "Hackatime Stats".to_string(),
                layout: DashboardLayout::Standard,
                stats: vec![StatLine {
                    label: "Current Project".to_string(),
                    value: "Unavailable".to_string(),
                }],
                languages_title: None,
                languages: Vec::new(),
            });
        };

        self.fetch_project_report(&project_name).await
    }

    async fn fetch_hours_for_range(&self, range: TimeRange) -> Result<DurationResponse> {
        let (start, end) = range.date_bounds()?;
        self.get_with_query(
            "/authenticated/hours",
            &[("start_date", start), ("end_date", end)],
        )
        .await
    }

    async fn fetch_language_breakdown(
        &self,
        user_id: u64,
        range: TimeRange,
        project_filter: Option<&str>,
    ) -> Result<Vec<LanguageLine>> {
        let (start, mut end) = range.date_bounds()?;
        let mut params = vec![
            ("features", "languages".to_string()),
            ("start_date", start),
            ("end_date", end.clone()),
            ("limit", "8".to_string()),
        ];
        if matches!(range, TimeRange::Day) {
            end = tomorrow_date_string()?;
            params[2] = ("end_date", end);
            params.push(("boundary_aware", "true".to_string()));
        }
        if let Some(project_name) = project_filter {
            params.push(("filter_by_project", project_name.to_string()));
        }

        let response = self
            .get_with_query::<UserStatsResponse>(&format!("/users/{user_id}/stats"), &params)
            .await?;

        Ok(response
            .data
            .languages
            .unwrap_or_default()
            .into_iter()
            .filter_map(|language| {
                let name = language.name?;
                let total_seconds = language.total_seconds.unwrap_or(0.0);
                if total_seconds <= 0.0 {
                    return None;
                }
                let percent = language.percent.unwrap_or(0.0);
                Some(LanguageLine {
                    name,
                    percent,
                    hours_text: format!("{:.1} hrs", total_seconds / 3600.0),
                })
            })
            .collect())
    }

    async fn fetch_project_total(&self, project_name: &str) -> Result<String> {
        Ok(self
            .fetch_project_summary(project_name, None)
            .await?
            .map(|project| project.display_time())
            .unwrap_or_else(|| NOT_AVAILABLE.to_string()))
    }

    async fn fetch_project_total_for_range(
        &self,
        project_name: &str,
        range: TimeRange,
    ) -> Result<String> {
        let project = self
            .fetch_project_summary(project_name, Some(range))
            .await?;
        Ok(project
            .map(|project| project.display_time())
            .unwrap_or_else(|| NOT_AVAILABLE.to_string()))
    }

    async fn fetch_top_project(&self) -> Result<String> {
        let response = self
            .get_with_query::<ProjectsResponse>("/authenticated/projects", &[])
            .await?;

        Ok(response
            .projects
            .into_iter()
            .filter_map(|project| {
                let total_seconds = project.total_seconds?;
                if total_seconds <= 0.0 {
                    return None;
                }

                let time = project.display_time();
                let name = project.name?;
                Some((total_seconds, name, time))
            })
            .max_by(|left, right| left.0.total_cmp(&right.0))
            .map(|(_, name, time)| format!("{name} ({time})"))
            .unwrap_or_else(|| NOT_AVAILABLE.to_string()))
    }

    async fn fetch_project_report(&self, project_name: &str) -> Result<DashboardData> {
        let profile = self.get::<UserProfile>("/authenticated/me").await?;
        let (project_total, project_today, languages) = tokio::try_join!(
            self.fetch_project_total(project_name),
            self.fetch_project_total_for_range(project_name, TimeRange::Day),
            async {
                if let Some(user_id) = profile.id {
                    self.fetch_language_breakdown(user_id, TimeRange::Lifetime, Some(project_name))
                        .await
                } else {
                    Ok(Vec::new())
                }
            }
        )?;

        let project_exists = project_total != NOT_AVAILABLE || project_today != NOT_AVAILABLE;
        let stats = if project_exists {
            vec![
                StatLine {
                    label: "Project".to_string(),
                    value: project_name.to_string(),
                },
                StatLine {
                    label: "Total Hours On Project".to_string(),
                    value: project_total,
                },
                StatLine {
                    label: "Hours On Project Today".to_string(),
                    value: project_today,
                },
            ]
        } else {
            vec![
                StatLine {
                    label: "Project".to_string(),
                    value: project_name.to_string(),
                },
                StatLine {
                    label: "Status".to_string(),
                    value: "Not found in your Hackatime projects".to_string(),
                },
            ]
        };

        Ok(DashboardData {
            title: "Hackatime Stats".to_string(),
            layout: DashboardLayout::Standard,
            stats,
            languages_title: if project_exists {
                Some("Languages In Project".to_string())
            } else {
                None
            },
            languages: if project_exists {
                languages
            } else {
                Vec::new()
            },
        })
    }

    async fn fetch_project_summary(
        &self,
        project_name: &str,
        range: Option<TimeRange>,
    ) -> Result<Option<crate::models::ProjectSummary>> {
        let (start, mut end) = match range {
            Some(range) => {
                let (start, end) = range.date_bounds()?;
                (Some(start), Some(end))
            }
            None => (None, None),
        };

        if matches!(range, Some(TimeRange::Day)) {
            end = Some(tomorrow_date_string()?);
        }

        let mut params = vec![("projects", project_name.to_string())];
        if let Some(start) = start {
            params.push(("start_date", start));
        }
        if let Some(end) = end {
            params.push(("end_date", end));
        }

        let response = self
            .get_with_query::<ProjectsResponse>("/authenticated/projects", &params)
            .await?;

        Ok(response
            .projects
            .into_iter()
            .find(|project| project.name.as_deref() == Some(project_name)))
    }

    async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.get_with_query(path, &[]).await
    }

    async fn get_optional<T>(&self, path: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let url = format!("{API_BASE_URL}{path}");
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .with_context(|| format!("request failed for {url}"))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let response = response
            .error_for_status()
            .with_context(|| format!("Hackatime returned an error for {url}"))?;

        let value = response
            .json::<Value>()
            .await
            .with_context(|| format!("failed to decode response from {url}"))?;

        Ok(Some(deserialize_api_value(value, &url)?))
    }

    async fn get_with_query<T>(&self, path: &str, params: &[(&str, String)]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!("{API_BASE_URL}{path}");
        let response = self
            .client
            .get(&url)
            .query(params)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .with_context(|| format!("request failed for {url}"))?
            .error_for_status()
            .with_context(|| format!("Hackatime returned an error for {url}"))?;

        let value = response
            .json::<Value>()
            .await
            .with_context(|| format!("failed to decode response from {url}"))?;

        deserialize_api_value(value, &url)
    }
}

fn deserialize_api_value<T>(value: Value, url: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let direct_attempt = serde_json::from_value::<T>(value.clone());
    if let Ok(parsed) = direct_attempt {
        return Ok(parsed);
    }

    if let Some(data) = value.get("data") {
        return serde_json::from_value::<T>(data.clone())
            .with_context(|| format!("failed to decode wrapped response from {url}"));
    }

    serde_json::from_value::<T>(value)
        .with_context(|| format!("failed to decode response payload from {url}"))
}

fn start_of_week(today: NaiveDate) -> NaiveDate {
    let days_from_monday = today.weekday().num_days_from_monday();
    today
        .checked_sub_days(Days::new(days_from_monday.into()))
        .unwrap_or(today)
}

fn tomorrow_date_string() -> Result<String> {
    let tomorrow = Local::now()
        .date_naive()
        .checked_add_days(Days::new(1))
        .context("failed to build tomorrow date")?;
    Ok(tomorrow.to_string())
}

impl TimeRange {
    fn date_bounds(self) -> Result<(String, String)> {
        let today = Local::now().date_naive();
        let start = match self {
            TimeRange::Day => today,
            TimeRange::Week => start_of_week(today),
            TimeRange::Month => NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                .context("failed to build month start")?,
            TimeRange::Year => {
                NaiveDate::from_ymd_opt(today.year(), 1, 1).context("failed to build year start")?
            }
            TimeRange::Lifetime => {
                NaiveDate::from_ymd_opt(1970, 1, 1).context("failed to build lifetime start")?
            }
        };

        Ok((start.to_string(), today.to_string()))
    }

    fn total_label(self) -> &'static str {
        match self {
            TimeRange::Day => "Total Hours Today",
            TimeRange::Week => "Total Hours This Week",
            TimeRange::Month => "Total Hours This Month",
            TimeRange::Year => "Total Hours This Year",
            TimeRange::Lifetime => "Total Hours Lifetime",
        }
    }

    fn languages_label(self) -> &'static str {
        match self {
            TimeRange::Day => "Languages Today",
            TimeRange::Week => "Languages This Week",
            TimeRange::Month => "Languages This Month",
            TimeRange::Year => "Languages This Year",
            TimeRange::Lifetime => "Languages Lifetime",
        }
    }
}

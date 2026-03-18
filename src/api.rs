use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Days, Local, NaiveDate, TimeZone, Utc, Weekday};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::time::{Duration, timeout};

use crate::models::{
    ActivityDay, ActivityGraph, ActivityWeek, DashboardData, DashboardLayout, DurationResponse,
    Heartbeat, HeartbeatSpan, HeartbeatSpansResponse, LanguageLine, ProjectGraphLine,
    ProjectLanguageSegment, ProjectsResponse, StatLine, StreakResponse, UserProfile,
    UserStatsResponse, UserStatsSummary,
};

const API_BASE_URL: &str = "https://hackatime.hackclub.com/api/v1";
const NOT_AVAILABLE: &str = "N/A";
const LAST_PROJECT_PLACEHOLDER: &str = "<<LAST_PROJECT>>";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportMode {
    Summary,
    Fetch,
    Graph,
    Projects,
    ProjectsWeek,
    ProjectsMonth,
    ProjectsYear,
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
            ReportMode::Graph => self.fetch_activity_graph_report().await,
            ReportMode::Projects => self.fetch_projects_report(TimeRange::Lifetime).await,
            ReportMode::ProjectsWeek => self.fetch_projects_report(TimeRange::Week).await,
            ReportMode::ProjectsMonth => self.fetch_projects_report(TimeRange::Month).await,
            ReportMode::ProjectsYear => self.fetch_projects_report(TimeRange::Year).await,
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

    pub async fn fetch_lookup_dashboard(
        &self,
        mode: ReportMode,
        username: &str,
    ) -> Result<DashboardData> {
        match mode {
            ReportMode::Summary => self.fetch_lookup_summary_report(username).await,
            ReportMode::Fetch => self.fetch_lookup_fetch_report(username).await,
            ReportMode::Graph => self.fetch_lookup_activity_graph_report(username).await,
            ReportMode::Projects => {
                self.fetch_lookup_projects_report(username, TimeRange::Lifetime)
                    .await
            }
            ReportMode::ProjectsWeek => {
                self.fetch_lookup_projects_report(username, TimeRange::Week)
                    .await
            }
            ReportMode::ProjectsMonth => {
                self.fetch_lookup_projects_report(username, TimeRange::Month)
                    .await
            }
            ReportMode::ProjectsYear => {
                self.fetch_lookup_projects_report(username, TimeRange::Year)
                    .await
            }
            ReportMode::Current => self.fetch_lookup_current_project_report(username).await,
            ReportMode::Day => {
                self.fetch_lookup_range_report(username, TimeRange::Day)
                    .await
            }
            ReportMode::Week => {
                self.fetch_lookup_range_report(username, TimeRange::Week)
                    .await
            }
            ReportMode::Month => {
                self.fetch_lookup_range_report(username, TimeRange::Month)
                    .await
            }
            ReportMode::Year => {
                self.fetch_lookup_range_report(username, TimeRange::Year)
                    .await
            }
            ReportMode::Lifetime => {
                self.fetch_lookup_range_report(username, TimeRange::Lifetime)
                    .await
            }
        }
    }

    pub async fn fetch_lookup_named_project_report(
        &self,
        username: &str,
        project_name: &str,
    ) -> Result<DashboardData> {
        self.fetch_lookup_project_report(username, project_name)
            .await
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

        let user_ref = user_ref_from_profile(&profile)?;
        let languages = self
            .fetch_language_breakdown(&user_ref, TimeRange::Lifetime, None)
            .await?;

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
            project_graphs_title: None,
            project_graphs: Vec::new(),
            activity_graph: None,
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
            self.fetch_language_breakdown(&user_id.to_string(), TimeRange::Lifetime, None)
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
            project_graphs_title: None,
            project_graphs: Vec::new(),
            activity_graph: None,
        })
    }

    async fn fetch_activity_graph_report(&self) -> Result<DashboardData> {
        let profile = self.get::<UserProfile>("/authenticated/me").await?;
        let user_ref = user_ref_from_profile(&profile)?;
        let graph = self.fetch_activity_graph(&user_ref).await?;

        Ok(DashboardData {
            title: "Hackatime Graph".to_string(),
            layout: DashboardLayout::Graph,
            stats: vec![
                StatLine {
                    label: "Total Hours Last 365 Days".to_string(),
                    value: graph.total_hours_text.clone(),
                },
                StatLine {
                    label: "Active Days".to_string(),
                    value: graph.active_days.to_string(),
                },
                StatLine {
                    label: "Best Day".to_string(),
                    value: graph.best_day_text.clone(),
                },
            ],
            languages_title: None,
            languages: Vec::new(),
            project_graphs_title: None,
            project_graphs: Vec::new(),
            activity_graph: Some(graph),
        })
    }

    async fn fetch_lookup_summary_report(&self, username: &str) -> Result<DashboardData> {
        let (total_today, total_week, total_month, total_year, total_lifetime, languages) = tokio::try_join!(
            self.fetch_public_stats_summary(username, TimeRange::Day, None, None),
            self.fetch_public_stats_summary(username, TimeRange::Week, None, None),
            self.fetch_public_stats_summary(username, TimeRange::Month, None, None),
            self.fetch_public_stats_summary(username, TimeRange::Year, None, None),
            self.fetch_public_stats_summary(username, TimeRange::Lifetime, None, None),
            self.fetch_language_breakdown(username, TimeRange::Lifetime, None),
        )?;

        Ok(DashboardData {
            title: lookup_stats_title(username),
            layout: DashboardLayout::Standard,
            stats: vec![
                StatLine {
                    label: "Total Hours Today".to_string(),
                    value: total_today.display_total(),
                },
                StatLine {
                    label: "Total Hours This Week".to_string(),
                    value: total_week.display_total(),
                },
                StatLine {
                    label: "Total Hours This Month".to_string(),
                    value: total_month.display_total(),
                },
                StatLine {
                    label: "Total Hours This Year".to_string(),
                    value: total_year.display_total(),
                },
                StatLine {
                    label: "Total Hours Lifetime".to_string(),
                    value: total_lifetime.display_total(),
                },
            ],
            languages_title: Some("Languages Lifetime".to_string()),
            languages,
            project_graphs_title: None,
            project_graphs: Vec::new(),
            activity_graph: None,
        })
    }

    async fn fetch_lookup_fetch_report(&self, username: &str) -> Result<DashboardData> {
        let (
            projects,
            streak,
            total_today,
            total_week,
            total_month,
            total_year,
            total_lifetime,
            languages,
        ) = tokio::try_join!(
            self.fetch_public_projects_details(username, TimeRange::Lifetime),
            self.fetch_public_stats_summary(username, TimeRange::Lifetime, None, None),
            self.fetch_public_stats_summary(username, TimeRange::Day, None, None),
            self.fetch_public_stats_summary(username, TimeRange::Week, None, None),
            self.fetch_public_stats_summary(username, TimeRange::Month, None, None),
            self.fetch_public_stats_summary(username, TimeRange::Year, None, None),
            self.fetch_public_stats_summary(username, TimeRange::Lifetime, None, None),
            self.fetch_language_breakdown(username, TimeRange::Lifetime, None),
        )?;

        let current_project = projects
            .iter()
            .filter_map(|project| {
                project
                    .last_heartbeat
                    .as_ref()
                    .map(|last_heartbeat| (last_heartbeat.as_str(), project.name.as_str()))
            })
            .max_by(|left, right| left.0.cmp(right.0))
            .map(|(_, name)| name.to_string())
            .unwrap_or_else(|| NOT_AVAILABLE.to_string());

        let top_project = projects
            .first()
            .map(|project| format!("{} ({})", project.name, project.hours_text))
            .unwrap_or_else(|| NOT_AVAILABLE.to_string());

        Ok(DashboardData {
            title: username.to_string(),
            layout: DashboardLayout::Fetch,
            stats: vec![
                StatLine {
                    label: "Current Project".to_string(),
                    value: current_project,
                },
                StatLine {
                    label: "Current Streak".to_string(),
                    value: streak.display_streak(),
                },
                StatLine {
                    label: "Top Project".to_string(),
                    value: top_project,
                },
                StatLine {
                    label: "Today".to_string(),
                    value: total_today.display_total(),
                },
                StatLine {
                    label: "This Week".to_string(),
                    value: total_week.display_total(),
                },
                StatLine {
                    label: "This Month".to_string(),
                    value: total_month.display_total(),
                },
                StatLine {
                    label: "This Year".to_string(),
                    value: total_year.display_total(),
                },
                StatLine {
                    label: "Lifetime".to_string(),
                    value: total_lifetime.display_total(),
                },
            ],
            languages_title: Some("Languages Lifetime".to_string()),
            languages,
            project_graphs_title: None,
            project_graphs: Vec::new(),
            activity_graph: None,
        })
    }

    async fn fetch_lookup_activity_graph_report(&self, username: &str) -> Result<DashboardData> {
        let graph = self.fetch_activity_graph(username).await?;
        Ok(DashboardData {
            title: format!("{username} Graph"),
            layout: DashboardLayout::Graph,
            stats: Vec::new(),
            languages_title: None,
            languages: Vec::new(),
            project_graphs_title: None,
            project_graphs: Vec::new(),
            activity_graph: Some(graph),
        })
    }

    async fn fetch_lookup_projects_report(
        &self,
        username: &str,
        range: TimeRange,
    ) -> Result<DashboardData> {
        let (total_lifetime, projects) = tokio::try_join!(
            self.fetch_public_stats_summary(username, range, None, None),
            self.fetch_public_project_breakdown_from_stats(username, range),
        )?;

        let tracked_projects = projects.len();
        let mut project_graphs = Vec::new();
        for project in projects.into_iter().take(10) {
            let name = project.name;
            let total_seconds = project.total_seconds;
            let hours_text = project.hours_text;
            let languages = match timeout(
                Duration::from_secs(5),
                self.fetch_language_breakdown(username, range, Some(&name)),
            )
            .await
            {
                Ok(Ok(languages)) => languages
                    .into_iter()
                    .map(|language| ProjectLanguageSegment {
                        name: language.name,
                        percent: language.percent,
                    })
                    .collect(),
                Ok(Err(_)) | Err(_) => Vec::new(),
            };

            project_graphs.push(ProjectGraphLine {
                name,
                total_seconds,
                hours_text,
                languages,
            });
        }

        let mut stats = vec![
            StatLine {
                label: "Tracked Projects".to_string(),
                value: tracked_projects.to_string(),
            },
            StatLine {
                label: range.total_label().to_string(),
                value: total_lifetime.display_total(),
            },
        ];

        let project_graphs_title = if project_graphs.is_empty() {
            stats.push(StatLine {
                label: "Status".to_string(),
                value: format!("No public projects found for {username}"),
            });
            None
        } else {
            Some(range.projects_title().to_string())
        };

        Ok(DashboardData {
            title: lookup_projects_title(username),
            layout: DashboardLayout::Projects,
            stats,
            languages_title: None,
            languages: Vec::new(),
            project_graphs_title,
            project_graphs,
            activity_graph: None,
        })
    }

    async fn fetch_public_project_breakdown_from_stats(
        &self,
        user_ref: &str,
        range: TimeRange,
    ) -> Result<Vec<ProjectBreakdown>> {
        let summary = self
            .fetch_public_stats_summary(user_ref, range, Some("projects"), None)
            .await?;

        let mut projects = summary
            .projects
            .unwrap_or_default()
            .into_iter()
            .filter_map(|project| {
                let total_seconds = project.total_seconds?;
                if total_seconds <= 0.0 {
                    return None;
                }

                let name = project.name?;
                if name == LAST_PROJECT_PLACEHOLDER {
                    return None;
                }

                Some(ProjectBreakdown {
                    name,
                    total_seconds,
                    hours_text: format!("{:.1} hrs", total_seconds / 3600.0),
                    last_heartbeat: None,
                })
            })
            .collect::<Vec<_>>();

        projects.sort_by(|left, right| right.total_seconds.total_cmp(&left.total_seconds));
        Ok(projects)
    }

    async fn fetch_lookup_range_report(
        &self,
        username: &str,
        range: TimeRange,
    ) -> Result<DashboardData> {
        let (total, languages) = tokio::try_join!(
            self.fetch_public_stats_summary(username, range, None, None),
            self.fetch_language_breakdown(username, range, None),
        )?;

        Ok(DashboardData {
            title: lookup_stats_title(username),
            layout: DashboardLayout::Standard,
            stats: vec![StatLine {
                label: range.total_label().to_string(),
                value: total.display_total(),
            }],
            languages_title: Some(range.languages_label().to_string()),
            languages,
            project_graphs_title: None,
            project_graphs: Vec::new(),
            activity_graph: None,
        })
    }

    async fn fetch_lookup_current_project_report(&self, username: &str) -> Result<DashboardData> {
        let projects = self
            .fetch_public_projects_details(username, TimeRange::Lifetime)
            .await?;
        let current_project = projects
            .iter()
            .filter_map(|project| {
                project
                    .last_heartbeat
                    .as_ref()
                    .map(|last_heartbeat| (last_heartbeat.as_str(), project.name.as_str()))
            })
            .max_by(|left, right| left.0.cmp(right.0))
            .map(|(_, name)| name.to_string());

        let Some(project_name) = current_project else {
            return Ok(DashboardData {
                title: lookup_stats_title(username),
                layout: DashboardLayout::Standard,
                stats: vec![StatLine {
                    label: "Current Project".to_string(),
                    value: NOT_AVAILABLE.to_string(),
                }],
                languages_title: None,
                languages: Vec::new(),
                project_graphs_title: None,
                project_graphs: Vec::new(),
                activity_graph: None,
            });
        };

        self.fetch_lookup_project_report(username, &project_name)
            .await
    }

    async fn fetch_projects_report(&self, range: TimeRange) -> Result<DashboardData> {
        let (profile, total_lifetime, projects) = tokio::try_join!(
            self.get::<UserProfile>("/authenticated/me"),
            self.fetch_hours_for_range(range),
            self.fetch_ranked_projects(range),
        )?;

        let tracked_projects = projects.len();
        let user_id = profile.id;
        let mut project_graphs = Vec::new();

        for project in projects.into_iter().take(10) {
            let name = project.name;
            let total_seconds = project.total_seconds;
            let hours_text = project.hours_text;
            let languages = if let Some(user_id) = user_id {
                match timeout(
                    Duration::from_secs(5),
                    self.fetch_language_breakdown(&user_id.to_string(), range, Some(&name)),
                )
                .await
                {
                    Ok(Ok(languages)) => languages
                        .into_iter()
                        .map(|language| ProjectLanguageSegment {
                            name: language.name,
                            percent: language.percent,
                        })
                        .collect(),
                    Ok(Err(_)) | Err(_) => Vec::new(),
                }
            } else {
                Vec::new()
            };

            project_graphs.push(ProjectGraphLine {
                name,
                total_seconds,
                hours_text,
                languages,
            });
        }

        let mut stats = vec![
            StatLine {
                label: "Tracked Projects".to_string(),
                value: tracked_projects.to_string(),
            },
            StatLine {
                label: range.total_label().to_string(),
                value: total_lifetime.display(),
            },
        ];

        let project_graphs_title = if project_graphs.is_empty() {
            stats.push(StatLine {
                label: "Status".to_string(),
                value: "No projects found in your Hackatime data".to_string(),
            });
            None
        } else {
            Some(range.projects_title().to_string())
        };

        Ok(DashboardData {
            title: "Hackatime Projects".to_string(),
            layout: DashboardLayout::Projects,
            stats,
            languages_title: None,
            languages: Vec::new(),
            project_graphs_title,
            project_graphs,
            activity_graph: None,
        })
    }

    async fn fetch_range_report(&self, range: TimeRange) -> Result<DashboardData> {
        let (profile, total) = tokio::try_join!(
            self.get::<UserProfile>("/authenticated/me"),
            self.fetch_hours_for_range(range),
        )?;

        let user_ref = user_ref_from_profile(&profile)?;
        let languages = self
            .fetch_language_breakdown(&user_ref, range, None)
            .await?;

        Ok(DashboardData {
            title: "Hackatime Stats".to_string(),
            layout: DashboardLayout::Standard,
            stats: vec![StatLine {
                label: range.total_label().to_string(),
                value: total.display(),
            }],
            languages_title: Some(range.languages_label().to_string()),
            languages,
            project_graphs_title: None,
            project_graphs: Vec::new(),
            activity_graph: None,
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
                project_graphs_title: None,
                project_graphs: Vec::new(),
                activity_graph: None,
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
        user_ref: &str,
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
            .get_with_query::<UserStatsResponse>(&format!("/users/{user_ref}/stats"), &params)
            .await?;

        let mut languages = response
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
                Some((name, total_seconds))
            })
            .collect::<Vec<_>>();

        let total_seconds = languages
            .iter()
            .map(|(_, total_seconds)| *total_seconds)
            .sum::<f64>();

        Ok(languages
            .drain(..)
            .map(|(name, language_seconds)| LanguageLine {
                name,
                percent: if total_seconds > 0.0 {
                    (language_seconds / total_seconds) * 100.0
                } else {
                    0.0
                },
                hours_text: format!("{:.1} hrs", language_seconds / 3600.0),
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
        Ok(self
            .fetch_ranked_projects(TimeRange::Lifetime)
            .await?
            .into_iter()
            .next()
            .map(|project| format!("{} ({})", project.name, project.hours_text))
            .unwrap_or_else(|| NOT_AVAILABLE.to_string()))
    }

    async fn fetch_ranked_projects(&self, range: TimeRange) -> Result<Vec<ProjectBreakdown>> {
        let params = if matches!(range, TimeRange::Lifetime) {
            Vec::new()
        } else {
            let (start, end) = range.date_bounds()?;
            vec![("start_date", start), ("end_date", end)]
        };
        let response = self
            .get_with_query::<ProjectsResponse>("/authenticated/projects", &params)
            .await?;

        let mut projects = response
            .projects
            .into_iter()
            .filter_map(|project| {
                let total_seconds = project.total_seconds?;
                if total_seconds <= 0.0 {
                    return None;
                }

                let hours_text = project.display_time();
                let name = project.name?;
                if name == LAST_PROJECT_PLACEHOLDER {
                    return None;
                }

                Some(ProjectBreakdown {
                    name,
                    total_seconds,
                    hours_text,
                    last_heartbeat: None,
                })
            })
            .collect::<Vec<_>>();

        projects.sort_by(|left, right| right.total_seconds.total_cmp(&left.total_seconds));
        Ok(projects)
    }

    async fn fetch_project_report(&self, project_name: &str) -> Result<DashboardData> {
        let profile = self.get::<UserProfile>("/authenticated/me").await?;
        let (project_total, project_today, languages) = tokio::try_join!(
            self.fetch_project_total(project_name),
            self.fetch_project_total_for_range(project_name, TimeRange::Day),
            async {
                if let Some(user_id) = profile.id {
                    self.fetch_language_breakdown(
                        &user_id.to_string(),
                        TimeRange::Lifetime,
                        Some(project_name),
                    )
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
            project_graphs_title: None,
            project_graphs: Vec::new(),
            activity_graph: None,
        })
    }

    async fn fetch_lookup_project_report(
        &self,
        username: &str,
        project_name: &str,
    ) -> Result<DashboardData> {
        let (project_total, project_today, languages) = tokio::try_join!(
            self.fetch_public_project_total(username, project_name),
            self.fetch_public_project_total_for_range(username, project_name, TimeRange::Day),
            self.fetch_language_breakdown(username, TimeRange::Lifetime, Some(project_name)),
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
                    value: format!("Not found in {username}'s public Hackatime projects"),
                },
            ]
        };

        Ok(DashboardData {
            title: lookup_stats_title(username),
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
            project_graphs_title: None,
            project_graphs: Vec::new(),
            activity_graph: None,
        })
    }

    async fn fetch_activity_graph(&self, user_ref: &str) -> Result<ActivityGraph> {
        let today = Local::now().date_naive();
        let graph_end = end_of_month(today)?;
        let last_365_start = today
            .checked_sub_days(Days::new(364))
            .context("failed to build graph start date")?;
        let display_start = align_to_week_start(last_365_start);

        let response = self
            .get_with_query::<HeartbeatSpansResponse>(
                &format!("/users/{user_ref}/heartbeats/spans"),
                &[
                    ("start_date", display_start.to_string()),
                    ("end_date", today.to_string()),
                ],
            )
            .await?;

        let mut day_seconds = std::collections::HashMap::<NaiveDate, f64>::new();
        for span in response.spans {
            add_span_duration(&mut day_seconds, span);
        }

        let mut weeks = Vec::new();
        let mut cursor = display_start;
        while cursor <= graph_end {
            let mut days = Vec::with_capacity(7);
            for _ in 0..7 {
                days.push(ActivityDay {
                    date: cursor,
                    seconds: day_seconds.get(&cursor).copied().unwrap_or(0.0),
                });
                cursor = cursor
                    .checked_add_days(Days::new(1))
                    .context("failed to advance graph cursor")?;
            }
            weeks.push(ActivityWeek { days });
        }

        let mut total_seconds = 0.0_f64;
        let mut active_days = 0_usize;
        let mut best_day: Option<(NaiveDate, f64)> = None;
        for week in &weeks {
            for day in &week.days {
                if day.date < last_365_start || day.date > today {
                    continue;
                }
                total_seconds += day.seconds;
                if day.seconds > 0.0 {
                    active_days += 1;
                }
                if best_day
                    .map(|(_, best_seconds)| day.seconds > best_seconds)
                    .unwrap_or(day.seconds > 0.0)
                {
                    best_day = Some((day.date, day.seconds));
                }
            }
        }

        Ok(ActivityGraph {
            weeks,
            today,
            display_end: graph_end,
            total_hours_text: format!("{:.1} hrs", total_seconds / 3600.0),
            active_days,
            best_day_text: best_day
                .map(|(date, seconds)| format!("{date} ({:.1} hrs)", seconds / 3600.0))
                .unwrap_or_else(|| NOT_AVAILABLE.to_string()),
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

    async fn fetch_public_stats_summary(
        &self,
        user_ref: &str,
        range: TimeRange,
        features: Option<&str>,
        project_filter: Option<&str>,
    ) -> Result<UserStatsSummary> {
        let (start, mut end) = range.date_bounds()?;
        let mut params = vec![("start_date", start), ("end_date", end.clone())];
        if matches!(range, TimeRange::Day) {
            end = tomorrow_date_string()?;
            params[1] = ("end_date", end);
            params.push(("boundary_aware", "true".to_string()));
        }
        if let Some(features) = features {
            params.push(("features", features.to_string()));
        }
        if let Some(project_name) = project_filter {
            params.push(("filter_by_project", project_name.to_string()));
        }

        self.get_with_query::<UserStatsResponse>(&format!("/users/{user_ref}/stats"), &params)
            .await
            .map(|response| response.data)
    }

    async fn fetch_public_projects_details(
        &self,
        user_ref: &str,
        range: TimeRange,
    ) -> Result<Vec<ProjectBreakdown>> {
        let (start, end) = range.date_bounds()?;
        let response = self
            .get_with_query::<ProjectsResponse>(
                &format!("/users/{user_ref}/projects/details"),
                &[("start_date", start), ("end_date", end)],
            )
            .await?;

        let mut projects = response
            .projects
            .into_iter()
            .filter_map(|project| {
                let total_seconds = project.total_seconds?;
                if total_seconds <= 0.0 {
                    return None;
                }
                let hours_text = project.display_time();
                let name = project.name?;
                if name == LAST_PROJECT_PLACEHOLDER {
                    return None;
                }
                Some(ProjectBreakdown {
                    name,
                    total_seconds,
                    hours_text,
                    last_heartbeat: project.last_heartbeat,
                })
            })
            .collect::<Vec<_>>();

        projects.sort_by(|left, right| right.total_seconds.total_cmp(&left.total_seconds));
        Ok(projects)
    }

    async fn fetch_public_project_total(
        &self,
        user_ref: &str,
        project_name: &str,
    ) -> Result<String> {
        Ok(self
            .fetch_public_project_summary(user_ref, project_name, None)
            .await?
            .map(|project| project.display_time())
            .unwrap_or_else(|| NOT_AVAILABLE.to_string()))
    }

    async fn fetch_public_project_total_for_range(
        &self,
        user_ref: &str,
        project_name: &str,
        range: TimeRange,
    ) -> Result<String> {
        Ok(self
            .fetch_public_project_summary(user_ref, project_name, Some(range))
            .await?
            .map(|project| project.display_time())
            .unwrap_or_else(|| NOT_AVAILABLE.to_string()))
    }

    async fn fetch_public_project_summary(
        &self,
        user_ref: &str,
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
            .get_with_query::<ProjectsResponse>(
                &format!("/users/{user_ref}/projects/details"),
                &params,
            )
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

fn align_to_week_start(date: NaiveDate) -> NaiveDate {
    let days_from_monday = match date.weekday() {
        Weekday::Mon => 0,
        weekday => weekday.num_days_from_monday(),
    };

    date.checked_sub_days(Days::new(days_from_monday.into()))
        .unwrap_or(date)
}

fn end_of_month(date: NaiveDate) -> Result<NaiveDate> {
    let first_of_next_month = if date.month() == 12 {
        NaiveDate::from_ymd_opt(date.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1)
    }
    .context("failed to build next month date")?;

    first_of_next_month
        .checked_sub_days(Days::new(1))
        .context("failed to build month end")
}

fn add_span_duration(
    day_seconds: &mut std::collections::HashMap<NaiveDate, f64>,
    span: HeartbeatSpan,
) {
    let start = span.start_time.unwrap_or(0.0);
    let duration = span
        .duration
        .or_else(|| span.end_time.map(|end| (end - start).max(0.0)))
        .unwrap_or(0.0);

    if duration <= 0.0 {
        return;
    }

    let Some(timestamp) = timestamp_to_local(start) else {
        return;
    };
    let date = timestamp.date_naive();
    *day_seconds.entry(date).or_insert(0.0) += duration;
}

fn timestamp_to_local(timestamp: f64) -> Option<DateTime<Local>> {
    let seconds = timestamp.trunc() as i64;
    let nanos = ((timestamp.fract() * 1_000_000_000.0).round() as u32).min(999_999_999);
    let utc = Utc.timestamp_opt(seconds, nanos).single()?;
    Some(utc.with_timezone(&Local))
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

    fn projects_title(self) -> &'static str {
        match self {
            TimeRange::Day => "Top Projects Today",
            TimeRange::Week => "Top Projects This Week",
            TimeRange::Month => "Top Projects This Month",
            TimeRange::Year => "Top Projects This Year",
            TimeRange::Lifetime => "Top Projects",
        }
    }
}

struct ProjectBreakdown {
    name: String,
    total_seconds: f64,
    hours_text: String,
    last_heartbeat: Option<String>,
}

fn user_ref_from_profile(profile: &UserProfile) -> Result<String> {
    profile
        .id
        .map(|id| id.to_string())
        .or_else(|| profile.github_username.clone())
        .or_else(|| {
            profile
                .emails
                .as_ref()
                .and_then(|emails| emails.first().cloned())
        })
        .context("Hackatime user reference unavailable")
}

fn lookup_stats_title(username: &str) -> String {
    format!("{username}'s Hackatime Stats")
}

fn lookup_projects_title(username: &str) -> String {
    format!("{username}'s Projects")
}

use anyhow::Result;
use rouille::try_or_400;
use rouille::Response;
use serde::Deserialize;
use serde::Serialize;
use serde_repr::Serialize_repr;
use std::collections::HashMap;
use std::io::Error;
use std::io::ErrorKind;

#[derive(Serialize_repr, Debug)]
#[repr(u32)]
enum Color {
    Red = 0x992D22,
    Green = 0x2ECC71,
    Grey = 0x95A5A6,
}

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
#[serde(rename_all = "lowercase")]
enum Status {
    Firing,
    Resolved,
}

#[derive(Deserialize, Debug)]
struct Annotations {
    summary: String,
    description: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Alert {
    status: Status,
    labels: HashMap<String, String>,
    annotations: Option<Annotations>,
    fingerprint: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AlertGroup {
    version: String,
    status: Status,
    alerts: Vec<Alert>,
    group_labels: HashMap<String, String>,
    common_labels: HashMap<String, String>,
    common_annotations: Option<Annotations>,
    truncated_alerts: i32,
}

#[derive(Serialize, Debug)]
struct DiscordEmbedField {
    name: String,
    value: String,
}

#[derive(Serialize, Debug)]
struct DiscordEmbed {
    title: String,
    description: String,
    color: Color,
    fields: Vec<DiscordEmbedField>,
}

#[derive(Serialize, Debug)]
struct DiscordContent {
    content: Option<String>,
    embeds: Vec<DiscordEmbed>,
}

#[async_std::main]
async fn main() -> Result<()> {
    rouille::start_server("[::]:9094", move |request| {
        let group: AlertGroup =
            try_or_400!(rouille::input::json_input(request));
        try_or_400!(forward_alert(group)
            .map_err(|e| { Error::new(ErrorKind::Other, e.to_string()) }));
        Response::text("OK")
    });
}

fn forward_alert(group: AlertGroup) -> Result<()> {
    let hook_url = std::env::var("DISCORD_WEBHOOK_URL")?.trim().to_string();
    let reqwest_client = reqwest::blocking::Client::new();

    let alert_name = group
        .common_labels
        .get("alertname")
        .map_or(String::from("unnamed"), |l| l.clone());

    let has_summary = group.common_annotations.is_some();
    let alert_summary = group
        .common_annotations
        .map_or(String::from("no summary"), |a| a.summary);

    let mut alert_by_status = HashMap::new();
    for alert in group.alerts {
        let list = alert_by_status
            .entry(alert.status.clone())
            .or_insert(Vec::new());
        list.push(alert);
    }

    for (status, alerts) in alert_by_status {
        let title = format!("[{:?}:{}] {}", status, alerts.len(), alert_name);
        let description = alert_summary.clone();

        let color = match status {
            Status::Firing => Color::Red,
            Status::Resolved => Color::Green,
        };

        let mut embed = DiscordEmbed {
            title,
            description,
            color,
            fields: Vec::new(),
        };

        let content = if has_summary {
            Some(alert_summary.clone())
        } else {
            None
        };

        for alert in alerts {
            let instance = alert
                .labels
                .get("instance")
                .map_or(String::from("unknown"), |l| l.clone());
            let exported_instance = alert.labels.get("exported_instance");

            let instance = if (instance == "unknown" || instance == "localhost")
                && exported_instance.is_some()
            {
                exported_instance.unwrap().to_string()
            } else {
                instance
            };

            let alert_name = alert
                .labels
                .get("alertname")
                .map_or(String::from("unknown"), |l| l.clone());
            let d = String::from("-");
            let name =
                format!("[{:?}]: {} on {}", status, alert_name, instance);

            let summary = alert
                .annotations
                .map_or(d.clone(), |a| a.description.unwrap_or(a.summary));
            let severity = alert
                .labels
                .get("severity")
                .map_or(String::from("INFO"), |l| l.clone().to_uppercase());
            let job = alert
                .labels
                .get("job")
                .map_or(String::from("-"), |l| l.clone());
            let value = format!("{} {} {}", severity, job, summary);

            embed.fields.push(DiscordEmbedField { name, value });
        }

        let embeds = vec![embed];
        let content = DiscordContent { content, embeds };

        reqwest_client.post(&hook_url).json(&content).send()?;
    }
    Ok(())
}

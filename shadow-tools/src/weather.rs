use crate::protocol::ToolDefinition;
use crate::util::required_string;
use color_eyre::eyre::eyre;
use reqwest::Client;
use reqwest::Url;
use serde_json::Value;

pub fn tool() -> ToolDefinition {
    ToolDefinition::new(
        "get_weather",
        "Get the current weather for a city or place using live internet data. Use this for weather, temperature, rain, wind, or outdoor-condition questions.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City, region, or country. Example: Nairobi or Nairobi, Kenya."
                }
            },
            "required": ["location"],
            "additionalProperties": false
        }),
        |args| async move { get_weather(args).await },
    )
}

async fn get_weather(args: Value) -> color_eyre::Result<String> {
    let location = required_string(&args, "location")?;
    let http = Client::new();

    let geo_url = Url::parse_with_params(
        "https://geocoding-api.open-meteo.com/v1/search",
        &[
            ("name", location.as_str()),
            ("count", "1"),
            ("language", "en"),
            ("format", "json"),
        ],
    )?;
    let geocode = http
        .get(geo_url)
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;

    let place = geocode["results"]
        .as_array()
        .and_then(|results| results.first())
        .ok_or_else(|| eyre!("could not find that location"))?;

    let latitude = place["latitude"]
        .as_f64()
        .ok_or_else(|| eyre!("weather lookup missing latitude"))?;
    let longitude = place["longitude"]
        .as_f64()
        .ok_or_else(|| eyre!("weather lookup missing longitude"))?;
    let resolved_name = [
        place["name"].as_str(),
        place["admin1"].as_str(),
        place["country"].as_str(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");

    let latitude = latitude.to_string();
    let longitude = longitude.to_string();
    let forecast_url = Url::parse_with_params(
        "https://api.open-meteo.com/v1/forecast",
        &[
            ("latitude", latitude.as_str()),
            ("longitude", longitude.as_str()),
            (
                "current",
                "temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,wind_speed_10m",
            ),
            ("timezone", "auto"),
        ],
    )?;

    let forecast = http
        .get(forecast_url)
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;

    let current = &forecast["current"];
    let weather_code = current["weather_code"].as_i64().unwrap_or(-1);

    Ok(format!(
        "Current weather for {resolved_name}: {}°C, feels like {}°C, humidity {}%, wind {} km/h, conditions: {}.",
        current["temperature_2m"].as_f64().unwrap_or_default(),
        current["apparent_temperature"].as_f64().unwrap_or_default(),
        current["relative_humidity_2m"].as_i64().unwrap_or_default(),
        current["wind_speed_10m"].as_f64().unwrap_or_default(),
        weather_code_description(weather_code),
    ))
}

fn weather_code_description(code: i64) -> &'static str {
    match code {
        0 => "clear",
        1 | 2 | 3 => "partly cloudy",
        45 | 48 => "foggy",
        51 | 53 | 55 | 56 | 57 => "drizzle",
        61 | 63 | 65 | 66 | 67 => "rain",
        71 | 73 | 75 | 77 => "snow",
        80 | 81 | 82 => "rain showers",
        85 | 86 => "snow showers",
        95 => "thunderstorm",
        96 | 99 => "thunderstorm with hail",
        _ => "unknown",
    }
}

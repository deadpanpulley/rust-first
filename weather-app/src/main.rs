use axum::{
    extract::Path,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use std::env;

// --- Structs for Geocoding API ---
#[derive(Deserialize, Debug)]
struct GeocodingResponse {
    results: Option<Vec<GeocodingResult>>,
}

#[derive(Deserialize, Debug)]
struct GeocodingResult {
    latitude: f64,
    longitude: f64,
}

// --- Structs for Weather API ---
#[derive(Deserialize, Debug)]
struct WeatherApiResponse {
    current: Current,
}

#[derive(Deserialize, Debug)]
struct Current {
    #[serde(rename = "temperature_2m")]
    temperature: f64,
    #[serde(rename = "wind_speed_10m")]
    windspeed: f64,
}

// --- Struct for our application's API response ---
#[derive(Serialize, Debug)]
struct AppWeatherResponse {
    temperature: String,
    windspeed: String,
}

#[tokio::main]
async fn main() {
    // The router that defines our application
    let app = Router::new()
        // API route
        .route("/api/weather/:city", get(weather_api_handler))
        // Static file serving for the frontend
        .nest_service("/", ServeDir::new("static"));

    // Get the port from the environment variable, default to 3000 for local dev
    let port_str = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let port = port_str.parse::<u16>().expect("PORT must be a number");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Weather app listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

// The handler for the /api/weather/:city route
async fn weather_api_handler(
    Path(city): Path<String>,
) -> Result<Json<AppWeatherResponse>, StatusCode> {
    let client = Client::new();

    // 1. Geocoding: Convert city name to latitude and longitude
    let geocoding_url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1",
        city
    );

    let geo_data = client
        .get(&geocoding_url)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .json::<GeocodingResponse>()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(city_data) = geo_data.results.and_then(|mut r| r.pop()) {
        let lat = city_data.latitude;
        let lon = city_data.longitude;

        // 2. Weather Forecast: Get weather using the coordinates
        let weather_url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,wind_speed_10m",
            lat, lon
        );

        let weather_data = client
            .get(&weather_url)
            .send()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .json::<WeatherApiResponse>()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Format the final response
        let response = AppWeatherResponse {
            temperature: format!("{}°C", weather_data.current.temperature),
            windspeed: format!("{} km/h", weather_data.current.windspeed),
        };
        Ok(Json(response))
    } else {
        // City not found
        Err(StatusCode::NOT_FOUND)
    }
}    

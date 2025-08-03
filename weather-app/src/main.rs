use eframe::{egui, App, Frame};
use reqwest::Client;
use serde::Deserialize;
use std::sync::mpsc::{channel, Receiver, Sender};
use tokio::runtime::Runtime;

// Structs for deserializing the geocoding API response from Open-Meteo
#[derive(Deserialize, Debug)]
struct GeocodingResponse {
    results: Option<Vec<GeocodingResult>>,
}

#[derive(Deserialize, Debug)]
struct GeocodingResult {
    latitude: f64,
    longitude: f64,
}

// Structs for deserializing the weather API response from Open-Meteo
#[derive(Deserialize, Debug)]
struct WeatherResponse {
    current_weather: CurrentWeather,
}

#[derive(Deserialize, Debug)]
struct CurrentWeather {
    temperature: f64,
}

// The main application struct
struct WeatherApp {
    city: String,
    weather_info: Option<String>,
    runtime: Runtime,
    // We use a channel to send data from the async thread (fetching weather)
    // to the main GUI thread.
    sender: Sender<Option<String>>,
    receiver: Receiver<Option<String>>,
}

impl Default for WeatherApp {
    fn default() -> Self {
        // Create a channel for communication
        let (sender, receiver) = channel();
        Self {
            city: "Berlin".to_string(), // Default city
            weather_info: None,
            runtime: Runtime::new().expect("Failed to create Tokio runtime."),
            sender,
            receiver,
        }
    }
}

// Implement the eframe::App trait, which is the core of the GUI application
impl App for WeatherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // Check if we have received new weather information from the async task
        if let Ok(new_info) = self.receiver.try_recv() {
            self.weather_info = new_info;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Weather App");

            // Input field for the city
            ui.horizontal(|ui| {
                ui.label("City: ");
                ui.text_edit_singleline(&mut self.city);
            });

            // Button to trigger the weather fetch
            if ui.button("Get Weather").clicked() {
                // Clone the necessary data to move into the async block
                let city = self.city.clone();
                let sender = self.sender.clone();
                let ctx_clone = ctx.clone();

                // Spawn an async task to fetch the weather without blocking the GUI
                self.runtime.spawn(async move {
                    let weather_result = fetch_weather(&city).await;
                    // Send the result back to the main thread
                    sender.send(weather_result).unwrap();
                    // Request a repaint to show the new data
                    ctx_clone.request_repaint();
                });
            }

            // Display the weather information
            if let Some(info) = &self.weather_info {
                ui.label(info);
            }
        });
    }
}

// The main entry point of the program
fn main() -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions::default();
    // Run the native eframe application
    eframe::run_native(
        "Weather App",
        native_options,
        Box::new(|_cc| Ok(Box::new(WeatherApp::default()))),
    )
}

// Asynchronous function to fetch weather data from the Open-Meteo API
async fn fetch_weather(city: &str) -> Option<String> {
    let client = Client::new();

    // 1. Geocoding: Convert city name to latitude and longitude
    let geocoding_url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1",
        city
    );

    let geo_resp = client.get(&geocoding_url).send().await.ok()?;
    if !geo_resp.status().is_success() {
        return Some(format!("Error fetching geocoding data for {}", city));
    }
    let geo_data: GeocodingResponse = geo_resp.json().await.ok()?;

    if let Some(results) = geo_data.results {
        if let Some(city_data) = results.first() {
            let lat = city_data.latitude;
            let lon = city_data.longitude;

            // 2. Weather Forecast: Get weather using the coordinates
            let weather_url = format!(
                "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current_weather=true",
                lat, lon
            );

            let weather_resp = client.get(&weather_url).send().await.ok()?;
            if !weather_resp.status().is_success() {
                return Some(format!("Error fetching weather data for {}", city));
            }
            let weather_data: WeatherResponse = weather_resp.json().await.ok()?;

            // Format the final string to be displayed
            let temp = weather_data.current_weather.temperature;
            return Some(format!("The current temperature in {} is {}°C", city, temp));
        }
    }

    Some(format!("Could not find city: {}", city))
}

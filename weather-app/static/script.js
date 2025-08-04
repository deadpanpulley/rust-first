const cityInput = document.getElementById('cityInput');
const getWeatherBtn = document.getElementById('getWeatherBtn');
const weatherResult = document.getElementById('weatherResult');

getWeatherBtn.addEventListener('click', async () => {
    const city = cityInput.value;
    if (!city) {
        weatherResult.textContent = 'Please enter a city.';
        return;
    }

    try {
        const response = await fetch(`/api/weather/${city}`);
        const data = await response.json();

        if (data) {
            weatherResult.innerHTML = `
                <p>Temperature: ${data.temperature}</p>
                <p>Wind Speed: ${data.windspeed}</p>
            `;
        } else {
            weatherResult.textContent = 'Could not find weather for that city.';
        }
    } catch (error) {
        weatherResult.textContent = 'Error fetching weather data.';
        console.error('Error:', error);
    }
});

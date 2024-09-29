use serde::Deserialize;
use reqwest::Error;


#[derive(Debug, Deserialize)]
pub struct DeribitOptionData {
    pub instrument_name: String,     // Option name (e.g., "BTC-29SEP24-56000-C")
    pub strike: f64,                 // Strike price of the option
    pub expiration_timestamp: u64,   // Expiration time (Unix timestamp)
    pub option_type: String,         // "call" or "put"
    pub price_index: String,
    pub settlement_currency: String,
    pub implied_volatility: Option<f64>,
    pub market_price: Option<f64>,
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
    pub theta: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct DeribitApiResponse {
    pub result: Vec<DeribitOptionData>,
}

impl DeribitOptionData {
    pub async fn fetch_data(asset: &str) -> Result<Vec<DeribitOptionData>, Error> {
        let url = format!("https://www.deribit.com/api/v2/public/get_instruments?currency={}&kind=option&expired=false", asset);
        let response: DeribitApiResponse = reqwest::get(&url).await?.json().await?;

        let mut options_data: Vec<DeribitOptionData> = response.result.into_iter()
            .map(|data| DeribitOptionData {
                instrument_name: data.instrument_name,
                strike: data.strike,
                expiration_timestamp: data.expiration_timestamp,
                option_type: data.option_type,
                price_index: data.price_index,
                settlement_currency: data.settlement_currency,
                implied_volatility: None,
                market_price: None,
                delta: None,
                gamma: None,
                theta: None,
                vega: None,
            })
            .collect();

        for option in &mut options_data {
            let orderbook_url = format!("https://www.deribit.com/api/v2/public/get_order_book?instrument_name={}", option.instrument_name);
            if let Ok(orderbook_response) = reqwest::get(&orderbook_url).await?.json::<serde_json::Value>().await {
                if let Some(result) = orderbook_response.get("result") {
                    // Parse market price, best bid, and implied volatility
                    option.market_price = result.get("mark_price").and_then(|p| p.as_f64());
                    option.implied_volatility = result.get("mark_iv").and_then(|iv| iv.as_f64());

                    // Parse Greeks (delta, gamma, theta, vega)
                    if let Some(greeks) = result.get("greeks") {
                        option.delta = greeks.get("delta").and_then(|v| v.as_f64());
                        option.gamma = greeks.get("gamma").and_then(|v| v.as_f64());
                        option.theta = greeks.get("theta").and_then(|v| v.as_f64());
                        option.vega = greeks.get("vega").and_then(|v| v.as_f64());
                    }
                }
            }
        }

        Ok(options_data)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_fetch_deribit_data() {
        // Create a tokio runtime to run the async test
        let rt = Runtime::new().unwrap();

        // Block on the async call to fetch the data
        rt.block_on(async {
            let asset = "BTC";  // You can change this to ETH or other assets
            let result = DeribitOptionData::fetch_data(asset).await;

            match result {
                Ok(option_data) => {
                    // Ensure that we have some data returned
                    assert!(!option_data.is_empty(), "No options data returned from Deribit");

                    // Print the first few options for debugging purposes
                    for option in option_data.iter().take(5) {
                        println!("{:?}", option);
                    }

                    // Optionally, you can check specific fields for expected ranges or values
                    let first_option = &option_data[0];
                    assert!(first_option.strike > 0.0, "Invalid underlying price");
                    assert!(first_option.implied_volatility.unwrap() > 0.0, "Invalid implied volatility");
                },
                Err(e) => panic!("Failed to fetch data: {}", e),
            }
        });
    }

}

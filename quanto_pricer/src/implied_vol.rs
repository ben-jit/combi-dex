use roots::{find_root_brent, find_root_secant};
use statrs::distribution::{Normal, ContinuousCDF};


pub struct ImpliedVolatility {
    pub spot: f64,
    pub strike: f64,
    pub r: f64,
    pub time_to_maturity: f64,
    pub market_price: f64,
    pub is_call: bool
}


impl ImpliedVolatility {
    fn black_scholes_price(&self, sigma: f64) -> f64 {
        let d1 = ((self.spot / self.strike).ln() + (self.r + 0.5 * sigma.powi(2)) * self.time_to_maturity)
            / (sigma * (self.time_to_maturity).sqrt());
        let d2 = d1 - sigma * (self.time_to_maturity).sqrt();

        let rng = Normal::new(0.0, 1.0).unwrap();

        let call_price = self.spot * rng.cdf(d1) - self.strike * (-self.r * self.time_to_maturity).exp() * rng.cdf(d2);
        if self.is_call {
            call_price
        } else {
            call_price + self.strike * (-self.r * self.time_to_maturity).exp() - self.spot
        }
    }

    pub fn implied_volatility(&self) -> f64 {
        let f = |volatility: f64| -> f64 {
            self.black_scholes_price(volatility) - self.market_price
        };

        match find_root_brent(0.001, 3.0, &f, &mut 1e-6) {
            Ok(root) => root,
            Err(_) => {
                let secant_result = find_root_secant(0.001, 3.0, &f, &mut 1e-6);
                secant_result.unwrap_or_else(|_| 0.0)
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_black_scholes_price_for_call() {
        // Given parameters for an at-the-money call option
        let option = ImpliedVolatility {
            spot: 100.0,            // Spot price
            strike: 100.0,          // Strike price (ATM)
            r: 0.05,    // 5% risk-free rate
            time_to_maturity: 1.0,  // 1 year to maturity
            market_price: 0.0,      // Not needed for this test
            is_call: true,          // This is a call option
        };

        let volatility = 0.2;
        let price = option.black_scholes_price(volatility);
        let expected_price = 10.4506;

        println!("Black-Scholes Call Option Price: {}", price);
        assert!((price - expected_price).abs() < 1e-4);
    }

    #[test]
    fn test_black_scholes_price_for_put() {
        let option = ImpliedVolatility {
            spot: 100.0,            // Spot price
            strike: 100.0,          // Strike price (ATM)
            r: 0.05,    // 5% risk-free rate
            time_to_maturity: 1.0,  // 1 year to maturity
            market_price: 0.0,      // Not needed for this test
            is_call: false,         // This is a put option
        };

        let volatility = 0.2;
        let price = option.black_scholes_price(volatility);
        let expected_price = 5.5735;

        println!("Black-Scholes Put Option Price: {}", price);
        assert!((price - expected_price).abs() < 1e-4);
    }

    #[test]
    fn test_implied_volatility_for_call() {
        let option = ImpliedVolatility {
            spot: 100.0,
            strike: 100.0,
            r: 0.05,
            time_to_maturity: 1.0,
            market_price: 10.4506, // Market price for an at-the-money European call option
            is_call: true,
        };

        let implied_vol = option.implied_volatility();
        println!("Implied volatility (call): {}", implied_vol);

        assert!((implied_vol - 0.2).abs() < 1e-2);
    }

    #[test]
    fn test_implied_volatility_for_put() {
        let option = ImpliedVolatility {
            spot: 100.0,
            strike: 100.0,
            r: 0.05,
            time_to_maturity: 1.0,
            market_price: 5.5735, // Market price for an at-the-money European put option
            is_call: false,
        };

        let implied_vol = option.implied_volatility();
        println!("Implied volatility (put): {}", implied_vol);

        assert!((implied_vol - 0.2).abs() < 1e-2);
    }
}
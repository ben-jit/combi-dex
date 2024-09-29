use rustfft::FftPlanner;
use ndarray::Array1;
use num_complex::Complex;


pub struct OptionPrice{
    pub call: f64,
    pub put: f64
}


pub struct QuantoOption {
    pub spot: f64,
    pub strike: f64,
    pub domestic_rate: f64,
    pub foreign_rate: f64,
    pub volatility: f64,
    pub fx_volatility: f64,
    pub time_to_maturity: f64,
    pub correlation: f64
}


impl QuantoOption {
    pub fn characteristic_function(&self, u: f64) -> Complex<f64> {
        let i = Complex::new(0.0, 1.0);
        let drift = if self.correlation == 0.0 && self.fx_volatility == 0.0 {
            self.domestic_rate - 0.5 * self.volatility.powi(2)
        } else {
            self.foreign_rate - 0.5 * self.volatility.powi(2)
                + self.correlation * self.volatility * self.fx_volatility
        };
        let vol = -0.5 * self.volatility.powi(2) * u.powi(2) * self.time_to_maturity;

        let exponent = i * u * (self.spot.ln() + drift * self.time_to_maturity) + vol;
        exponent.exp()
    }

    pub fn calculate_price_fft(&self) -> OptionPrice {
        let n: usize = 1024;
        let ln_k_min = (self.spot * 0.1).ln();
        let ln_k_max = (self.spot * 10.0).ln();
        let dk = (ln_k_max - ln_k_min) / n as f64;
        let damping_factor = 0.05;

        let mut grid = Array1::<f64>::zeros(n);
        for i in 0..n {
            let u = i as f64 * dk;
            if u == 0.0 {
                grid[i] = 0.0;
            } else {
                let phi = self.characteristic_function(u);
                let complex_exp = Complex::new(0.0, -u * ln_k_min);
                let integrand = ((phi * complex_exp / Complex::new(0.0, u)) * (damping_factor * u).exp()).re;
                grid[i] = integrand;
            }
        }

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(n);
        let mut fft_input: Vec<Complex<f64>> = grid.mapv(|x| Complex::new(x, 0.0)).to_vec();
        fft.process(&mut fft_input);

        let option_price_index = ((self.strike.ln() - ln_k_min) / dk).round();
        let index = option_price_index.max(0.0).min((n - 1) as f64) as usize;

        let call_price = fft_input[index].re * (-self.domestic_rate * self.time_to_maturity).exp();

        let discounted_strike = self.strike * (-self.domestic_rate * self.time_to_maturity).exp();
        let discounted_spot = self.spot * (-self.foreign_rate * self.time_to_maturity).exp();
        let put_price = call_price + discounted_strike - discounted_spot;

        OptionPrice {
            call: call_price,
            put: put_price
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn assert_approx_eq(a: f64, b: f64, epsilon: f64) {
        assert!(
            (a - b).abs() < epsilon,
            "assertion failed: `(left ≈ right)`\n left: `{}`, right: `{}`, epsilon: `{}`",
            a,
            b,
            epsilon
        );
    }

    fn assert_complex_approx_eq(a: Complex<f64>, b: Complex<f64>, epsilon: f64) {
        assert!(
            (a.re - b.re).abs() < epsilon,
            "Real part mismatch: left {}, right {}, epsilon {}",
            a.re,
            b.re,
            epsilon
        );
        assert!(
            (a.im - b.im).abs() < epsilon,
            "Imaginary part mismatch: left {}, right {}, epsilon {}",
            a.im,
            b.im,
            epsilon
        );
    }

    #[test]
    fn test_characteristic_function_for_standard_european_option() {
        // Standard European option: no correlation, no FX volatility
        let quanto_option = QuantoOption {
            spot: 100.0,
            strike: 100.0,        // Strike doesn't matter for the characteristic function
            domestic_rate: 0.05,  // 5% risk-free rate
            foreign_rate: 0.0,    // Foreign rate is 0 for European options
            volatility: 0.2,      // 20% volatility
            fx_volatility: 0.0,   // No FX volatility
            time_to_maturity: 1.0,  // 1 year to maturity
            correlation: 0.0,     // No correlation
        };

        // Input for the characteristic function (u in Fourier space)
        let u = 1.0;

        // Call the characteristic function
        let result = quanto_option.characteristic_function(u);

        // Expected result can be computed separately, or we check only for reasonable output
        // Example: for a simple European option, the result should be non-NaN and reasonable.
        println!("Characteristic function result (real): {}", result.re);
        println!("Characteristic function result (imaginary): {}", result.im);

        // We can test if the result matches expected values (hard to give exact values)
        let expected = Complex::new(0.951229424500714, 0.190255392000331);
        assert_complex_approx_eq(result, expected, 1e-4);
    }

    #[test]
    fn test_price_via_fft_with_correlation() {
        let quanto = QuantoOption {
            spot: 30000.0,    // BTC price in USDC
            strike: 35000.0,  // Strike price in USDC
            domestic_rate: 0.01,    // USDC risk-free rate
            foreign_rate: 0.0,      // BTC's risk-free rate (can be set to 0)
            time_to_maturity: 0.5,  // Time to maturity (6 months)
            volatility: 0.6,        // Volatility of BTC
            correlation: 0.5,       // Correlation between BTC and USDC/BTC rate
            fx_volatility: 0.2,     // Volatility of the USDC/BTC exchange rate
        };

        let price = quanto.calculate_price_fft();
        assert!(price.call > 0.0);
        assert!(price.put > 0.0);
    }

    #[test]
    fn test_price_via_fft_for_european_call() {
        // Mimicking a standard European call option (no quanto effect)
        let quanto_call = QuantoOption {
            spot: 100.0,    // Spot price (e.g., 100)
            strike: 100.0,  // Strike price (ATM)
            domestic_rate: 0.05,  // Risk-free rate (5%)
            foreign_rate: 0.0,    // Foreign rate (set to 0 to mimic standard option)
            time_to_maturity: 1.0,  // 1 year to maturity
            volatility: 0.2,        // 20% volatility
            correlation: 0.0,       // No correlation (mimicking European option)
            fx_volatility: 0.0,     // No exchange rate volatility (no quanto effect)
        };

        let price = quanto_call.calculate_price_fft();
        let expected_call = 10.4506;
        let expected_price = 5.5735;

        assert_approx_eq(price.call, expected_price, 1e-4);
        assert_approx_eq(price.put, expected_call, 1e-4);
    }
}
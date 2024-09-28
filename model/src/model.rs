use std::cmp::{PartialEq, Ordering};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::ser::SerializeStruct;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub balance: f64,

}
impl User {
    pub fn new(id: u64, name: &str, balance: f64) -> Self {
        User {
            id,
            name: name.to_string(),
            balance,
        }
    }
    pub fn deposit(&mut self, amount: f64) {
        self.balance += amount;
    }
    pub fn withdraw(&mut self, amount: f64) {
        self.balance -= amount;
    }
    pub fn can_afford(&self, amount: f64) -> bool {
        self.balance >= amount
    }
}
impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub base: String,
    pub quote: String,
}
impl Asset {
    pub fn new(base: &str, quote: &str) -> Self {
        Asset {
            base: base.to_string(),
            quote: quote.to_string(),
        }
    }
    pub fn from_str(s: &str) -> Self {
        let parts: Vec<&str> = s.split('/').collect();
        Asset::new(parts[0], parts[1])
    }
}
impl Eq for Asset{}
impl PartialEq for Asset {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base && self.quote == other.quote
    }
}
impl Hash for Asset {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.hash(state);
        self.quote.hash(state);
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub asset: Asset,
    pub quantity: f64,
    pub price: f64,
}
impl AssetInfo {
    pub fn new(asset: Asset, quantity: f64, price: f64) -> Self {
        AssetInfo {
            asset,
            quantity,
            price,
        }
    }
    pub fn from_str(s: &str, quantity: f64, price: f64) -> Self {
        AssetInfo {
            asset: Asset::from_str(s),
            quantity,
            price,
        }
    }
    pub fn total_value(&self) -> f64 {
        self.quantity * self.price
    }
    pub fn update_price(&mut self, price: f64) {
        self.price = price;
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Basket {
    pub id: u64,
    pub assets: Vec<AssetInfo>,
}
impl Basket {
    pub fn total_value(&self) -> f64 {
        self.assets.iter().map(|asset| asset.total_value()).sum()
    }
    pub fn update_price(&mut self, asset_str: &Asset, new_price: f64) {
        if let Some(asset) = self.assets.iter_mut().find(|a| a.asset == *asset_str) {
            asset.update_price(new_price);
        }
    }
    pub fn is_asset_in_basket(&self, asset: &AssetInfo) -> bool {
        self.assets.iter().any(|a| a.asset == asset.asset)
    }
    pub fn asset_amount_in_basket(&self, asset: &AssetInfo) -> f64 {
        self.assets.iter().find(|a| a.asset == asset.asset).map(|a| a.quantity).unwrap_or(0.0)
    }
    pub fn asset_value_in_basket(&self, asset: &AssetInfo) -> f64 {
       self.assets.iter().find(|a| a.asset == asset.asset).map(|a| a.total_value()).unwrap_or(0.0)
    }
    pub fn assets_valuation(&self) -> HashMap<Asset, f64> {
        self.assets.iter().map(|asset| (asset.asset.clone(), asset.total_value())).collect()
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BidType {
    XOR,
    OR
}
impl PartialEq for BidType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (BidType::XOR, BidType::XOR) => true,
            (BidType::OR, BidType::OR) => true,
            _ => false
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bid {
    pub user: Arc<User>,
    pub basket_id: u64,
    pub bid_type: BidType,
    pub price: f64,
    pub quantity: Option<f64>
}
impl Bid {
    pub fn new(
        user: Arc<User>,
        basket_id: u64,
        bid_type: BidType,
        price: f64,
        quantity: Option<f64>
    ) -> Self {
        Bid {
            user,
            basket_id,
            bid_type,
            price,
            quantity
        }
    }
    pub fn is_valid(&self) -> bool {
        self.user.can_afford(self.price) && self.price > 0.0 && self.quantity.map_or(true, |q| q > 0.0 && q <= 1.0)
    }
    pub fn match_basket<'a>(&self, baskets: &'a [Basket]) -> Option<&'a Basket> {
        baskets.iter().find(|basket| basket.id == self.basket_id)
    }
    pub fn estimate_value_of_bid(&self, basket: &Basket) -> f64 {
        let basket_value = basket.total_value();
        let proportion = self.quantity.unwrap_or(1.0);
        proportion * basket_value
    }
}
impl PartialEq for Bid {
    fn eq(&self, other: &Self) -> bool {
        self.price == other.price
    }
}
impl PartialOrd for Bid {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.price.partial_cmp(&other.price)
    }

    fn lt(&self, other: &Self) -> bool {
        self.price < other.price
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_user_creation() {
        let user = User::new(1, "Alice", 1000.0);
        assert_eq!(user.id, 1);
        assert_eq!(user.name, "Alice");
        assert_eq!(user.balance, 1000.0);
    }

    #[test]
    fn test_user_deposit_withdraw() {
        let mut user = User::new(1, "Alice", 1000.0);
        user.deposit(500.0);
        assert_eq!(user.balance, 1500.0);
        user.withdraw(300.0);
        assert_eq!(user.balance, 1200.0);
    }

    #[test]
    fn test_user_can_afford() {
        let user = User::new(1, "Alice", 1000.0);
        assert!(user.can_afford(500.0));
        assert!(!user.can_afford(1500.0));
    }

    #[test]
    fn test_asset_creation() {
        let asset = Asset::new("BTC", "USD");
        assert_eq!(asset.base, "BTC");
        assert_eq!(asset.quote, "USD");
    }

    #[test]
    fn test_asset_info_total_value() {
        let asset = Asset::new("BTC", "USD");
        let asset_info = AssetInfo::new(asset, 2.0, 30000.0);
        assert_eq!(asset_info.total_value(), 60000.0);
    }

    #[test]
    fn test_asset_info_update_price() {
        let asset = Asset::new("BTC", "USD");
        let mut asset_info = AssetInfo::new(asset, 2.0, 30000.0);
        asset_info.update_price(35000.0);
        assert_eq!(asset_info.price, 35000.0);
        assert_eq!(asset_info.total_value(), 2.0 * 35000.0);
    }

    #[test]
    fn test_basket_total_value() {
        let asset1 = AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0);
        let asset2 = AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0);
        let basket = Basket {
            id: 1,
            assets: vec![asset1, asset2],
        };
        assert_eq!(basket.total_value(), 70000.0);
    }

    #[test]
    fn test_basket_update_asset_price() {
        let asset = Asset::new("BTC", "USD");
        let asset_info = AssetInfo::new(asset.clone(), 2.0, 30000.0);
        let mut basket = Basket {
            id: 1,
            assets: vec![asset_info],
        };
        basket.update_price(&asset, 35000.0);
        assert_eq!(basket.assets[0].price, 35000.0);
        assert_eq!(basket.total_value(), 70000.0);
    }

    #[test]
    fn test_basket_is_asset_in_basket() {
        let asset_info = AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0);
        let basket = Basket {
            id: 1,
            assets: vec![asset_info.clone()],
        };
        assert!(basket.is_asset_in_basket(&asset_info));

        let other_asset_info = AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0);
        assert!(!basket.is_asset_in_basket(&other_asset_info));
    }

    #[test]
    fn test_basket_asset_amount_in_basket() {
        let asset_info = AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0);
        let basket = Basket {
            id: 1,
            assets: vec![asset_info.clone()],
        };
        assert_eq!(basket.asset_amount_in_basket(&asset_info), 2.0);

        let other_asset_info = AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0);
        assert_eq!(basket.asset_amount_in_basket(&other_asset_info), 0.0);
    }

    #[test]
    fn test_basket_asset_value_in_basket() {
        let asset_info = AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0);
        let basket = Basket {
            id: 1,
            assets: vec![asset_info.clone()],
        };
        assert_eq!(basket.asset_value_in_basket(&asset_info), 60000.0);

        let other_asset_info = AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0);
        assert_eq!(basket.asset_value_in_basket(&other_asset_info), 0.0);
    }

    #[test]
    fn test_basket_assets_valuation() {
        let asset1 = AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0);
        let asset2 = AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0);
        let basket = Basket {
            id: 1,
            assets: vec![asset1.clone(), asset2.clone()],
        };
        let valuation = basket.assets_valuation();
        let expected_valuation: HashMap<Asset, f64> = vec![
            (asset1.asset.clone(), 60000.0), // BTC total value
            (asset2.asset.clone(), 10000.0),
        ]
            .into_iter()
            .collect();
        assert_eq!(valuation, expected_valuation);
    }

    #[test]
    fn test_bid_creation() {
        let user = Arc::new(User::new(1, "Alice", 1000.0));
        let bid = Bid::new(user.clone(), 1, BidType::XOR, 500.0, Some(0.2));
        assert_eq!(bid.user.id, 1);
        assert_eq!(bid.basket_id, 1);
        assert_eq!(bid.bid_type, BidType::XOR);
        assert_eq!(bid.price, 500.0);
        assert_eq!(bid.quantity, Some(0.2));
    }

    #[test]
    fn test_bid_is_valid() {
        let user = Arc::new(User::new(1, "Alice", 1000.0));
        let bid = Bid::new(user.clone(), 1, BidType::XOR, 500.0, Some(0.2));
        assert!(bid.is_valid());

        let invalid_bid = Bid::new(user.clone(), 1, BidType::XOR, 1500.0, Some(0.2));
        assert!(!invalid_bid.is_valid());
    }

    #[test]
    fn test_bid_match_basket() {
        let user = Arc::new(User::new(1, "Alice", 1000.0));
        let bid = Bid::new(user.clone(), 1, BidType::XOR, 500.0, Some(0.2));

        let basket1 = Basket {
            id: 1,
            assets: vec![],
        };
        let basket2 = Basket {
            id: 2,
            assets: vec![],
        };
        let baskets = vec![basket1, basket2];

        let matched_basket = bid.match_basket(&baskets);
        assert!(matched_basket.is_some());
        assert_eq!(matched_basket.unwrap().id, 1);
    }

    #[test]
    fn test_bid_comparisons() {
        let user1 = Arc::new(User::new(1, "Alice", 1000.0));
        let user2 = Arc::new(User::new(2, "Bob", 500.0));

        let bid1 = Bid::new(user1.clone(), 1, BidType::XOR, 400.0, Some(0.2));
        let bid2 = Bid::new(user2.clone(), 1, BidType::XOR, 300.0, Some(0.2));

        assert!(bid1 > bid2);
        assert!(bid2 < bid1);
    }

    #[test]
    fn test_bid_creation_with_optional_quantity() {
        let user = Arc::new(User::new(1, "Alice", 1000.0));

        // Bid for 50% of the basket
        let bid_half = Bid::new(user.clone(), 1, BidType::XOR, 500.0, Some(0.5));
        assert_eq!(bid_half.quantity, Some(0.5));

        // Bid for the entire basket (None quantity)
        let bid_full = Bid::new(user.clone(), 1, BidType::XOR, 500.0, None);
        assert_eq!(bid_full.quantity, None);
    }

    #[test]
    fn test_bid_is_valid_with_optional_quantity() {
        let user = Arc::new(User::new(1, "Alice", 1000.0));

        // Valid bids
        let valid_half_bid = Bid::new(user.clone(), 1, BidType::XOR, 500.0, Some(0.5));
        assert!(valid_half_bid.is_valid());

        let valid_full_bid = Bid::new(user.clone(), 1, BidType::XOR, 500.0, None);
        assert!(valid_full_bid.is_valid());

        // Invalid bid with high quantity
        let invalid_bid_high_proportion = Bid::new(user.clone(), 1, BidType::XOR, 500.0, Some(1.5));
        assert!(!invalid_bid_high_proportion.is_valid());

        // Invalid bid with zero quantity
        let invalid_bid_zero_quantity = Bid::new(user.clone(), 1, BidType::XOR, 500.0, Some(0.0));
        assert!(!invalid_bid_zero_quantity.is_valid());
    }

    #[test]
    fn test_bid_estimate_value_of_bid_with_optional_quantity() {
        let user = Arc::new(User::new(1, "Alice", 1000.0));

        let asset1 = AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0);
        let asset2 = AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0);
        let basket = Basket {
            id: 1,
            assets: vec![asset1, asset2],
        };

        // Bid for 50% of the basket
        let bid_half = Bid::new(user.clone(), 1, BidType::XOR, 500.0, Some(0.5));
        let estimated_value_half = bid_half.estimate_value_of_bid(&basket);
        assert_eq!(estimated_value_half, 35000.0);

        // Bid for the entire basket (None quantity)
        let bid_full = Bid::new(user.clone(), 1, BidType::XOR, 1000.0, None);
        let estimated_value_full = bid_full.estimate_value_of_bid(&basket);
        assert_eq!(estimated_value_full, 70000.0);
    }
}
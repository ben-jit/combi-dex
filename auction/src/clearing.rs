use std::collections::HashMap;
use model::model::{User, Bid, AssetInfo, Basket};
use std::sync::Arc;


pub struct Clearing;

impl Clearing {
    pub fn clear_winning_bids(
        winning_bids: Vec<Bid>,
        allocation: HashMap<u64, Vec<AssetInfo>>,
    ) -> Result<HashMap<u64, Arc<User>>, &'static str> {
        let mut users: HashMap<u64, Arc<User>> = HashMap::new();

        for bid in winning_bids {
            let user_id = bid.user.id;
            let price = bid.price;

            // Get the user or initialize with the current bid user (if new)
            let user = users
                .entry(user_id)
                .or_insert_with(|| Arc::clone(&bid.user));

            // Ensure the user can afford the bid
            if !user.can_afford(price) {
                return Err("User cannot afford the payment");
            }

            // Deduct the price from the user's balance
            Arc::get_mut(user).unwrap().withdraw(price);

            // Handle asset allocation for the user
            if let Some(assets) = allocation.get(&user_id) {
                println!(
                    "User {} receives the following assets: {:?}",
                    user_id, assets
                );
                // Here you would implement the actual asset transfer logic.
            }
        }

        Ok(users)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use model::model::{User, Bid, Basket, AssetInfo, Asset, BidType};
    use std::sync::Arc;
    use std::collections::HashMap;

    #[test]
    fn test_clear_winning_bids() {
        let user1 = Arc::new(User::new(1, "Alice", 100000.0));
        let user2 = Arc::new(User::new(2, "Bob", 200000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0),
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),
            ],
        };

        let bid1 = Bid::new(user1.clone(), 1, BidType::XOR, 60000.0, Some(1.0));
        let bid2 = Bid::new(user2.clone(), 1, BidType::XOR, 70000.0, Some(1.0));

        let allocation = HashMap::from([
            (user1.id, vec![AssetInfo::new(Asset::new("BTC", "USD"), 1.0, 30000.0)]),
            (user2.id, vec![AssetInfo::new(Asset::new("BTC", "USD"), 1.0, 30000.0)]),
        ]);

        let bids = vec![bid1, bid2];
        let cleared_users = Clearing::clear_winning_bids(bids, allocation).unwrap();

        // Check user balances after clearing
        assert_eq!(cleared_users.get(&1).unwrap().balance, 40000.0);
        assert_eq!(cleared_users.get(&2).unwrap().balance, 130000.0);
    }

    #[test]
    fn test_cannot_afford_bid() {
        let user1 = Arc::new(User::new(1, "Alice", 50000.0));  // Can't afford 60000
        let user2 = Arc::new(User::new(2, "Bob", 200000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0),
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),
            ],
        };

        let bid1 = Bid::new(user1.clone(), 1, BidType::XOR, 60000.0, Some(1.0));  // Alice can't afford this
        let bid2 = Bid::new(user2.clone(), 1, BidType::XOR, 70000.0, Some(1.0));

        let allocation = HashMap::from([
            (user1.id, vec![AssetInfo::new(Asset::new("BTC", "USD"), 1.0, 30000.0)]),
            (user2.id, vec![AssetInfo::new(Asset::new("BTC", "USD"), 1.0, 30000.0)]),
        ]);

        let bids = vec![bid1, bid2];
        let result = Clearing::clear_winning_bids(bids, allocation);

        // Check that the clearing fails due to insufficient funds
        assert!(result.is_err());
    }
}
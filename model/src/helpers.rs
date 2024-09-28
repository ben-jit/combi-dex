use std::collections::HashMap;
use std::hash::Hash;
use crate::model::{Bid, Basket, AssetInfo};


pub fn filter_valid_bids<'a>(bids: &'a [Bid], basket: &'a Basket) -> Vec<&'a Bid> {
    bids.iter()
        .filter(|bid| bid.basket_id == basket.id)
        .filter(|bid| bid.is_valid())
        .collect()
}


pub fn sort_bids_by_price<'a>(bids: &'a [&'a Bid]) -> Vec<&'a Bid> {
    let mut sorted_bids = bids.to_vec();
    sorted_bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
    sorted_bids
}


pub fn get_highest_bid(bids: Vec<&Bid>) -> Option<&Bid> {
    bids.into_iter()
        .max_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
}


pub fn evaluate_xor_bids<'a>(bids: &'a [Bid], basket: &'a Basket) -> Option<&'a Bid> {
    let valid_bids = filter_valid_bids(bids, basket);
    get_highest_bid(valid_bids)
}


pub fn evaluate_partial_xor_bids<'a>(bids: &'a [Bid], basket: &'a Basket) -> Option<(&'a Bid, HashMap<u64, Vec<AssetInfo>>)> {
    let valid_bids = filter_valid_bids(bids, basket);
    if let Some(highest_bid) = get_highest_bid(valid_bids) {
        let allocation = allocate_basket(&[highest_bid], basket);
        Some((highest_bid, allocation))
    } else {
        None
    }
}


pub fn evaluate_or_bids<'a>(bids: &'a [Bid], basket: &'a Basket) -> Vec<&'a Bid> {
    filter_valid_bids(bids, basket)
}


pub fn evaluate_partial_or_bids<'a>(bids: &'a [Bid], basket: &'a Basket) -> (Vec<&'a Bid>, HashMap<u64, Vec<AssetInfo>>) {
    let valid_bids = filter_valid_bids(bids, basket);
    let allocation = allocate_basket(&valid_bids, basket);
    (valid_bids, allocation)
}


pub fn total_value_of_bids_for_basket(bids: &[Bid], basket: &Basket) -> f64 {
    filter_valid_bids(bids, basket)
        .iter()
        .map(|bid| bid.estimate_value_of_bid(basket))
        .sum()
}


pub fn allocate_basket(bids: &[&Bid], basket: &Basket) -> HashMap<u64, Vec<AssetInfo>> {
    let mut allocation: HashMap<u64, Vec<AssetInfo>> = HashMap::new();
    for bid in bids {
        let mut allocated_assets = Vec::new();
        let proportion = bid.quantity.unwrap_or(1.0);

        for asset in &basket.assets {
            let quantity = asset.quantity * proportion;
            let value = asset.price * quantity;
            allocated_assets.push(AssetInfo::new(asset.asset.clone(), quantity, value));
        }

        allocation.insert(bid.user.id, allocated_assets);
    }
    allocation
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::model::{Asset, AssetInfo, Bid, BidType, User};

    fn create_user(can_afford: bool) -> Arc<User> {
        Arc::new(User {
            id: 1,
            name: String::from("Test User"),
            balance: if can_afford { 1000.0 } else { 10.0 },
        })
    }

    fn create_bid(user: Arc<User>, basket_id: u64, bid_type: BidType, price: f64, quantity: Option<f64>) -> Bid {
        Bid::new(user, basket_id, bid_type, price, quantity)
    }

    fn create_basket(id: u64, assets: Vec<AssetInfo>) -> Basket {
        Basket { id, assets }
    }

    #[test]
    fn test_filter_valid_bids() {
        let user = create_user(true); // User can afford
        let basket = create_basket(1, vec![]); // Empty basket

        let valid_bid = create_bid(user.clone(), 1, BidType::XOR, 100.0, Some(0.5));
        let invalid_bid = create_bid(user.clone(), 2, BidType::XOR, 200.0, Some(0.5)); // Basket ID doesn't match

        let bids = vec![valid_bid, invalid_bid];
        let valid_bids = filter_valid_bids(&bids, &basket);

        assert_eq!(valid_bids.len(), 1); // Only one valid bid
        assert_eq!(valid_bids[0].basket_id, basket.id); // Basket ID matches
    }

    #[test]
    fn test_sort_bids_by_price() {
        let user = create_user(true);
        let bid1 = create_bid(user.clone(), 1, BidType::XOR, 100.0, None);
        let bid2 = create_bid(user.clone(), 1, BidType::XOR, 200.0, None);
        let bid3 = create_bid(user.clone(), 1, BidType::XOR, 50.0, None);

        let bids = vec![&bid1, &bid2, &bid3];
        let sorted_bids = sort_bids_by_price(&bids);

        assert_eq!(sorted_bids[0].price, 200.0);
        assert_eq!(sorted_bids[1].price, 100.0);
        assert_eq!(sorted_bids[2].price, 50.0);
    }

    #[test]
    fn test_get_highest_bid() {
        let user = create_user(true);
        let bid1 = create_bid(user.clone(), 1, BidType::XOR, 100.0, None);
        let bid2 = create_bid(user.clone(), 1, BidType::XOR, 200.0, None);
        let bid3 = create_bid(user.clone(), 1, BidType::XOR, 50.0, None);

        let bids = vec![&bid1, &bid2, &bid3];
        let highest_bid = get_highest_bid(bids);

        assert!(highest_bid.is_some());
        assert_eq!(highest_bid.unwrap().price, 200.0);
    }

    #[test]
    fn test_evaluate_xor_bids() {
        let user = create_user(true);
        let basket = create_basket(1, vec![]);
        let bid1 = create_bid(user.clone(), 1, BidType::XOR, 100.0, None);
        let bid2 = create_bid(user.clone(), 1, BidType::XOR, 200.0, None);

        let bids = vec![bid1, bid2];
        let highest_bid = evaluate_xor_bids(&bids, &basket);

        assert!(highest_bid.is_some());
        assert_eq!(highest_bid.unwrap().price, 200.0);
    }

    #[test]
    fn test_evaluate_or_bids() {
        let user = create_user(true);
        let basket = create_basket(1, vec![]);
        let bid1 = create_bid(user.clone(), 1, BidType::OR, 100.0, None);
        let bid2 = create_bid(user.clone(), 1, BidType::OR, 200.0, None);

        let bids = vec![bid1, bid2];
        let valid_bids = evaluate_or_bids(&bids, &basket);

        assert_eq!(valid_bids.len(), 2); // Both bids are valid
        assert_eq!(valid_bids[0].price, 100.0);
        assert_eq!(valid_bids[1].price, 200.0);
    }

    #[test]
    fn test_total_value_of_bids_for_basket() {
        let user = create_user(true);
        let asset_info = AssetInfo { asset: Asset::new("BTC", "USDC"), quantity: 10.0, price: 100.0 };
        let basket = create_basket(1, vec![asset_info.clone()]);
        let bid1 = create_bid(user.clone(), 1, BidType::XOR, 100.0, Some(0.5)); // 50% of the basket's value
        let bid2 = create_bid(user.clone(), 1, BidType::XOR, 200.0, None); // Full basket

        let bids = vec![bid1, bid2];
        let total_value = total_value_of_bids_for_basket(&bids, &basket);

        let expected_value = (0.5 * asset_info.total_value()) + asset_info.total_value();
        assert_eq!(total_value, expected_value);
    }

    #[test]
    fn test_evaluate_xor_bids_for_basket() {
        let user1 = Arc::new(User::new(1, "Alice", 100000.0));
        let user2 = Arc::new(User::new(2, "Bob", 200000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0),
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),
            ],
        };

        // Ensure the bids have correct basket_id
        let bid1 = Bid::new(user1.clone(), 1, BidType::XOR, 60000.0, Some(1.0));
        let bid2 = Bid::new(user2.clone(), 1, BidType::XOR, 65000.0, Some(1.0));
        let bids = vec![bid1, bid2];

        let result = evaluate_xor_bids(&bids, &basket);
        assert!(result.is_some());
        assert_eq!(result.unwrap().user.id, 2);  // Bob should win with the higher bid
    }

    #[test]
    fn test_evaluate_or_bids_for_basket() {
        let user1 = Arc::new(User::new(1, "Alice", 100000.0));
        let user2 = Arc::new(User::new(2, "Bob", 200000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0),
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),
            ],
        };

        let bid1 = Bid::new(user1.clone(), 1, BidType::OR, 60000.0, Some(1.0));
        let bid2 = Bid::new(user2.clone(), 1, BidType::OR, 65000.0, Some(1.0));
        let bids = vec![bid1, bid2];

        let results = evaluate_or_bids(&bids, &basket);
        assert_eq!(results.len(), 2);  // Both bids are valid and can win
    }

    #[test]
    fn test_evaluate_partial_xor_bids() {
        let user1 = Arc::new(User::new(1, "Alice", 1000000.0));
        let user2 = Arc::new(User::new(2, "Bob", 2000000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0),  // 60,000 total
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),   // 10,000 total
            ],
        };

        let bid1 = Bid::new(user1.clone(), 1, BidType::XOR, 60000.0, Some(1.0)); // Full basket
        let bid2 = Bid::new(user2.clone(), 1, BidType::XOR, 65000.0, Some(1.0)); // Higher bid, full basket

        // Introduce a binding to hold the array of bids, avoiding the temporary value error
        let bids = [bid1, bid2];
        let result = evaluate_partial_xor_bids(&bids, &basket);

        // Check that the highest bid is returned
        assert!(result.is_some());
        let (highest_bid, allocation) = result.unwrap();
        assert_eq!(highest_bid.user.id, 2);  // Bob should win with the higher bid

        // Check that the allocation is correct
        let allocated_assets = allocation.get(&2).unwrap();  // Allocation for Bob (user 2)
        assert_eq!(allocated_assets.len(), 2);  // Should have 2 assets allocated
        assert_eq!(allocated_assets[0].quantity, 2.0);  // Full BTC quantity
        assert_eq!(allocated_assets[1].quantity, 5.0);  // Full ETH quantity
    }

    #[test]
    fn test_evaluate_partial_or_bids() {
        let user1 = Arc::new(User::new(1, "Alice", 1000000.0));
        let user2 = Arc::new(User::new(2, "Bob", 2000000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0),  // 60,000 total
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),   // 10,000 total
            ],
        };

        let bid1 = Bid::new(user1.clone(), 1, BidType::OR, 60000.0, Some(0.5)); // 50% of basket
        let bid2 = Bid::new(user2.clone(), 1, BidType::OR, 65000.0, Some(1.0)); // Full basket

        // Introduce a binding to hold the array of bids, avoiding the temporary value error
        let bids = [bid1, bid2];
        let (valid_bids, allocation) = evaluate_partial_or_bids(&bids, &basket);

        // Check that both valid bids are returned
        assert_eq!(valid_bids.len(), 2);

        // Check that user 1 (Alice) has 50% of the basket allocated
        let alice_allocated_assets = allocation.get(&1).unwrap();  // Allocation for Alice (user 1)
        assert_eq!(alice_allocated_assets.len(), 2);  // Should have 2 assets allocated
        assert_eq!(alice_allocated_assets[0].quantity, 1.0);  // 50% of BTC (2.0 * 0.5)
        assert_eq!(alice_allocated_assets[1].quantity, 2.5);  // 50% of ETH (5.0 * 0.5)

        // Check that user 2 (Bob) has the full basket allocated
        let bob_allocated_assets = allocation.get(&2).unwrap();  // Allocation for Bob (user 2)
        assert_eq!(bob_allocated_assets.len(), 2);  // Should have 2 assets allocated
        assert_eq!(bob_allocated_assets[0].quantity, 2.0);  // Full BTC quantity
        assert_eq!(bob_allocated_assets[1].quantity, 5.0);  // Full ETH quantity
    }

    #[test]
    fn test_allocate_basket() {
        let user1 = Arc::new(User::new(1, "Alice", 1000000.0));
        let user2 = Arc::new(User::new(2, "Bob", 2000000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0),
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),
            ],
        };

        let bid1 = Bid::new(user1.clone(), 1, BidType::OR, 60000.0, Some(1.0));
        let bid2 = Bid::new(user2.clone(), 1, BidType::OR, 65000.0, Some(1.0));

        // Introduce a variable to hold the bids array, extending its lifetime
        let bids = [bid1, bid2];
        let (valid_bids, allocation) = evaluate_partial_or_bids(&bids, &basket);

        assert_eq!(valid_bids.len(), 2);
        assert_eq!(allocation.get(&1).unwrap().len(), 2); // Check user 1 has two allocated assets
        assert_eq!(allocation.get(&2).unwrap().len(), 2); // Check user 2 has two allocated assets
    }
}
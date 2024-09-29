use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::wdp::WDPSolver;
use model::model::{Bid, Basket, AssetInfo, User};
use crate::clearing::Clearing;

pub struct CombiClockAuction;

impl CombiClockAuction {

    fn evaluate_bids_in_round<'a>(
        bids: &'a [Bid],
        basket: &'a Basket,
        prices: &HashMap<&'a str, f64>,
        active_bidders: &HashSet<u64>
    ) -> (Vec<&'a Bid>, HashMap<&'a str, f64>) {
        let mut valid_bids = Vec::new();
        let mut total_demand: HashMap<&'a str, f64> = HashMap::new();
        let mut excess_demand: HashMap<&'a str, f64> = HashMap::new();

        for bid in bids.iter() {
            if !active_bidders.contains(&bid.user.id) {
                continue;
            }
            if bid.is_valid() {
                valid_bids.push(bid);
                for asset_info in &basket.assets {
                    let current_price = *prices.get(asset_info.asset.base.as_str()).unwrap_or(&asset_info.price);
                    let max_affordable_quantity = bid.price / current_price;

                    let requested_quantity = bid.quantity.unwrap_or(1.0);
                    let actual_demand = requested_quantity.min(max_affordable_quantity);

                    let demand = total_demand.entry(asset_info.asset.base.as_str()).or_insert(0.0);
                    *demand += actual_demand;
                }
            }
        }

        for asset_info in &basket.assets {
            let supply = asset_info.quantity;
            let demand = *total_demand.get(asset_info.asset.base.as_str()).unwrap_or(&0.0);
            if demand > supply {
                excess_demand.insert(asset_info.asset.base.as_str(), demand - supply);
            }
        }

        (valid_bids, excess_demand)
    }

    fn update_prices<'a>(
        current_prices: &HashMap<&'a str, f64>,
        excess_demand: &HashMap<&'a str, f64>,
        base_price_increment: f64
    ) -> HashMap<&'a str, f64> {
        let mut new_prices = current_prices.clone();

        for (asset, excess) in excess_demand {
            if *excess > 0.0 {
                let current_price = *current_prices.get(asset).unwrap();
                let dynamic_increment = base_price_increment * (1.0 + (excess / current_price) * 10.0);
                new_prices.insert(asset, current_price * (1.0 + dynamic_increment));
            }
        }

        new_prices
    }

    fn apply_activity_rule(active_bidders: &mut HashSet<u64>, valid_bids: Vec<&Bid>) {
        let bidders_in_round: HashSet<u64> = valid_bids.iter().map(|bid| bid.user.id).collect();
        *active_bidders = active_bidders.intersection(&bidders_in_round).copied().collect();
    }

    /// Allocate assets to the winning bids based on the final prices.
    fn allocate_assets<'a>(
        valid_bids: Vec<&Bid>,
        basket: &'a Basket,
        final_prices: &HashMap<&'a str, f64>
    ) -> HashMap<u64, Vec<AssetInfo>> {
        let mut allocation: HashMap<u64, Vec<AssetInfo>> = HashMap::new();

        for bid in valid_bids {
            let mut allocated_assets = Vec::new();
            let proportion = bid.quantity.unwrap_or(1.0);

            for asset_info in &basket.assets {
                if let Some(final_price) = final_prices.get(asset_info.asset.base.as_str()) {
                    let allocated_quantity = asset_info.quantity * proportion;
                    let allocated_value = allocated_quantity * final_price;
                    allocated_assets.push(AssetInfo::new(
                        asset_info.asset.clone(),
                        allocated_quantity,
                        allocated_value,
                    ));
                }
            }
            allocation.insert(bid.user.id, allocated_assets);
        }

        allocation
    }

    pub fn run_auction<'a>(
        bids: &'a [Bid],
        basket: &'a Basket,
        initial_prices: HashMap<&'a str, f64>,
        price_increment: f64,
        max_rounds: usize,
    ) -> (Vec<Bid>, HashMap<u64, Vec<AssetInfo>>, HashMap<u64, Arc<User>>) {
        let mut prices = initial_prices.clone();
        let mut active_bidders: HashSet<u64> = bids.iter().map(|bid| bid.user.id).collect();
        let mut best_allocation = HashMap::new();
        let mut best_bids = Vec::new();

        for round in 0..max_rounds {
            let (valid_bids, excess_demand) = CombiClockAuction::evaluate_bids_in_round(bids, basket, &prices, &active_bidders);
            println!("Excess demand: {:?}", excess_demand);
            if excess_demand.is_empty() {
                let owned_valid_bids: Vec<Bid> = valid_bids.into_iter().cloned().collect();
                let (winning_bids, _) = WDPSolver::maximize_welfare_cca(&owned_valid_bids, basket);
                let allocation = CombiClockAuction::allocate_assets(winning_bids, basket, &prices);
                let result = Clearing::clear_winning_bids(best_bids.clone(), best_allocation.clone()).unwrap();
                return (owned_valid_bids, allocation, result);
            }

            if round == max_rounds - 1 {
                println!("Reached maximum number of rounds with remaining excess demand.");
                let owned_valid_bids: Vec<Bid> = valid_bids.into_iter().cloned().collect();
                let (winning_bids, _) = WDPSolver::maximize_welfare_cca(&owned_valid_bids, basket);
                let allocation = CombiClockAuction::allocate_assets(winning_bids, basket, &prices);
                let result = Clearing::clear_winning_bids(best_bids.clone(), best_allocation.clone()).unwrap();
                return (owned_valid_bids, allocation, result);
            }

            prices = CombiClockAuction::update_prices(&prices, &excess_demand, price_increment);
            println!("Round {}: Updated prices: {:?}", round, prices);
            CombiClockAuction::apply_activity_rule(&mut active_bidders, valid_bids.clone());

            // Track best bids and allocation so far
            best_bids = valid_bids.into_iter().cloned().collect();
            let references_to_best_bids: Vec<&Bid> = best_bids.iter().collect();
            best_allocation = CombiClockAuction::allocate_assets(references_to_best_bids, basket, &prices);
        }
        println!("Returning best allocation after {} rounds.", max_rounds);
        let result = Clearing::clear_winning_bids(best_bids.clone(), best_allocation.clone()).unwrap();
        (best_bids, best_allocation, result)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use model::model::{Bid, User, Basket, AssetInfo, Asset, BidType};
    use std::sync::Arc;
    use std::collections::HashMap;

    #[test]
    fn test_cca_auction_no_excess_demand() {
        let user1 = Arc::new(User::new(1, "Alice", 1000000.0));
        let user2 = Arc::new(User::new(2, "Bob", 2000000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0), // 2 BTC
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),  // 5 ETH
            ],
        };

        let initial_prices = HashMap::from([("BTC", 30000.0), ("ETH", 2000.0)]);
        let price_increment = 0.10; // 10% increment per round

        let bid1 = Bid::new(user1.clone(), 1, BidType::XOR, 60000.0, Some(0.5));  // Wants 100% of basket
        let bid2 = Bid::new(user2.clone(), 1, BidType::XOR, 70000.0, Some(0.75)); // Wants 75% of basket
        let bid3 = Bid::new(user1.clone(), 1, BidType::XOR, 80000.0, Some(0.5));  // Wants 50% of basket

        let bids = vec![bid1, bid2, bid3];
        let (winning_bids, allocation, result) = CombiClockAuction::run_auction(&bids, &basket, initial_prices, price_increment, 10);

        assert_eq!(winning_bids.len(), 2);
        println!("{:?}", allocation);
    }

    #[test]
    fn test_cca_auction_w_excess_demand() {
        let user1 = Arc::new(User::new(1, "Alice", 1000000.0));
        let user2 = Arc::new(User::new(2, "Bob", 2000000.0));
        let user3 = Arc::new(User::new(3, "Charlie", 3000000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0), // 2 BTC
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),  // 5 ETH
            ],
        };

        let initial_prices = HashMap::from([("BTC", 30000.0), ("ETH", 2000.0)]);
        let price_increment = 0.10; // 10% increment per round

        let bid1 = Bid::new(user1.clone(), 1, BidType::XOR, 60000.0, Some(1.0));  // Wants 100% of basket
        let bid2 = Bid::new(user2.clone(), 1, BidType::XOR, 70000.0, Some(0.75)); // Wants 75% of basket
        let bid3 = Bid::new(user3.clone(), 1, BidType::XOR, 80000.0, Some(0.5));  // Wants 50% of basket

        let bids = vec![bid1, bid2, bid3];
        let (winning_bids, allocation, result) = CombiClockAuction::run_auction(&bids, &basket, initial_prices, price_increment, 20);

        assert_eq!(winning_bids.len(), 2);
        println!("{:?}", allocation);
    }

    #[test]
    fn test_cca_auction_with_clearing() {
        let user1 = Arc::new(User::new(1, "Alice", 1000000.0));
        let user2 = Arc::new(User::new(2, "Bob", 2000000.0));
        let user3 = Arc::new(User::new(3, "Charlie", 3000000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0),
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),
            ],
        };

        let initial_prices = HashMap::from([("BTC", 30000.0), ("ETH", 2000.0)]);
        let price_increment = 0.10; // 10% increment per round

        let bid1 = Bid::new(user1.clone(), 1, BidType::XOR, 60000.0, Some(1.0));
        let bid2 = Bid::new(user2.clone(), 1, BidType::XOR, 70000.0, Some(0.75));
        let bid3 = Bid::new(user3.clone(), 1, BidType::XOR, 80000.0, Some(0.5));

        let bids = vec![bid1, bid2, bid3];
        let (winning_bids, allocation, result) = CombiClockAuction::run_auction(&bids, &basket, initial_prices, price_increment, 10);

        // Check that the auction completed and cleared
        assert_eq!(winning_bids.len(), 2);  // Only 2 bids should win (depending on availability)
        println!("{:?}", allocation);

        // Check user balances after clearing
        assert_eq!(result.get(&1).unwrap().balance, 940000.0); // Alice pays 60000
        assert_eq!(result.get(&2).unwrap().balance, 1930000.0); // Bob pays 70000
    }
}
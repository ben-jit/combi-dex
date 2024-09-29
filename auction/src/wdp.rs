use std::collections::{HashMap, HashSet};

use model::model::{Bid, Basket, AssetInfo};
use model::helpers::{filter_valid_bids, allocate_basket, can_fulfill};

pub struct WDPSolver;

impl WDPSolver {

    pub fn solve_xor<'a>(bids: &'a [Bid], basket: &'a Basket) -> Option<&'a Bid> {
        let valid_bids = filter_valid_bids(bids, basket);
        valid_bids.into_iter()
            .max_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
    }

    pub fn solve_or<'a>(bids: &'a [Bid], basket: &'a Basket) -> (Vec<&'a Bid>, HashMap<u64, Vec<AssetInfo>>) {
        let valid_bids = filter_valid_bids(bids, basket);
        let allocation = allocate_basket(&valid_bids, basket);
        (valid_bids, allocation)
    }

    pub fn maximize_welfare_vcg<'a>(bids: &'a [Bid], basket: &'a Basket) -> (Vec<&'a Bid>, f64) {
        let valid_bids = filter_valid_bids(bids, basket);

        let mut total_value = 0.0;
        let mut selected_bids = Vec::new();

        for bid in valid_bids.iter() {
            selected_bids.push(*bid);
            total_value += bid.price;
        }

        (selected_bids, total_value)
    }

    pub fn maximize_welfare_cca<'a>(bids: &'a [Bid], basket: &'a Basket) -> (Vec<&'a Bid>, f64) {
        let valid_bids = filter_valid_bids(bids, basket);

        let mut total_value = 0.0;
        let mut selected_bids = Vec::new();
        let mut remaining_assets: HashMap<String, f64> = basket
            .assets
            .iter()
            .map(|asset_info| (asset_info.asset.base.clone(), asset_info.quantity))
            .collect();
        let mut selected_users = HashSet::new();

        for bid in valid_bids.iter() {
            if selected_users.contains(&bid.user.id) {
                continue;
            }

            let mut can_fulfill_bid = true;
            for asset_info in &basket.assets {
                let available_quantity = remaining_assets.get(&asset_info.asset.base).unwrap_or(&0.0);
                let bid_demand = bid.quantity.unwrap_or(1.0) * asset_info.quantity;

                if bid_demand > *available_quantity {
                    can_fulfill_bid = false;
                    break;
                }
            }

            if can_fulfill_bid {
                // Select this bid
                selected_bids.push(*bid);
                total_value += bid.price;
                selected_users.insert(bid.user.id);

                for asset_info in &basket.assets {
                    let available_quantity = remaining_assets.get_mut(&asset_info.asset.base).unwrap();
                    let bid_demand = bid.quantity.unwrap_or(1.0) * asset_info.quantity;
                    *available_quantity -= bid_demand;
                }
            }
        }

        (selected_bids, total_value)
    }

    pub fn branch_and_bound<'a>(bids: &'a [Bid], basket: &'a Basket) -> (Vec<&'a Bid>, f64) {
        let valid_bids = filter_valid_bids(bids, basket);
        let mut selected_bids = Vec::new();
        let mut best_solution = (Vec::new(), 0.0);  // (Bids, total value)

        fn recursive_solve<'b>(
            bids: &[&'b Bid],
            basket: &Basket,
            current_solution: &mut Vec<&'b Bid>,
            best_solution: &mut (Vec<&'b Bid>, f64),
            current_value: f64,
            level: usize
        ) {
            // Base case: if we reach the end of the bids or basket capacity is exceeded
            if level == bids.len() || !can_fulfill(current_solution, basket) {
                if current_value > best_solution.1 {
                    *best_solution = (current_solution.clone(), current_value);
                }
                return;
            }

            // Recursive case: Include or exclude current bid
            recursive_solve(bids, basket, current_solution, best_solution, current_value, level + 1);

            current_solution.push(bids[level]);
            let new_value = current_value + bids[level].price;
            recursive_solve(bids, basket, current_solution, best_solution, new_value, level + 1);
            current_solution.pop();
        }

        let valid_bids_refs: Vec<&Bid> = valid_bids.iter().map(|&bid| bid).collect();
        recursive_solve(&valid_bids_refs, basket, &mut selected_bids, &mut best_solution, 0.0, 0);

        best_solution
    }

    pub fn dynamic_programming<'a>(bids: &'a [Bid], basket: &'a Basket) -> (Vec<&'a Bid>, f64) {
        let valid_bids = filter_valid_bids(bids, basket);

        // Initialize DP table (knapsack-like problem)
        let n = valid_bids.len();
        let mut dp: Vec<Vec<f64>> = vec![vec![0.0; n + 1]; n + 1];
        let mut selected_bids: Vec<&'a Bid> = Vec::new();

        for i in 1..=n {
            for j in 1..=n {
                // Case 1: Not taking the current bid
                dp[i][j] = dp[i - 1][j];

                // Case 2: Taking the current bid if feasible
                let bid_quantity = valid_bids[i - 1].quantity.unwrap_or(1.0);
                let available_quantity = basket.assets.iter().map(|a| a.quantity).sum::<f64>();

                if bid_quantity <= available_quantity {
                    dp[i][j] = dp[i - 1][j - 1] + valid_bids[i - 1].price;
                    selected_bids.push(&valid_bids[i - 1]);
                }
            }
        }

        let max_value = dp[n][n];
        (selected_bids, max_value)
    }
}


#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use super::*;
    use model::model::{User, Asset, BidType};

    #[test]
    fn test_solve_xor() {
        let (basket, bids) = setup_sample_data();

        let winning_bid = WDPSolver::solve_xor(&bids, &basket);
        assert!(winning_bid.is_some());
        assert_eq!(winning_bid.unwrap().price, 80000.0);  // Charlie's bid should win (highest price)
    }

    #[test]
    fn test_solve_or() {
        let (basket, bids) = setup_sample_data();

        let (winning_bids, allocation) = WDPSolver::solve_or(&bids, &basket);
        assert_eq!(winning_bids.len(), 2);  // Expect Alice and Bob's bids to win based on quantity availability
        assert_eq!(allocation.len(), 2);    // Two users should get allocations
    }

    #[test]
    fn test_maximize_welfare_vcg() {
        let (basket, bids) = setup_sample_data();

        let (winning_bids, total_value) = WDPSolver::maximize_welfare_vcg(&bids, &basket);
        assert_eq!(winning_bids.len(), 3);  // All bids should be selected to maximize welfare
        assert_eq!(total_value, 210000.0);  // Total value = 60,000 + 70,000 + 80,000
    }

    #[test]
    fn test_maximize_welfare_cca() {
        let (basket, bids) = setup_sample_data();

        let (winning_bids, total_value) = WDPSolver::maximize_welfare_cca(&bids, &basket);
        assert_eq!(winning_bids.len(), 2);  // Only Alice and Bob's bids can be fulfilled
        assert_eq!(total_value, 130000.0);  // Total value = 60,000 + 70,000
    }

    #[test]
    fn test_branch_and_bound() {
        let (basket, bids) = setup_sample_data();

        let (winning_bids, total_value) = WDPSolver::branch_and_bound(&bids, &basket);
        assert_eq!(winning_bids.len(), 2);  // Only two bids can be selected based on assets
        assert_eq!(total_value, 130000.0);  // Total value = 60,000 + 70,000
    }

    #[test]
    fn test_dynamic_programming() {
        let (basket, bids) = setup_sample_data();

        let (winning_bids, total_value) = WDPSolver::dynamic_programming(&bids, &basket);
        assert_eq!(winning_bids.len(), 2);  // Expect Alice and Bob's bids to win based on DP approach
        assert_eq!(total_value, 130000.0);  // Total value = 60,000 + 70,000
    }

    // Utility function to set up sample data for the tests
    fn setup_sample_data() -> (Basket, Vec<Bid>) {
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

        let bid1 = Bid::new(user1.clone(), 1, BidType::XOR, 60000.0, Some(1.0));  // 1 BTC
        let bid2 = Bid::new(user2.clone(), 1, BidType::XOR, 70000.0, Some(1.5));  // 1.5 BTC
        let bid3 = Bid::new(user3.clone(), 1, BidType::XOR, 80000.0, Some(2.0));  // 2 BTC

        (basket, vec![bid1, bid2, bid3])
    }
}

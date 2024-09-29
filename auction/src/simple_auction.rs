use std::collections::HashMap;

use crate::wdp::WDPSolver;
use model::model::{Bid, Basket, AssetInfo};
use model::helpers::{allocate_basket};


pub struct XorAuction;

impl XorAuction {
    /// Evaluates bids for an XOR auction, returning the highest valid bid.
    pub fn evaluate_bids<'a>(bids: &'a [Bid], basket: &'a Basket) -> Option<&'a Bid> {
        WDPSolver::solve_xor(bids, basket)
    }

    /// Evaluates partial bids for an XOR auction, allocating a portion of the basket to the highest bidder.
    pub fn evaluate_partial_bids<'a>(
        bids: &'a [Bid],
        basket: &'a Basket
    ) -> Option<(&'a Bid, HashMap<u64, Vec<AssetInfo>>)> {
        if let Some(highest_bid) = WDPSolver::solve_xor(bids, basket) {
            let allocation = allocate_basket(&[highest_bid], basket);
            Some((highest_bid, allocation))
        } else {
            None
        }
    }
}


pub struct OrAuction;

impl OrAuction {
    /// Evaluates bids for an OR auction, returning all valid bids.
    pub fn evaluate_bids<'a>(bids: &'a [Bid], basket: &'a Basket) -> (Vec<&'a Bid>, HashMap<u64, Vec<AssetInfo>>) {
        WDPSolver::solve_or(bids, basket)
    }

    /// Evaluates partial bids for an OR auction, allocating portions of the basket to multiple bidders.
    pub fn evaluate_partial_bids<'a>(
        bids: &'a [Bid],
        basket: &'a Basket
    ) -> (Vec<&'a Bid>, HashMap<u64, Vec<AssetInfo>>) {
        WDPSolver::solve_or(bids, basket)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use model::model::{Bid, User, Basket, AssetInfo, Asset, BidType};
    use std::sync::Arc;

    #[test]
    fn test_xor_auction() {
        let user1 = Arc::new(User::new(1, "Alice", 1000000.0));
        let user2 = Arc::new(User::new(2, "Bob", 2000000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0),
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),
            ],
        };

        let bid1 = Bid::new(user1.clone(), 1, BidType::XOR, 60000.0, Some(1.0));
        let bid2 = Bid::new(user2.clone(), 1, BidType::XOR, 70000.0, Some(1.0));

        let bids = [bid1, bid2];
        let highest_bid = XorAuction::evaluate_bids(&bids, &basket).unwrap();
        assert_eq!(highest_bid.user.id, 2);  // Bob should win with the higher bid
    }

    #[test]
    fn test_or_auction() {
        let user1 = Arc::new(User::new(1, "Alice", 1000000.0));
        let user2 = Arc::new(User::new(2, "Bob", 2000000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0),
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),
            ],
        };

        let bid1 = Bid::new(user1.clone(), 1, BidType::OR, 60000.0, Some(0.5));
        let bid2 = Bid::new(user2.clone(), 1, BidType::OR, 70000.0, Some(0.5));

        let bids = [bid1, bid2];
        let (valid_bids, allocation) = OrAuction::evaluate_partial_bids(&bids, &basket);

        assert_eq!(valid_bids.len(), 2);  // Both bids should be valid
        assert!(allocation.contains_key(&1));  // Check allocation for Alice
        assert!(allocation.contains_key(&2));  // Check allocation for Bob
    }
}
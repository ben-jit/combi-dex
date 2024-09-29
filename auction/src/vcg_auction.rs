use std::collections::HashMap;
use std::sync::Arc;
use crate::wdp::WDPSolver;
use crate::clearing::Clearing;
use model::model::{Bid, Basket, AssetInfo, User};
use model::helpers::{allocate_basket};



pub struct VCGAuction;

impl VCGAuction {

    fn compute_payments<'a>(
        bids: &'a [Bid],
        basket: &'a Basket,
        winning_bids: &[&'a Bid],
        total_welfare: f64
    ) -> HashMap<u64, f64> {
        let mut payments: HashMap<u64, f64> = HashMap::new();

        for &winning_bid in winning_bids {
            let remaining_bids: Vec<Bid> = bids.iter()
                .filter(|&bid| bid.user.id != winning_bid.user.id)
                .cloned()
                .collect();

            let (_, welfare_without_bidder) = WDPSolver::maximize_welfare_vcg(&remaining_bids, basket);

            let payment = welfare_without_bidder - (total_welfare - winning_bid.price);
            payments.insert(winning_bid.user.id, payment.max(0.0));
        }

        payments
    }

    fn allocate_assets<'a>(
        winning_bids: Vec<&'a Bid>,
        basket: &'a Basket
    ) -> HashMap<u64, Vec<AssetInfo>> {
        allocate_basket(&winning_bids, basket)
    }


    pub fn run_auction<'a>(
        bids: &'a [Bid],
        basket: &'a Basket
    ) -> (Vec<Bid>, HashMap<u64, Vec<AssetInfo>>, HashMap<u64, f64>, HashMap<u64, Arc<User>>) {
        // Step 1: Maximize social welfare by selecting the winning bids
        let (winning_bids, total_welfare) = WDPSolver::maximize_welfare_vcg(bids, basket);

        // Step 2: Calculate payments for each winning bidder
        let payments = VCGAuction::compute_payments(bids, basket, &winning_bids, total_welfare);

        // Step 3: Allocate the basket to the winning bidders (use references)
        let allocation = VCGAuction::allocate_assets(winning_bids.clone(), basket);

        // Step 4: Clone owned bids to clear (convert references to owned Bids)
        let winning_bids_owned: Vec<Bid> = winning_bids.into_iter().cloned().collect();

        // Call Clearing to settle payments and distribute assets
        let result = Clearing::clear_winning_bids(winning_bids_owned.clone(), allocation.clone()).unwrap();

        (winning_bids_owned, allocation, payments, result)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use model::model::{Bid, User, Basket, AssetInfo, Asset, BidType};
    use std::sync::Arc;

    #[test]
    fn test_vcg_auction() {
        let user1 = Arc::new(User::new(1, "Alice", 100000.0));
        let user2 = Arc::new(User::new(2, "Bob", 200000.0));
        let user3 = Arc::new(User::new(3, "Charlie", 300000.0));

        let basket = Basket {
            id: 1,
            assets: vec![
                AssetInfo::new(Asset::new("BTC", "USD"), 2.0, 30000.0),
                AssetInfo::new(Asset::new("ETH", "USD"), 5.0, 2000.0),
            ],
        };

        let bid1 = Bid::new(user1.clone(), 1, BidType::XOR, 60000.0, Some(1.0));
        let bid2 = Bid::new(user2.clone(), 1, BidType::XOR, 70000.0, Some(1.0));
        let bid3 = Bid::new(user3.clone(), 1, BidType::XOR, 80000.0, Some(1.0));

        let bids = vec![bid1, bid2, bid3];
        let (winning_bids, allocation, payments, result) = VCGAuction::run_auction(&bids, &basket);

        assert_eq!(winning_bids.len(), 3);

        for (user_id, payment) in &payments {
            println!("User {} must pay: ${:.2}", user_id, payment);
        }

        println!("{:?}", allocation);
    }
}
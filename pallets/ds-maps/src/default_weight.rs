use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
    fn zone_add() -> Weight {
        (1_000_000 as Weight).saturating_add(DbWeight::get().writes(1))
    }
}
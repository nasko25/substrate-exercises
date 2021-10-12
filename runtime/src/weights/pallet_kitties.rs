
//! Autogenerated weights for pallet_kitties
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-10-12, STEPS: `[]`, REPEAT: 1, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 128

// Executed Command:
// target/release/node-template
// benchmark
// --extrinsic
// *
// --pallet
// pallet_kitties
// --output
// runtime/src/weights/pallet_kitties.rs
// --execution=wasm
// --wasm-execution=compiled


#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_kitties.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_kitties::WeightInfo for WeightInfo<T> {
	fn create() -> Weight {
		(51_100_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	fn breed() -> Weight {
		(64_900_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	fn transfer() -> Weight {
		(38_600_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	fn set_price() -> Weight {
		(31_100_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn buy() -> Weight {
		(103_400_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(5 as Weight))
	}
}

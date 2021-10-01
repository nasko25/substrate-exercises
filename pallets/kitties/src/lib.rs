// don't include types from std
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	pallet_prelude::*,
	traits::Randomness,
};
use frame_system::pallet_prelude::*;
use sp_runtime::ArithmeticError;
use sp_io::hashing::blake2_128;
use sp_std::result::Result;

pub use pallet::*;


// define an enum for the kitty gender
#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq)]
pub enum KittyGender {
    Male,
    Female
}

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct Kitty(pub [u8; 16]);    // each kitty must have a 128-bit value representing its dna

impl Kitty {
    pub fn gender(&self) -> KittyGender {
        // if the dna of the kitty has an even first bit, then the kitty is male
        if self.0[0] % 2 == 0 {
            KittyGender::Male
        }
        else {
            KittyGender::Female
        }
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_randomness_collective_flip::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	/// Stores all the kitties. Key is (user, kitty_id).
	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	pub type Kitties<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat, T::AccountId, // user
		Blake2_128Concat, u32,  // kitty id
		Kitty, OptionQuery
	>;

	/// Stores the next kitty Id.
	#[pallet::storage]
	#[pallet::getter(fn next_kitty_id)]
	pub type NextKittyId<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		/// A kitty is created. \[owner, kitty_id, kitty\]
		KittyCreated(T::AccountId, u32, Kitty),
        /// A new kitten is bred. \[owner, kitty_id, kitty\]
        KittyBred(T::AccountId, u32, Kitty),
	}

    #[pallet::error]
    pub enum Error<T> {
        InvalidKittyId,
        SameGender
    }

	#[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T:Config> Pallet<T> {

		/// Create a new kitty
		#[pallet::weight(1000)]
		pub fn create(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// ensure kitty id does not overflow
            if Self::next_kitty_id().overflowing_add(1).1 {
			    return Err(ArithmeticError::Overflow.into());
            }

			// NOTE: NOT a cryptographically secure random number!
			// Generate a random 128bit value
			let payload = (
                // use N previous block hashes to generate a random number
                // .1 will be a value showing when (after how many blocks) this number can be
                // used securely
				<pallet_randomness_collective_flip::Pallet<T> as Randomness<T::Hash, T::BlockNumber>>::random_seed().0,
				&sender,
				<frame_system::Pallet<T>>::extrinsic_index(),
			);
            // encode the (random) payload as a 128-bit value
			let dna = payload.using_encoded(blake2_128);

			// Create and store kitty
			let kitty = Kitty(dna);
			let kitty_id = Self::next_kitty_id();
			Kitties::<T>::insert(&sender, kitty_id, kitty.clone());
			NextKittyId::<T>::put(kitty_id + 1);

			// Emit event
			Self::deposit_event(Event::KittyCreated(sender, kitty_id, kitty));

			Ok(())
		}

        /// Breed kitties
        #[pallet::weight(1000)]
        pub fn breed(origin: OriginFor<T>, kitty_id_1: u32, kitty_id_2: u32) -> DispatchResult {
            // get the sender
            let sender = ensure_signed(origin)?;

            // use the kitties getter (Self::kitties) to get the kitties from their ids
            // since the getter returns an optional kitty, check if it is Ok or None
            //  if the getter returns None, the kitty does not exist,
            //  so early return InvalidKittyId to the calling function
            //  (because of the ?)
            let kitty1 = Self::kitties(&sender, kitty_id_1).ok_or(Error::<T>::InvalidKittyId)?;
            let kitty2 = Self::kitties(&sender, kitty_id_2).ok_or(Error::<T>::InvalidKittyId)?;

            ensure!(kitty1.gender() != kitty2.gender(), Error::<T>::SameGender);

            let kitty_id = Self::get_next_kitty_id()?;

            let kitty1_dna = kitty1.0;
            let kitty2_dna = kitty2.0;

            // generate a random value for the dna
            // the selector will decide whether to pick the dna from parent 1 or 2
            let selector = Self::random_value(&sender);
            let mut new_dna = [0u8; 16];

            for i in 0..kitty1_dna.len() {
                // combine the dna of the new kitty's parents depending on the selector
                new_dna[i] = combine_dna(kitty1_dna[i], kitty2_dna[i], selector[i]);
            }

            // create the new kitty
            let new_kitty = Kitty(new_dna);

            // insert the new kitty to the storage
            Kitties::<T>::insert(&sender, kitty_id, &new_kitty);

            // deposit an event to indicate what happened on the blockchain
            Self::deposit_event(Event::KittyBred(sender, kitty_id, new_kitty));

            Ok(())
        }
	}
}

fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
    // TODO finish

    // for example
    // selector[bit_index] = 0 -> use dna1[bit_index]
    // selector[bit_index] = 1 -> use dna2[bit_index]
    //
    // selector = 0b00000001
    // dna1     = 0b10101010
    // dna2     = 0b00001111
    // result   = 0b10101011

    0
}

impl<T: Config> Pallet<T> {
    // get the next kitty id and update NextKittyId
    // returns an error if there is no kitty id available
    fn get_next_kitty_id() -> Result<u32, DispatchError> {
        // try to add 1 to the kitty id
        // return error if an overflow happens
        NextKittyId::<T>::try_mutate(|next_id| -> Result<u32, DispatchError> {
            // get the current id
            let current_id = *next_id;
            // safe uodate the current id by adding 1
            *next_id = next_id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
            // return the current id (before the update)
            Ok(current_id)
        })
    }

    fn random_value(sender: &T::AccountId) -> [u8; 16] {
        // TODO finish
        Default::default()
    }
}

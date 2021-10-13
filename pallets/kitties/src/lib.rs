// don't include types from std
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	pallet_prelude::*,
	traits::{Randomness, Currency, ExistenceRequirement},
    transactional,
};
use frame_system::{
    pallet_prelude::*,
    offchain::{SendTransactionTypes, SubmitTransaction},
};
use sp_std::{
    prelude::*,
    convert::TryInto,
};
use sp_io::hashing::blake2_128;
use sp_runtime::offchain::storage_lock::{StorageLock, BlockAndTime};
use rand_chacha::{
    rand_core::{RngCore, SeedableRng},
    ChaChaRng,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub use pallet::*;

// only include the tests module for the "test" build
#[cfg(test)]
mod tests;

// only enabled when the "runtime-benchmarks" feature is enabled
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod weights;

pub use weights::WeightInfo;

// define an enum for the kitty gender
#[derive(Encode, Decode, Clone, Copy, RuntimeDebug, PartialEq, Eq)]
pub enum KittyGender {
    Male,
    Female
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
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
	pub trait Config: frame_system::Config + orml_nft::Config<TokenData = Kitty, ClassData = ()> + SendTransactionTypes<Call<Self>> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
        // use "fungibles" pallet if working with multiple currencies
        type Currency: Currency<Self::AccountId>;
        type WeightInfo: WeightInfo;
        #[pallet::constant]     // => make this variable available in the metadata as well
        type DefaultDifficulty: Get<u32>;

	}

    pub type KittyIndexOf<T> = <T as orml_nft::Config>::TokenId;
    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /// Get kitty price. None means not for sale.
    #[pallet::storage]
    #[pallet::getter(fn kitty_prices)]
    pub type KittyPrices<T: Config> = StorageMap<
        _,
        Blake2_128Concat, KittyIndexOf<T>,
        BalanceOf<T>, OptionQuery
    >;

    // All kitties should belong to the same class
    /// The class id for orml_nft
    #[pallet::storage]
    #[pallet::getter(fn class_id)]
    pub type ClassId<T: Config> = StorageValue<_, T::ClassId, ValueQuery>;

    /// Nonce for auto breed to prevent replay attack
    #[pallet::storage]
    #[pallet::getter(fn auto_breed_nonce)]
    pub type AutoBreedNonce<T: Config> = StorageValue<_, u32, ValueQuery>;

    // define a hook for the offchain worker
    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn offchain_worker(_now: T::BlockNumber) {
            let _ = Self::run_offchain_worker();
        }
    }

    // initialize this class at the genesis time
    #[pallet::genesis_config]
    #[derive(Default)]
    pub struct GenesisConfig;

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            // create an NFT class
            let class_id = orml_nft::Pallet::<T>::create_class(&Default::default(), Vec::new(), ())
                .expect("Cannot fail or invalid chain spec");
            ClassId::<T>::put(class_id);
        }
    }

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId", KittyIndexOf<T> = "KittyIndex", Option<BalanceOf<T>> = "Option<Balance>", BalanceOf<T> = "Balance")]
	pub enum Event<T: Config> {
		/// A kitty is created. \[owner, kitty_id, kitty\]
		KittyCreated(T::AccountId, KittyIndexOf<T>, Kitty),
        /// A new kitten is bred. \[owner, kitty_id, kitty\]
        KittyBred(T::AccountId, KittyIndexOf<T>, Kitty),
        /// A kitty is transferred. \[from, to, kitty_id\]
        KittyTransferred(T::AccountId, T::AccountId, KittyIndexOf<T>),
        /// The price for a kitty is updated. \[owner, kitty_id, price\]
        KittyPriceUpdated(T::AccountId, KittyIndexOf<T>, Option<BalanceOf<T>>),
        /// A kitty is sold. \[old_owner, new_owner, kitty_id, price\]
        KittySold(T::AccountId, T::AccountId, KittyIndexOf<T>, BalanceOf<T>),
	}

    #[pallet::error]
    pub enum Error<T> {
        InvalidKittyId,
        SameGender,
        NotOwner,
        NotForSale,
        PriceTooLow,
        BuyFromSelf,
    }

	#[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T:Config> Pallet<T> {

		/// Create a new kitty
		#[pallet::weight(T::WeightInfo::create())]
		pub fn create(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

            let dna = Self::random_value(&sender);

			// Create and store kitty
			let kitty = Kitty(dna);
            let kitty_id = orml_nft::Pallet::<T>::mint(&sender, Self::class_id(), /* metadata: */ Vec::new(), /* data: */ kitty.clone())?;

			// Emit event
			Self::deposit_event(Event::KittyCreated(sender, kitty_id, kitty));

			Ok(())
		}

        /// Breed kitties
        #[pallet::weight(T::WeightInfo::breed())]
        pub fn breed(origin: OriginFor<T>, kitty_id_1: KittyIndexOf<T>, kitty_id_2: KittyIndexOf<T>) -> DispatchResult {
            // get the sender
            let sender = ensure_signed(origin)?;

            // use the kitties getter (Self::kitties) to get the kitties from their ids
            // since the getter returns an optional kitty, check if it is Ok or None
            //  if the getter returns None, the kitty does not exist,
            //  so early return InvalidKittyId to the calling function
            //  (because of the ?)
            let kitty1 = Self::kitties(&sender, kitty_id_1).ok_or(Error::<T>::InvalidKittyId)?;
            let kitty2 = Self::kitties(&sender, kitty_id_2).ok_or(Error::<T>::InvalidKittyId)?;

            Self::do_breed(sender, kitty1, kitty2)
        }

        /// Transfer a kitty to a new owner
        #[pallet::weight(T::WeightInfo::transfer())]
        pub fn transfer(origin: OriginFor<T>, to: T::AccountId, kitty_id: KittyIndexOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            orml_nft::Pallet::<T>::transfer(&sender, &to, /* token: */ (Self::class_id(), kitty_id))?;

            // if the sender does not transfer to themselves, remove the kitty price and deposit
            // the KittyTransferred event
            if sender != to {
                KittyPrices::<T>::remove(kitty_id);

                Self::deposit_event(Event::KittyTransferred(sender, to, kitty_id));
            }

            Ok(())
        }

        /// Set a price for a kitty for sale
        /// None to delist the kitty
        #[pallet::weight(T::WeightInfo::set_price())]
        pub fn set_price(origin: OriginFor<T>, kitty_id: KittyIndexOf<T>, new_price: Option<BalanceOf<T>>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // ensure the sender is the owner of the kitty id
            ensure!(orml_nft::TokensByOwner::<T>::contains_key(&sender, (Self::class_id(), kitty_id)), Error::<T>::NotOwner);

            // set the price
            KittyPrices::<T>::mutate_exists(kitty_id, |price| *price = new_price);
            // mutate_exists() will check if the new_price is None and add new_price to KittyPrices
            // if it is not None.
            // Otherwise, it will remove the kitty_id from KittyPrices

            Self::deposit_event(Event::KittyPriceUpdated(sender, kitty_id, new_price));

            Ok(())
        }

        /// Buy a kitty
        #[pallet::weight(T::WeightInfo::buy())]
        #[transactional]
        pub fn buy(origin: OriginFor<T>, owner: T::AccountId, kitty_id: KittyIndexOf<T>, max_price: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // you should not be able to buy a kitty from yourself
            // (this could also be an early return without an error message,
            // but it does not really make sense to buy a kitty from yourself)
            ensure!(sender != owner, Error::<T>::BuyFromSelf);

            // read and delete the kitty price
            KittyPrices::<T>::try_mutate_exists(kitty_id, |price| -> DispatchResult {
                // remove the price of the kitty (and ensure it is actually for sale) as it
                // will be bought
                let price = price.take().ok_or(Error::<T>::NotForSale)?;

                // ensure the buyer is not overpaying
                ensure!(max_price >= price, Error::<T>::PriceTooLow);

                // do the actual transfer

                // since now both transfers can fail, they should be atomic
                //  (which is done by #[transactional], which will revert all storages
                //  that were changed in buy()'s body, if something fails)

                // tranfer the ownership of the kitty
                orml_nft::Pallet::<T>::transfer(&owner, &sender, (Self::class_id(), kitty_id))?;

                // send `price` from the sender to the owner of the kitty
                //  ExistenceRequirement::KeepAlive will ensure that the transfer will not kill
                //  the account of the sender if there is no more money left
                T::Currency::transfer(&sender, &owner, price, ExistenceRequirement::KeepAlive)?;

                Self::deposit_event(Event::KittySold(owner, sender, kitty_id, price));

                Ok(())
            })
        }

        // auto breed feature that is used by the offchain worker
        #[pallet::weight(1000)]
        pub fn auto_breed(origin: OriginFor<T>, kitty_id_1: KittyIndexOf<T>, kitty_id_2: KittyIndexOf<T>, _nonce: u32, _solution: u128) -> DispatchResult {
            // ensure this is an unsigned transaction because the offchain worker is designed for a
            // PoW approach, so anyone can become a miner
            // anyone with a valid solution nonce will be able to participate; they don't need an
            // account (or tokens to pay for a transaction)
            ensure_none(origin)?;

            // ensure the kitty ids are valid and get the kitties
            let kitty1 = orml_nft::Pallet::<T>::tokens(Self::class_id(), kitty_id_1).ok_or(Error::<T>::InvalidKittyId)?;
            let kitty2 = orml_nft::Pallet::<T>::tokens(Self::class_id(), kitty_id_2).ok_or(Error::<T>::InvalidKittyId)?;

            Self::do_breed(kitty1.owner, kitty1.data, kitty2.data)
        }
	}

    // need to implement this to be able to use unsigned transactions
    // otherwise people could fill the blocks with unsigned transactions
    //  (even if the _nonce and _solution are validated inside the auto_breed() function,
    //  the transaction will be included in the block, as it would be already too late to reject
    //  the transaction)
    #[pallet::validate_unsigned]
    impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        // validate_unsigned() wil be executed before a transaction is accepted in a transaction pool
        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match *call {
                // the only unsigned transaction is for auto_breed(), so we only care about it
                Call::auto_breed(kitty_id_1, kitty_id_2, nonce, solution) => {
                    // validate the solution to verify the work performed by the worker
                    if Self::validate_solution(kitty_id_1, kitty_id_2, nonce, solution) {
                        // if the solution is valid, the nonce should also match the auto_breed_nonce
                        // otherwise it is a replay attack
                        if nonce != Self::auto_breed_nonce() {
                            return InvalidTransaction::BadProof.into();
                        }

                        // if the nonce is valid, increase it with 1, to render the current
                        // solution no longer valid
                        AutoBreedNonce::<T>::mutate(|nonce| *nonce = nonce.saturating_add(1));

                        // return a valid transaction
                        ValidTransaction::with_tag_prefix("kitties")    // there could be different cathegories of transactions
                            .longevity(64_u64)  // how many blocks the transaction is valid for;
                                                // if after 64 blocks the transaction is still not
                                                // confirmed, it will be discarded
                            .propagate(true)    // since anyone can become a miner this transaction has to be sent to other nodes as well
                                                // eventually reaching a validator or creator node
                            .build()
                    } else {
                        InvalidTransaction::BadProof.into()
                    }
                },
                // in the default case, this would be an invalid unsigned transaction
                //  so all other transactions need to be signed
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}

fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
    // selector[bit_index] = 0 -> use dna1[bit_index]
    // selector[bit_index] = 1 -> use dna2[bit_index]
    //
    // selector = 0b00000001
    // dna1     = 0b10101010
    // dna2     = 0b00001111
    // result   = 0b10101011

    (!selector & dna1) | (selector & dna2)
}

impl<T: Config> Pallet<T> {
    fn kitties(owner: &T::AccountId, kitty_id: KittyIndexOf<T>) -> Option<Kitty> {
        // get the tokens for the class_id and the kitty_id
        orml_nft::Pallet::<T>::tokens(Self::class_id(), kitty_id).and_then(|x| {
            // check the owner
            if x.owner == *owner {
                // if `owner` is the owner of the kitty, return the data
                Some(x.data)
            } else {
                None
            }
        })
    }

    fn random_value(sender: &T::AccountId) -> [u8; 16] {
        // NOTE: NOT a cryptographically secure random number!
        // Generate a random 128bit value
        let payload = (
            // use N previous block hashes to generate a random number
            // .1 will be a value showing when (after how many blocks) this number can be
            // used securely
            T::Randomness::random_seed().0,
            &sender,
            <frame_system::Pallet<T>>::extrinsic_index(),
        );
        // encode the (random) payload as a 128-bit value and return it
        payload.using_encoded(blake2_128)
    }

    fn do_breed(owner: T::AccountId, kitty1: Kitty, kitty2: Kitty) -> DispatchResult {
        ensure!(kitty1.gender() != kitty2.gender(), Error::<T>::SameGender);

        let kitty1_dna = kitty1.0;
        let kitty2_dna = kitty2.0;

        // generate a random value for the dna
        // the selector will decide whether to pick the dna from parent 1 or 2
        let selector = Self::random_value(&owner);
        let mut new_dna = [0u8; 16];

        for i in 0..kitty1_dna.len() {
            // combine the dna of the new kitty's parents depending on the selector
            new_dna[i] = combine_dna(kitty1_dna[i], kitty2_dna[i], selector[i]);
        }

        // create the new kitty
        let new_kitty = Kitty(new_dna);

        // mint the new kitty to the storage
        let kitty_id = orml_nft::Pallet::<T>::mint(&owner, Self::class_id(), Vec::new(), new_kitty.clone())?;

        // deposit an event to indicate what happened on the blockchain
        Self::deposit_event(Event::KittyBred(owner, kitty_id, new_kitty));

        Ok(())
    }

    fn run_offchain_worker() -> Result<(), ()> {
        // declare a storage lock with key "kitties/lock"
        // it will spend 1 block worth of time to run the offchain worker
        let mut lock = StorageLock::<'_, BlockAndTime<frame_system::Pallet<T>>>::with_block_deadline(&b"kitties/lock"[..], 1);
        // try to acquire the lock; if another offchain worker with that key is already running and
        // holding the lock, the try_lock() line will fail, so a new offchain worker will not be run
        let _guard = lock.try_lock().map_err(|_| ())?;

        // generate a secure random seed from an offchain worker
        let random_seed = sp_io::offchain::random_seed();
        // generate random numbers using ChaChaRng
        let mut rng = ChaChaRng::from_seed(random_seed);

        // get the kitty count and convert it to u32
        // this will only work if kitty_count <= u32::max_value()
        //  it will fail if there are more kitties than the maximum number of kitties
        let kitty_count = TryInto::<u32>::try_into(orml_nft::Pallet::<T>::next_token_id(Self::class_id())).map_err(|_| ())?;

        // if there are no kitties, there is nothing to be done
        if kitty_count == 0 {
            return Ok(());
        }

        // set max iterations so the offchain worker does not work forever
        const MAX_ITERATIONS: u128 = 500;

        // get the latest nonce
        let nonce = Self::auto_breed_nonce();

        // keep count of the remaining iterations
        let mut remaining_iterations = MAX_ITERATIONS;

        // pick a random pair of kitties
        let (kitty_1, kitty_2) = loop {
            // get 2 u32 random numbers and convert them to kitty ids
            let kitty_id_1: KittyIndexOf<T> = (rng.next_u32() % kitty_count).into();
            let kitty_id_2: KittyIndexOf<T> = (rng.next_u32() % kitty_count).into();

            // get the kitties with these ids
            let kitty_1 = orml_nft::Pallet::<T>::tokens(Self::class_id(), kitty_id_1).ok_or(())?;
            let kitty_2 = orml_nft::Pallet::<T>::tokens(Self::class_id(), kitty_id_2).ok_or(())?;

            if kitty_1.data.gender() != kitty_2.data.gender() {
                break (kitty_id_1, kitty_id_2);
            }

            remaining_iterations -= 1;

            if remaining_iterations == 0 {
                return Err(());
            }
        };

        // find a solution

        // add a random solution prefix to ensure that different nodes are exploring
        // different solution spaces
        //  since u32 is a large number, it is highly unlikely that any two nodes will pick the
        //  same solution space to brute force a solution
        let solution_prefix = rng.next_u32() as u128;

        // brute force a solution

        // for the remaining iterations
        for i in 0 .. remaining_iterations {
            let solution = (solution_prefix << 32) + i;
            // if the miner is lucky and a solution was found, submit an unsigned transaction with
            // the solution
            if Self::validate_solution(kitty_1, kitty_2, nonce, solution) {
                let _ = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(Call::<T>::auto_breed(kitty_1, kitty_2, nonce, solution).into());
                break;
            }
        }

        Ok(())
    }

    fn validate_solution(kitty_id_1: KittyIndexOf<T>, kitty_id_2: KittyIndexOf<T>, nonce: u32, solution: u128) -> bool {
        let payload = (kitty_id_1, kitty_id_2, nonce, solution);
        // hash the payload
        let hash = payload.using_encoded(blake2_128);
        // convert the 128-bit hash to a u128 number
        let hash_value = u128::from_le_bytes(hash);
        let difficulty = T::DefaultDifficulty::get();

        // create a random chance of finding a valid solution (based on difficulty)
        // for example if difficulty == 2, then there is a 50% chance of finding a solution
        // if difficulty == 100, there is a 1% of a miner/worker finding a solution
        //  also hash_value is basically a random number
        hash_value < (u128::max_value() / difficulty as u128)
    }
}

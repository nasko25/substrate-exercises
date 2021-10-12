use super::*;

use crate as kitties;
use sp_core::H256;
use frame_support::{parameter_types, assert_ok, assert_noop};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup}, testing::Header,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    // create a test runtime
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        // the three pallets included in the Test runtime
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},    // System pallet - always a requirement
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},    // Balances pallet - used to deal with kitties' prices and exchanges
        KittiesModule: kitties::{Pallet, Call, Storage, Event<T>, Config},          // the kitties pallet
        Nft: orml_nft::{Pallet, Storage, Config<T>},
    }
);

// -------------------------------------
// parameter types for the system pallet
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

// use the unit type "()" for most of the types
// as it provides the default mocking behavior
impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}
// --------------------------------------
// parameter types for the balances pallet
parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = u64;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
}

// --------------------------------------
// parameter types for the randomness collective pallet
// create a static global variable that can be used by the unit tests
parameter_types! {
    pub static MockRandom: H256 = Default::default();
}

impl Randomness<H256, u64> for MockRandom {
    fn random(_subject: &[u8]) -> (H256, u64) {
        (MockRandom::get(), 0)
    }
}

// --------------------------------------
// parameter types for the orml_nft pallet
parameter_types! {
    pub const MaxClassMetadata: u32 = 0;
    pub const MaxTokenMetadata: u32 = 0;
}

impl orml_nft::Config for Test {
    type ClassId = u32;
    type TokenId = u32;
    type ClassData = ();
    type TokenData = Kitty;
    type MaxClassMetadata = MaxClassMetadata;
    type MaxTokenMetadata = MaxTokenMetadata;
}

// --------------------------------------
// parameter types for the kitties pallet
impl Config for Test {
    type Event = Event;
    type Randomness = MockRandom;
    type Currency = Balances;
    type WeightInfo = ();
}

// construct the runtime for the unit tests
// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    // build new storage into the <Test> runtime
    // generate a genesis block for the Test runtime
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    pallet_balances::GenesisConfig::<Test>{
        balances: vec![(200, 500)],
    }.assimilate_storage(&mut t).unwrap();

    <crate::GenesisConfig as GenesisBuild<Test>>::assimilate_storage(&crate::GenesisConfig::default(), &mut t).unwrap();


    let mut t: sp_io::TestExternalities = t.into();

    // set the block number to 1 in the newly created genesis state `t`
    // (events on block 0 are ignored, so in order to unit test events, the block number should not
    // be equal to 0)
    t.execute_with(|| System::set_block_number(1) );
    t
}

// standard unit test
#[test]
fn can_create() {
    // new_test_ext().execute_with will set up the environment for the test runtime
    new_test_ext().execute_with(|| {
        // Origin is created by the construct_runtime! macro
        assert_ok!(KittiesModule::create(Origin::signed(100)));

        let kitty = Kitty([59, 250, 138, 82, 209, 39, 141, 109, 163, 238, 183, 145, 235, 168, 18, 122]);

        assert_eq!(KittiesModule::kitties(&100, 0), Some(kitty.clone()));
        assert_eq!(Nft::tokens(KittiesModule::class_id(), 0).unwrap().owner, 100);

        System::assert_last_event(Event::KittiesModule(crate::Event::<Test>::KittyCreated(100, 0, kitty)));
    });
}

#[test]
fn gender() {
    assert_eq!(Kitty([0; 16]).gender(), KittyGender::Male);
    assert_eq!(Kitty([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).gender(), KittyGender::Female);
}

#[test]
fn can_breed() {
    new_test_ext().execute_with(|| {
        assert_ok!(KittiesModule::create(Origin::signed(100)));

        // set the MockRandom to ensure the second kitty has a different gender
        MockRandom::set(H256::from([2; 32]));

        assert_ok!(KittiesModule::create(Origin::signed(100)));

        // assert_noop will assert there is no state change caused by the function call and that breed() return the given error
        assert_noop!(KittiesModule::breed(Origin::signed(100), 0, 11), Error::<Test>::InvalidKittyId);
        assert_noop!(KittiesModule::breed(Origin::signed(100), 0, 0), Error::<Test>::SameGender);
        assert_noop!(KittiesModule::breed(Origin::signed(101), 0, 1), Error::<Test>::InvalidKittyId);

        assert_ok!(KittiesModule::breed(Origin::signed(100), 0, 1));

        let kitty = Kitty([187, 250, 235, 118, 211, 247, 237, 253, 187, 239, 191, 185, 239, 171, 211, 122]);

        assert_eq!(KittiesModule::kitties(&100, 2), Some(kitty.clone()));
        assert_eq!(Nft::tokens(KittiesModule::class_id(), 2).unwrap().owner, 100);

        System::assert_last_event(Event::KittiesModule(crate::Event::<Test>::KittyBred(100u64, 2u32, kitty)));
    });
}

#[test]
fn can_transfer() {
    new_test_ext().execute_with(|| {
        // create a kitty
        assert_ok!(KittiesModule::create(Origin::signed(100)));
        // set a price for the newly created kitty
        assert_ok!(KittiesModule::set_price(Origin::signed(100), 0, Some(20)));

        // no one other than the owner should be able to transfer that kitty
        assert_noop!(KittiesModule::transfer(Origin::signed(101), 102, 0), orml_nft::Error::<Test>::NoPermission);

        // transfer the kitty to a new owner
        assert_ok!(KittiesModule::transfer(Origin::signed(100), 103, 0));

        // now the previous owner can no longer transfer that kitty
        assert_noop!(KittiesModule::transfer(Origin::signed(100), 103, 0), orml_nft::Error::<Test>::NoPermission);
        // after the transfer the price of the kitty should be reset
        assert_eq!(KittyPrices::<Test>::contains_key(0), false);

        // account 103 should now have the kitty with id 0
        assert_eq!(Nft::tokens(KittiesModule::class_id(), 0).unwrap().owner, 103);

        // the last event on the blockchain should be kitty transfer
        System::assert_last_event(Event::KittiesModule(crate::Event::<Test>::KittyTransferred(100, 103, 0)));
    });
}

#[test]
fn handle_self_transfer() {
    new_test_ext().execute_with(|| {
        assert_ok!(KittiesModule::create(Origin::signed(100)));

        // reset the events state to ensure that no events were transmitted
        // after the creation of the kitty
        System::reset_events();

        // user should not be able to transfer kitties they don't own
        assert_noop!(KittiesModule::transfer(Origin::signed(100), 100, 10), orml_nft::Error::<Test>::TokenNotFound);

        // tranferring a kitty you own to yourself should do nothing
        assert_ok!(KittiesModule::transfer(Origin::signed(100), 100, 0));

        assert_eq!(Nft::tokens(KittiesModule::class_id(), 0).unwrap().owner, 100);

        // there should be no event after the system event reset, because no transfer
        // should have been executed
        assert_eq!(System::events().len(), 0);
    });
}

// TODO add tests for set_price() and buy()
#[test]
fn can_set_price() {
    new_test_ext().execute_with(|| {
        // create a kitty for account wit id 100
        // the newly created kitty will have id 0
        assert_ok!(KittiesModule::create(Origin::signed(100)));

        // account 101 should not be able to set the price for 100's kitty
        assert_noop!(KittiesModule::set_price(Origin::signed(101), 0, Some(15)), Error::<Test>::NotOwner);

        // account 100 should be able to set the price of its own kitty
        assert_ok!(KittiesModule::set_price(Origin::signed(100), 0, Some(20)));

        // a KittyPriceUpdated event should have been submitted after the price change
        System::assert_last_event(Event::KittiesModule(crate::Event::KittyPriceUpdated(100, 0, Some(20))));

        // kitty 0's price should be correctly set now
        assert_eq!(KittiesModule::kitty_prices(0), Some(20));

        // setting a kitty not for sale is the same as setting its price to None
        assert_ok!(KittiesModule::set_price(Origin::signed(100), 0, None));

        // now kitty 0 should no longer be for sale
        assert_eq!(KittiesModule::kitty_prices(0), None);
        // and KittyPrices should no longer have kitty 0's id
        //  (as the kitty is no longer for sale and its price was set to None)
        assert_eq!(KittyPrices::<Test>::contains_key(0), false);

        // a KittyPriceUpdated event should have been submitted after removing kitty 0's price
        System::assert_last_event(Event::KittiesModule(crate::Event::KittyPriceUpdated(100, 0, None)));
    });
}

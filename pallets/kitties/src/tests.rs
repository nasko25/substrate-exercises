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
        KittiesModule: kitties::{Pallet, Call, Storage, Event<T>},          // the kitties pallet
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
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
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
// parameter types for the kitties pallet
impl Config for Test {
    type Event = Event;
    type Randomness = MockRandom;
    type KittyIndex = u32;
}

// construct the runtime for the unit tests
// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    // build new storage into the <Test> runtime
    // generate a genesis block for the Test runtime
    let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into();
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

        assert_eq!(KittiesModule::kitties(100, 0), Some(kitty.clone()));
        assert_eq!(KittiesModule::next_kitty_id(), 1);

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

        assert_eq!(KittiesModule::kitties(100, 2), Some(kitty.clone()));
        assert_eq!(KittiesModule::next_kitty_id(), 3);

        System::assert_last_event(Event::KittiesModule(crate::Event::<Test>::KittyBred(100u64, 2u32, kitty)));
    });
}

#[test]
fn can_transfer() {
    new_test_ext().execute_with(|| {
        // create a kitty
        assert_ok!(KittiesModule::create(Origin::signed(100)));
        const KITTY: Kitty = Kitty([59, 250, 138, 82, 209, 39, 141, 109, 163, 238, 183, 145, 235, 168, 18, 122]);

        // account 100 should have the kitty with id 0
        assert_eq!(KittiesModule::kitties(100, 0), Some(KITTY));

        // no one other than the owner should be able to transfer that kitty
        assert_noop!(KittiesModule::transfer(Origin::signed(101), 102, 0), Error::<Test>::InvalidKittyId);

        // account 0 should still have the kitty with id 0
        assert_eq!(KittiesModule::kitties(100, 0), Some(KITTY));

        // transfer the kitty to a new owner
        assert_ok!(KittiesModule::transfer(Origin::signed(100), 103, 0));

        // now past owner can no longer transfer that kitty
        assert_noop!(KittiesModule::transfer(Origin::signed(100), 103, 0), Error::<Test>::InvalidKittyId);

        // account 103 should now have the kitty with id 0
        assert_eq!(KittiesModule::kitties(103, 0), Some(KITTY));
        // and account 0 should no longer have kitty 0
        assert_eq!(KittiesModule::kitties(100, 0), None);
        assert_eq!(Kitties::<Test>::contains_key(100, 0), false);

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
        assert_noop!(KittiesModule::transfer(Origin::signed(100), 100, 10), Error::<Test>::InvalidKittyId);

        // tranferring a kitty you own to yourself should do nothing
        assert_ok!(KittiesModule::transfer(Origin::signed(100), 100, 0));

        const KITTY: Kitty = Kitty([59, 250, 138, 82, 209, 39, 141, 109, 163, 238, 183, 145, 235, 168, 18, 122]);

        assert_eq!(KittiesModule::kitties(100, 0), Some(KITTY));

        // there should be no event after the system event reset, because no transfer
        // should have been executed
        assert_eq!(System::events().len(), 0);
    });
}

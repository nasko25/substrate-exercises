// include all types in lib.rs
use super::*;

use frame_system::RawOrigin;
use frame_benchmarking::{ benchmarks, impl_benchmark_test_suite, whitelisted_caller, account };

// usually testing the "happy" pass, which is usually the longer pass
//  as error passes return earlier
benchmarks! {
    create {
        // use the whitelisted caller, as caller also introduces some storage access overhead, like
        // nonce, account balance, etc.
        // for the whitelisted caller, this overhead is not counted
        // we need this because this overhead is standard overhead and is already counted in the extrinsic based weights
        let caller = whitelisted_caller();  // ignore the storage access of this caller
    }: _(RawOrigin::Signed(caller))     // pass the benchmarking a create() method

    breed {
        let caller = whitelisted_caller();

        // mint the parent kitties
        let mut kitty = Kitty(Default::default());
        let kitty_id = orml_nft::Pallet::<T>::mint(&caller, Pallet::<T>::class_id(), Vec::new(), kitty.clone())?;

        kitty.0[0] = 1;  // modify the kitty DNA, so one is a male and the other is a female
        let kitty_id2 = orml_nft::Pallet::<T>::mint(&caller, Pallet::<T>::class_id(), Vec::new(), kitty)?;
    }: _(RawOrigin::Signed(caller), kitty_id, kitty_id2)    // pass the benchmarking a breed() method

    transfer {
        let caller = whitelisted_caller();
        // generate a test account with account()
        let to = account("to", 0, 0);

        // transfer the kitty to the test account
        let kitty_id = orml_nft::Pallet::<T>::mint(&caller, Pallet::<T>::class_id(), Vec::new(), Kitty(Default::default()))?;
    }: _(RawOrigin::Signed(caller), to, kitty_id)

    // the difference between set_price() and clear_price() is really small, so just ignore
    // clear_price()
    set_price {
        let caller = whitelisted_caller();

        let kitty_id = orml_nft::Pallet::<T>::mint(&caller, Pallet::<T>::class_id(), Vec::new(), Kitty(Default::default()))?;
    }: _(RawOrigin::Signed(caller), kitty_id, Some(100u32.into()))

    buy {
        let caller = whitelisted_caller();
        let seller = account("seller", 0, 0);

        let _ = T::Currency::make_free_balance_be(&caller, 1000u32.into());

        let kitty_id = orml_nft::Pallet::<T>::mint(&seller, Pallet::<T>::class_id(), Vec::new(), Kitty(Default::default()))?;
        Pallet::<T>::set_price(RawOrigin::Signed(seller.clone()).into(), kitty_id, Some(500u32.into()))?;
    }: _(RawOrigin::Signed(caller), seller, kitty_id, 500u32.into())
}

// convert the benchmarks above to unit tests, so they can be used as additional unit tests
//  (they could also have additional verify block to do additional assertions)
impl_benchmark_test_suite!(
    Pallet,
    crate::tests::new_test_ext(),
    crate::tests::Test,
);

use parity_codec::{Decode, Encode};
use runtime_primitives::traits::As;
/// A runtime module template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references

/// For more guidance on Substrate modules, see the example module
/// https://github.com/paritytech/substrate/blob/master/srml/example/src/lib.rs
use support::{
    decl_event, decl_module, decl_storage,
    dispatch::{Result, Vec},
    ensure, StorageMap,
};
use system::ensure_signed;

pub type BalanceOf<T> = <T as generic_asset::Trait>::Balance;
pub type AssetIdOf<T> = <T as generic_asset::Trait>::AssetId;
pub type PriceOf<T> = (AssetIdOf<T>, BalanceOf<T>);

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Domain<AccountId, Balance> {
    owner: AccountId,
    price: Option<Balance>,
}

/// The module's configuration trait.
pub trait Trait: generic_asset::Trait {
    // TODO: Add other types and constants required configure this module.

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as Ubns {
        Domains get(domains): map Vec<u8> => Option<Domain<T::AccountId, PriceOf<T>>>;
        Addresses get(addresses): map (Vec<u8>, Vec<u8>) => Option<Vec<u8>>;
        SubDomains get(sub_domain_by_index): map (Vec<u8>, u64) => Vec<u8>;
        SubDomainsCount get(sub_domain_count): map Vec<u8> => u64;
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event<T>() = default;

        pub fn purchase(origin, domain: Vec<u8>) -> Result {
            let who = ensure_signed(origin)?;
            ensure!(!<Domains<T>>::exists(domain.clone()), "domain name is unavailable");
            let domain_info = Domain {
                owner: who.clone(),
                price: None,
            };

            <Domains<T>>::insert(domain.clone(), domain_info);

            Self::deposit_event(RawEvent::Purchased(domain, who));
            Ok(())
        }

        pub fn delete(origin, domain: Vec<u8>) -> Result {
            let who = ensure_signed(origin)?;
            if let Some(domain_info) = <Domains<T>>::get(domain.clone()) {
                ensure!(domain_info.owner == who, "you don't own the domain");
                <Domains<T>>::remove(domain.clone());
                let sub_domain_count = <SubDomainsCount<T>>::get(domain.clone());
                for index in 0..sub_domain_count {
                    let sub_domain = <SubDomains<T>>::get((domain.clone(), index));
                    <Addresses<T>>::remove((domain.clone(), sub_domain.clone()));
                    <SubDomains<T>>::remove((domain.clone(), index));
                }
                <SubDomainsCount<T>>::remove(domain.clone());

                Self::deposit_event(RawEvent::Delete(domain, who));
                return Ok(());
            }
            Err("domain not found")
        }

        pub fn ask(origin, domain: Vec<u8>, price: u64) -> Result {
            let who = ensure_signed(origin)?;
            if let Some(mut domain_info) = <Domains<T>>::get(domain.clone()) {
                ensure!(domain_info.owner == who, "you don't own the domain");
                domain_info.price = Some((<AssetIdOf<T> as As<u64>>::sa(16000), <BalanceOf<T> as As<u64>>::sa(price)));
                <Domains<T>>::insert(domain, domain_info);

                return Ok(());
            }
            Err("domain not found")
        }

        pub fn buy(origin, domain: Vec<u8>) -> Result {
            let who = ensure_signed(origin)?;
            if let Some(domain_info) = <Domains<T>>::get(domain.clone()) {
                ensure!(domain_info.owner != who, "you already owned the domain");
                ensure!(domain_info.price != None, "the domain is not for sale");
                let price = domain_info.price.unwrap();
                <generic_asset::Module<T>>::make_transfer_with_event(&price.0, &who, &domain_info.owner, price.1)?;
                <Domains<T>>::remove(domain.clone());
                let sub_domain_count = <SubDomainsCount<T>>::get(domain.clone());
                for index in 0..sub_domain_count {
                    let sub_domain = <SubDomains<T>>::get((domain.clone(), index));
                    <Addresses<T>>::remove((domain.clone(), sub_domain.clone()));
                    <SubDomains<T>>::remove((domain.clone(), index));
                }
                <SubDomainsCount<T>>::remove(domain.clone());

                <Domains<T>>::insert(domain.clone(), Domain{
                    owner: who.clone(),
                    price: None,
                });

                Self::deposit_event(RawEvent::Buy(domain, who, domain_info.owner));
                return Ok(());
            }
            Err("domain not found")
        }

        pub fn add_sub_domain(origin, domain: Vec<u8>, sub_domain: Vec<u8>, address: Vec<u8>) -> Result {
            let who = ensure_signed(origin)?;
            if let Some(domain_info) = <Domains<T>>::get(domain.clone()) {
                ensure!(domain_info.owner == who, "you don't own the domain");
                ensure!(!<Addresses<T>>::exists((domain.clone(), sub_domain.clone())), "sub domain is already exists");
                <Addresses<T>>::insert((domain.clone(), sub_domain.clone()), address.clone());
                let sub_domain_count = Self::sub_domain_count(&domain.clone());
                <SubDomains<T>>::insert((domain.clone(), sub_domain_count), sub_domain.clone());
                let new_sub_domain_count = sub_domain_count.checked_add(1)
                                .ok_or("add_sub_domain causes overflow of sub_domain_count")?;
                <SubDomainsCount<T>>::insert(domain.clone(), new_sub_domain_count);

                Self::deposit_event(RawEvent::AddSubDomain(who, domain, sub_domain, address));
                return Ok(());
            }
            Err("domain not found")
        }
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        //
        Purchased(Vec<u8>, AccountId),
        //
        Delete(Vec<u8>, AccountId),
        //
        Ask(Vec<u8>, AccountId, u128),
        //
        Buy(Vec<u8>, AccountId, AccountId),
        //
        AddSubDomain(AccountId, Vec<u8>, Vec<u8>, Vec<u8>),
    }
);

/// tests for this module
#[cfg(test)]
mod tests {
    use super::*;

    use primitives::{Blake2Hasher, H256};
    use runtime_io::with_externalities;
    use runtime_primitives::{
        testing::{Digest, DigestItem, Header},
        traits::{BlakeTwo256, IdentityLookup},
        BuildStorage,
    };
    use support::{assert_ok, impl_outer_origin};

    impl_outer_origin! {
        pub enum Origin for Test {}
    }

    // For testing the module, we construct most of a mock runtime. This means
    // first constructing a configuration type (`Test`) which `impl`s each of the
    // configuration traits of modules we want to use.
    #[derive(Clone, Eq, PartialEq)]
    pub struct Test;
    impl system::Trait for Test {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Digest = Digest;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();
        type Log = DigestItem;
    }
    impl Trait for Test {
        type Event = ();
    }
    type Ubns = Module<Test>;

    // This function basically just builds a genesis storage key/value store according to
    // our desired mockup.
    fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap()
            .0
            .into()
    }

    #[test]
    fn it_works_for_default_value() {
        with_externalities(&mut new_test_ext(), || {
            // Just a dummy test for the dummy funtion `do_something`
            // calling the `do_something` function with a value 42
            assert_ok!(Ubns::do_something(Origin::signed(1), 42));
            // asserting that the stored value is equal to what we stored
            assert_eq!(Ubns::something(), Some(42));
        });
    }
}

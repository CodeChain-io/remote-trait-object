// Copyright 2020 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use super::store::run_store;
use super::types::*;
use remote_trait_object::*;

struct MyCreditCard {
    balance: u32,
}

impl CreditCard for MyCreditCard {
    fn pay(&mut self, money: u32) -> Result<(), ()> {
        if self.balance >= money {
            self.balance -= money;
            Ok(())
        } else {
            Err(())
        }
    }
}

impl Service for MyCreditCard {}

fn test_runner(f: impl Fn(Box<dyn Store>)) {
    let crate::transport::TransportEnds {
        recv1,
        send1,
        recv2,
        send2,
    } = crate::transport::create();
    let store_runner =
        std::thread::Builder::new().name("Store Runner".to_owned()).spawn(move || run_store((send2, recv2))).unwrap();

    let (rto_context, store): (Context, ServiceRef<dyn Store>) = Context::with_initial_service(
        Config::default_setup(),
        send1,
        recv1,
        ServiceRef::from_service(create_null_service()),
    );
    let store: Box<dyn Store> = store.into_remote();

    f(store);

    rto_context.firm_close(None).unwrap();
    store_runner.join().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::RwLock;
    use std::sync::{Arc, Barrier};

    #[test]
    fn test_order1() {
        fn f(store: Box<dyn Store>) {
            assert_eq!(store.order_coke("Cherry", 4), "Here's a Cherry coke");
            assert_eq!(store.order_coke("Cherry", 3), "Here's a Cherry coke");
            assert_eq!(store.order_coke("Cherry", 2), "Not enough money");

            assert_eq!(store.order_pizza(Pizza::Pepperoni, 13), "Here's a delicious pepperoni pizza");
            assert_eq!(store.order_pizza(Pizza::Pepperoni, 12), "Not enough money");
        }
        test_runner(f);
    }

    #[test]
    fn test_order2() {
        fn f(store: Box<dyn Store>) {
            let card = Arc::new(RwLock::new(MyCreditCard {
                balance: 11,
            }));
            let card_to_give = card.clone() as Arc<RwLock<dyn CreditCard>>;
            assert_eq!(
                store.order_pizza_credit_card(Pizza::Veggie, ServiceRef::from_service(card_to_give.clone())),
                "Here's a delicious veggie pizza"
            );
            assert_eq!(
                store.order_pizza_credit_card(Pizza::Veggie, ServiceRef::from_service(card_to_give.clone())),
                "Not enough balance"
            );
            card.write().balance += 10;
            assert_eq!(
                store.order_pizza_credit_card(Pizza::Veggie, ServiceRef::from_service(card_to_give)),
                "Here's a delicious veggie pizza"
            );
        }
        test_runner(f);
    }

    #[test]
    fn test_order3() {
        fn f(store: Box<dyn Store>) {
            let n = 64;
            let card = Arc::new(RwLock::new(MyCreditCard {
                balance: 11 * n as u32,
            }));

            let start = Arc::new(Barrier::new(n));
            let mut threads = Vec::new();

            let store: Arc<dyn Store> = Arc::from(store);
            for _ in 0..n {
                let store = store.clone();
                let start = start.clone();
                let card_to_give = card.clone() as Arc<RwLock<dyn CreditCard>>;
                threads.push(std::thread::spawn(move || {
                    start.wait();
                    assert_eq!(
                        store.order_pizza_credit_card(Pizza::Pineapple, ServiceRef::from_service(card_to_give)),
                        "Here's a delicious pineapple pizza"
                    );
                }));
            }

            for t in threads {
                t.join().unwrap();
            }
        }
        test_runner(f);
    }

    #[test]
    fn drop_service_which_holds_remote() {
        let crate::transport::TransportEnds {
            recv1,
            send1,
            recv2,
            send2,
        } = crate::transport::create();

        let store_runner = std::thread::Builder::new()
            .name("Store Runner".to_owned())
            .spawn(move || run_store((send2, recv2)))
            .unwrap();

        let (rto_context, store): (Context, ServiceRef<dyn Store>) = Context::with_initial_service(
            Config::default_setup(),
            send1,
            recv1,
            ServiceRef::from_service(create_null_service()),
        );
        let mut store: Box<dyn Store> = store.into_remote();

        let card = Box::new(MyCreditCard {
            balance: 0,
        }) as Box<dyn CreditCard>;
        store.register_card(ServiceRef::from_service(card));

        rto_context.disable_garbage_collection();
        // This must not fail
        drop(store);
        rto_context.firm_close(None).unwrap();
        store_runner.join().unwrap();
    }

    #[test]
    fn credit_card_cast() {
        let crate::transport::TransportEnds {
            recv1,
            send1,
            recv2,
            send2,
        } = crate::transport::create();
        let store_runner = std::thread::Builder::new()
            .name("Store Runner".to_owned())
            .spawn(move || run_store((send2, recv2)))
            .unwrap();

        let (rto_context, store): (Context, ServiceRef<dyn Store>) = Context::with_initial_service(
            Config::default_setup(),
            send1,
            recv1,
            ServiceRef::from_service(create_null_service()),
        );
        let store: Box<dyn WeirdSmallStore> = store.cast_service().unwrap().into_remote();
        assert_eq!(store.order_pizza(Pizza::Pepperoni, &&&&&&&&&&&&&&13), "Here's a delicious pepperoni pizza");

        drop(store);
        rto_context.firm_close(None).unwrap();

        store_runner.join().unwrap();
    }
}

pub fn massive_no_export(n: usize) {
    fn f(n: usize, store: Box<dyn Store>) {
        for _ in 0..n {
            assert_eq!(store.order_pizza(Pizza::Pepperoni, 13), "Here's a delicious pepperoni pizza");
        }
    }
    test_runner(|store: Box<dyn Store>| f(n, store));
}

pub fn massive_with_export(n: usize) {
    fn f(n: usize, store: Box<dyn Store>) {
        for _ in 0..n {
            let card = Box::new(MyCreditCard {
                balance: 13,
            }) as Box<dyn CreditCard>;
            assert_eq!(
                store.order_pizza_credit_card(Pizza::Pepperoni, ServiceRef::from_service(card)),
                "Here's a delicious pepperoni pizza"
            );
        }
    }
    test_runner(|store: Box<dyn Store>| f(n, store));
}

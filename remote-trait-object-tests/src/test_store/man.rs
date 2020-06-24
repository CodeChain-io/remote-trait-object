use super::store::run_store;
use super::types::*;
use cbasesandbox::ipc::Ipc;
use crossbeam::channel::bounded;
use remote_trait_object::*;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Barrier};

struct MyCreditCard {
    balance: AtomicU32,
}

impl CreditCard for MyCreditCard {
    fn pay(&self, money: u32) -> Result<(), ()> {
        if self.balance.load(Ordering::SeqCst) >= money {
            self.balance.fetch_sub(money, Ordering::SeqCst);
            Ok(())
        } else {
            Err(())
        }
    }
}

impl Service for MyCreditCard {}

fn test_runner(f: impl Fn(Arc<dyn Store>)) {
    let (ipc_arg1, ipc_arg2) = cbasesandbox::ipc::intra::Intra::arguments_for_both_ends();
    let (export_send, export_recv) = bounded(100);
    let (signal_send, signal_recv) = bounded(0);

    let store_runner = std::thread::Builder::new()
        .name("Store Runner".to_owned())
        .spawn(move || run_store(ipc_arg2, export_send, signal_recv))
        .unwrap();
    let (ipc_send, ipc_recv) = cbasesandbox::ipc::intra::Intra::new(ipc_arg1).split();

    let rto_context = Context::new(ipc_send, ipc_recv);
    let store_handle: HandleToExchange = serde_cbor::from_slice(&export_recv.recv().unwrap()).unwrap();
    let store = import_service!(Store, rto_context, store_handle);

    f(store);

    signal_send.send(()).unwrap();
    store_runner.join().unwrap();
}

#[test]
fn test_order1() {
    fn f(store: Arc<dyn Store>) {
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
    fn f(store: Arc<dyn Store>) {
        let card = Arc::new(MyCreditCard {
            balance: AtomicU32::new(11),
        });
        let card_to_give = card.clone() as Arc<dyn CreditCard>;
        assert_eq!(
            store.order_pizza_credit_card(Pizza::Veggie, SArc::new(card_to_give.clone())),
            "Here's a delicious veggie pizza"
        );
        assert_eq!(store.order_pizza_credit_card(Pizza::Veggie, SArc::new(card_to_give.clone())), "Not enough balance");
        card.balance.fetch_add(10, Ordering::SeqCst);
        assert_eq!(
            store.order_pizza_credit_card(Pizza::Veggie, SArc::new(card_to_give)),
            "Here's a delicious veggie pizza"
        );
    }
    test_runner(f);
}

#[test]
fn test_order3() {
    fn f(store: Arc<dyn Store>) {
        let n = 64;
        let card = Arc::new(MyCreditCard {
            balance: AtomicU32::new(11 * n as u32),
        });

        let start = Arc::new(Barrier::new(n));
        let mut threads = Vec::new();

        for _ in 0..n {
            let store = store.clone();
            let start = start.clone();
            let card_to_give = card.clone() as Arc<dyn CreditCard>;
            threads.push(std::thread::spawn(move || {
                start.wait();
                assert_eq!(
                    store.order_pizza_credit_card(Pizza::Pineapple, SArc::new(card_to_give)),
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

use super::types::*;
use crate::ipc::{IntraRecv, IntraSend};
use crossbeam::channel::{Receiver, Sender};
use remote_trait_object::*;
use std::sync::Arc;

struct MyPizzaStore {
    vat: u32,
}

impl MyPizzaStore {
    fn order_pizza_common(&self, menu: Pizza) -> (u32, &'static str) {
        match menu {
            Pizza::Pepperoni => (12, "pepperoni pizza"),
            Pizza::Veggie => (8, "veggie pizza"),
            Pizza::Pineapple => (10, "pineapple pizza"),
        }
    }
}

impl Store for MyPizzaStore {
    fn order_pizza(&self, menu: Pizza, money: u32) -> String {
        let (price, name) = self.order_pizza_common(menu);
        if price + self.vat <= money {
            format!("Here's a delicious {}", name)
        } else {
            "Not enough money".to_owned()
        }
    }

    fn order_coke(&self, flavor: &str, money: u32) -> String {
        let price = match flavor {
            "Plain" => 1,
            "Cherry" => 2,
            "Zero" => 3,
            _ => return "Not available".to_owned(),
        };
        if price + self.vat <= money {
            format!("Here's a {} coke", flavor)
        } else {
            "Not enough money".to_owned()
        }
    }

    fn order_pizza_credit_card(&self, menu: Pizza, credit_card: SArc<dyn CreditCard>) -> String {
        let credit_card = credit_card.unwrap();
        let (price, name) = self.order_pizza_common(menu);
        match credit_card.pay(price + self.vat) {
            Ok(_) => format!("Here's a delicious {}", name),
            Err(_) => "Not enough balance".to_owned(),
        }
    }
}

impl Service for MyPizzaStore {}

pub fn run_store(ipc: (IntraSend, IntraRecv), export_channel: Sender<Vec<u8>>, end_signal: Receiver<()>) {
    let (ipc_send, ipc_recv) = ipc;
    let rto_context = Context::new(ipc_send, ipc_recv);
    let store = Arc::new(MyPizzaStore {
        vat: 1,
    }) as Arc<dyn Store>;
    let handle = export_service::<dyn Store>(&rto_context, store);
    export_channel.send(serde_cbor::to_vec(&handle).unwrap()).unwrap();
    end_signal.recv().unwrap();
}

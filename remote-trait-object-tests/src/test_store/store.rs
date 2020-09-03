use super::types::*;
use crate::transport::{IntraRecv, IntraSend};
use remote_trait_object::*;

struct MyPizzaStore {
    vat: u32,
    registered_card: Option<Box<dyn CreditCard>>,
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

    fn order_pizza_credit_card(&self, menu: Pizza, credit_card: ServiceRef<dyn CreditCard>) -> String {
        let mut credit_card: Box<dyn CreditCard> = credit_card.into_object();
        let (price, name) = self.order_pizza_common(menu);
        let result = credit_card.pay(price + self.vat);
        match result {
            Ok(_) => format!("Here's a delicious {}", name),
            Err(_) => "Not enough balance".to_owned(),
        }
    }

    fn register_card(&mut self, credit_card: ServiceRef<dyn CreditCard>) {
        self.registered_card.replace(credit_card.unwrap_import().into_proxy());
    }
}

impl Service for MyPizzaStore {}

pub fn run_store(transport: (IntraSend, IntraRecv)) {
    let (transport_send, transport_recv) = transport;
    let rto_context = Context::with_initial_service_export(
        Config::default_setup(),
        transport_send,
        transport_recv,
        ServiceToExport::new(Box::new(MyPizzaStore {
            vat: 1,
            registered_card: None,
        }) as Box<dyn Store>),
    );
    rto_context.firm_close(None).unwrap();
}

#[cfg(test)]
pub fn create_store() -> Box<dyn Store> {
    Box::new(MyPizzaStore {
        vat: 1,
        registered_card: None,
    })
}

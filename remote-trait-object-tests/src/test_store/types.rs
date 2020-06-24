use remote_trait_object::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Pizza {
    Pepperoni,
    Veggie,
    Pineapple,
}

#[rto_macro::service]
pub trait CreditCard: Service {
    fn pay(&self, money: u32) -> Result<(), ()>;
}

#[rto_macro::service]
pub trait Store: Service {
    fn order_pizza(&self, menu: Pizza, money: u32) -> String;
    fn order_coke(&self, flavor: &str, money: u32) -> String;
    fn order_pizza_credit_card(&self, menu: Pizza, credit_card: SArc<dyn CreditCard>) -> String;
}

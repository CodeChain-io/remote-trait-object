use remote_trait_object::*;
use serde::{Deserialize, Serialize};

pub struct Bincode;

impl SerdeFormat for Bincode {
    fn to_vec<S: serde::Serialize>(s: &S) -> Result<Vec<u8>, ()> {
        bincode::serialize(s).map_err(|_| ())
    }

    fn from_slice<D: serde::de::DeserializeOwned>(data: &[u8]) -> Result<D, ()> {
        bincode::deserialize(data).map_err(|_| ())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Pizza {
    Pepperoni,
    Veggie,
    Pineapple,
}

#[service]
pub trait CreditCard: Service {
    fn pay(&mut self, money: u32) -> Result<(), ()>;
}

/// We use a different format for test
#[service(serde_format = Bincode)]
pub trait Store: Service {
    fn order_pizza(&self, menu: Pizza, money: u32) -> String;
    fn order_coke(&self, flavor: &str, money: u32) -> String;
    fn order_pizza_credit_card(&self, menu: Pizza, credit_card: ServiceRef<dyn CreditCard>) -> String;
    fn register_card(&mut self, credit_card: ServiceRef<dyn CreditCard>);
}

// Some variations of traits for tests

/// This fails to compile without `no_skeleton`
#[service(no_skeleton, serde_format = Bincode)]
pub trait WeirdSmallStore: Service {
    fn order_pizza(&self, menu: Pizza, money: &&&&&&&&&&&&&&u32) -> String;
}

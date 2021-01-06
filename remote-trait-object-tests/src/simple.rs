//! This is a working version of the example in the crate-level documentation of `remote-trait-object`

use remote_trait_object::*;

#[service]
pub trait CreditCard: Service {
    fn pay(&mut self, ammount: u64) -> Result<(), ()>;
}
struct SomeCreditCard {
    money: u64,
}
impl Service for SomeCreditCard {}
impl CreditCard for SomeCreditCard {
    fn pay(&mut self, ammount: u64) -> Result<(), ()> {
        if ammount <= self.money {
            self.money -= ammount;
            Ok(())
        } else {
            Err(())
        }
    }
}

#[service]
pub trait PizzaStore: Service {
    fn order_pizza(&self, credit_card: ServiceRef<dyn CreditCard>) -> Result<String, ()>;
}
struct SomePizzaStore;
impl Service for SomePizzaStore {}
impl PizzaStore for SomePizzaStore {
    fn order_pizza(&self, credit_card: ServiceRef<dyn CreditCard>) -> Result<String, ()> {
        let mut credit_card_proxy: Box<dyn CreditCard> = credit_card.unwrap_import().into_proxy();
        credit_card_proxy.pay(10)?;
        Ok("Tasty Pizza".to_owned())
    }
}

#[test]
fn test() {
    let crate::transport::TransportEnds {
        recv1,
        send1,
        recv2,
        send2,
    } = crate::transport::create();

    let _context_pizza_town = Context::with_initial_service_export(
        Config::default_setup(),
        send1,
        recv1,
        ServiceToExport::new(Box::new(SomePizzaStore) as Box<dyn PizzaStore>),
    );

    let (_context_customer, pizza_store): (_, ServiceToImport<dyn PizzaStore>) =
        Context::with_initial_service_import(Config::default_setup(), send2, recv2);
    let pizza_store_proxy: Box<dyn PizzaStore> = pizza_store.into_proxy();

    let my_credit_card = Box::new(SomeCreditCard { money: 11 }) as Box<dyn CreditCard>;
    assert_eq!(
        pizza_store_proxy
            .order_pizza(ServiceRef::create_export(my_credit_card))
            .unwrap(),
        "Tasty Pizza"
    );

    let my_credit_card = Box::new(SomeCreditCard { money: 9 }) as Box<dyn CreditCard>;
    assert!(pizza_store_proxy
        .order_pizza(ServiceRef::create_export(my_credit_card))
        .is_err());
}

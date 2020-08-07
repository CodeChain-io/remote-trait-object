# remote-trait-object

![Build Status](https://github.com/CodeChain-io/intertrait/workflows/ci/badge.svg)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](
https://github.com/CodeChain-io/remote-trait-object#license)
[![Cargo](https://img.shields.io/crates/v/remote-trait-object.svg)](
https://crates.io/crates/remote-trait-object)
[![Documentation](https://docs.rs/remote-trait-object/badge.svg)](
https://docs.rs/remote-trait-object)
[![chat](https://img.shields.io/discord/569610676205781012.svg?logo=discord)](https://discord.gg/xhpdXm7)

`remote-trait-object` is a general, powerful, and simple [remote method invocation](https://en.wikipedia.org/wiki/Distributed_object_communication) library
based on trait objects.

It is...

1. Based on _services_ that can be exported and imported **as trait objects** -
You register a service object, which is a trait object, and export it. On the other side, you import it into a proxy object, which is also a trait object.
1. Based on a point-to-point connection - All operations are conducted upon a single connection, which has **two ends**.
1. Easy to export and import services - During a remote method call in some service, you **can export and import another service as an argument or a return value** of the method.
1. Independent from the transport model - The transport model is abstracted and **users must provide a concrete implementation of it**.
1. Concurrent - you can both **call and handle remote calls concurrently**.

See the [documentation](https://docs.rs/remote-trait-object).

## Example

[This example code](https://github.com/CodeChain-io/remote-trait-object/blob/master/remote-trait-object-tests/src/simple.rs)
briefly shows how you can use `remote-trait-object`.

Note that `crate::transport::create()` is for creating the transport ends that are provided to `remote-trait-object ` contexts, which is just in-process communication for the test.
You have to implement your own transport implementation if you're going to actually use this crate./

```rust
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

    let my_credit_card = Box::new(SomeCreditCard {
        money: 11,
    }) as Box<dyn CreditCard>;
    assert_eq!(pizza_store_proxy.order_pizza(ServiceRef::create_export(my_credit_card)).unwrap(), "Tasty Pizza");

    let my_credit_card = Box::new(SomeCreditCard {
        money: 9,
    }) as Box<dyn CreditCard>;
    assert!(pizza_store_proxy.order_pizza(ServiceRef::create_export(my_credit_card)).is_err());
}
```
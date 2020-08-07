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

/*!
`remote-trait-object` is a general, powerful, and simple [remote method invocation](https://en.wikipedia.org/wiki/Distributed_object_communication) library
based on trait objects.

It is...

1. Based on _services_ that can be exported and imported **as trait objects** -
You register a service object, which is a trait object, and export it. On the other side, you import it into a proxy object, which is also a trait object.
1. Based on a point-to-point connection - All operations are conducted upon a single connection, which has **two ends**.
1. Easy to export and import services - During a remote method call in some service, you **can export and import another service as an argument or a return value** of the method.
1. Independent from the transport model - The transport model is abstracted and **users must provide a concrete implementation of it**.
1. Concurrent - you can both **call and handle remote calls concurrently**.

Note that it is commonly abbreviated as **RTO**.

## Introduction

**_Connection_** is a concept of a solid point-to-point pair where all operations of `remote-trait-object` are conducted.

**_Context_** is one end of a connection, and one connection will have two instances of this. Each side will access the _connection_ via the _context_.
This corresponds to [`Context`].

**_Service_** is a set of well-defined, coherent operations that can be provided for use by other code.
All communication between two parties takes place only through _services_.

**_Service object_** is the subject who provides a _service_.
It is a trait object that is wrapped in a _skeleton_.
The wrapping _skeleton_ will invoke its method to handle remote calls from the other side.
You can use any trait object as a _service object_, as long as the trait is a service trait.
You can even export your proxy object as a service object.

**_Skeleton_** is a wrapper of a service object, which is registered on a _context_ and will be invoked with remote calls from its _proxy object_ from the _client_ side.
This corresponds to [`ServiceToExport`] or [`Skeleton`].

**_Proxy object_** is the provided _service_.
It is a trait object that is a proxy to the remote _service object_ on the _server_ side. You can call its methods just like a local object.
A _proxy object_ corresponds to exactly one _skeleton_, and vice versa.
If a proxy object is dropped, it will request its deletion on the server side. This is called _delete request_.
With this, the server side's context will remove the skeleton if the client doesn't own its proxy anymore.

**_Service trait_** is a trait that represents a `service`.
It is for two trait objects (_service object_ and _proxy object_).
Both can share an identical _service trait_, but might have different but compatible _service traits_ as well.

**_Server_** side refers to one side (or one _context_) in which the _skeleton_ resides.
When talking about service exchange, it is sometimes called the _exporter_ side.

**_Client_** side refers to one side (or one _context_) in which the _proxy object_ resides.
When talking about service exchange, it is sometimes called the _importer_ side.

Note that the concept of _server_ and _client_ are for one _skeleton_-_proxy_ pair, not the whole _context_-_context_ pair itself.
No context is either _server_ nor _client_ by itself, but can be referred as one when we say a particular _skeleton_-_proxy_ pair.

**_Handle_** is an index-like identifier that corresponds to a particular _skeleton_ in the _server_ side's context. Each _proxy object_ is carrying one.
This corresponds to [`ServiceToImport`] or [`HandleToExchange`].

**_Exporting_** a service object means
wrapping it in a _skeleton_,
registering that on the _server_ side's _context_,
producing a _handle_ to it,
and passing the _handle_ to the client side.
Note that you won't go through all these processes unless you're using [`raw_exchange`] module.

**_Importing_** a handle into a proxy object means creating an object that fulfills its method calls by remotely invoking the skeleton on the server side.
It carries the handle to fill it in the packet to send, which is for identifying the skeleton that this proxy corresponds to.

It is sometimes called an _exchange_ when referring to both _export_ and _import_.

## How It Works
![diagram](https://github.com/CodeChain-io/remote-trait-object/raw/master/remote-trait-object/flow.png)

1. User calls a method of _proxy object_ which is a trait object wrapped in a smart pointer.
2. The call will be delivered to the _context_ from which the _proxy object_ is imported, after serialized into a byte packet.
Note that the actual transportation of data happens only at the _context_, which functions as a connection end.
3. The packet will be sent to the other end, (or context) by the _transport_.
4. After the other side's _context_ receives the packet, it forwards the packet to the target _skeleton_ in its registry.
5. The skeleton will dispatch the packet into an actual method call to the _service object_, which is a trait object wrapped in a smart pointer.

The result will go back to the user, again via the contexts and transport.

## Smart Pointers
Both service object and proxy object are trait objects like `dyn MyService`.
To own and pass a trait object, it should be holded by some smart pointer.
Currently `remote-trait-object` supports three types of smart pointers for service objects and proxy objects.

1. `Box<dyn MyService>`
2. `std::sync::Arc<dyn MyService>`
3. `std::sync::Arc<parking_lot::RwLock<dyn MyService>>`

When you export a service object, you can **export from whichever type** among them.

On the other hand when you import a proxy object, you can **import into whichever type** among them.
Choosing smart pointer types is completely independent for exporting & importing sides.
Both can decide which to use, depending on their own requirements.

**Exporter (server)**
- Use `Box<>` when you have nothing to do with the object after you export it.
It will be registered in the [`Context`], and will be alive until the corresponding proxy object is dropped.
You can never access the object directly, since it will be _moved_ to the registry.

- Use `Arc<>` when you have something to do with the object after you export it, by `Arc::clone()` and holding somewhere.
In this case, both its proxy object and some `Arc` copy on the exporter side can access the object,
though the latter can only access it immutably.
With this a single _service object_ can be shared among multiple _skeletons_ while a _skeleton_ always matches to exactly one _service object_.

- Use `Arc<RwLock<>>` when you have to access the object **mutably**, in the similar situation with `Arc` case.

**Importer (client)**
- This is not different from the plain Rust programming. Choose whichever type you want depending on your use.

Note that `Arc<>` will not be supported if the trait has a method that takes `&mut self`.
You must use either `Box<>` or `Arc<RwLock<>>` in such casse.


## Service Trait
Service trait is the core idea of the `remote-trait-object`. Once you define a trait that you want to use it
as a interface between two ends, you can put `#[remote_trait_object::service]` to make it as a service trait.
It will generate all required code to construct a proxy and skeleton to it.

### Trait Requirements
There are some rules to use a trait as a service trait.

1. Of course, the trait must be **object-safe** because it will be used as a trait object.

1. It can't have any type item.

1. No generic parameter (including lifetime) is allowed, in both trait definition and methods.

1. All types appeared in method parameter or return value must implement [`serde`]'s [`Serialize`] and [`Deserialize`].
This library performs de/serialization of data using [`serde`], though the data format can be chosen.
Depending on your choice of macro arguments, this condition may differ slightly. See this [section](https://github.com/CodeChain-io/remote-trait-object)

1. You can't return a reference as a return type.
This holds for a composite type too. For example, you can't return `&i32` nor `(i32, i32, &i32)`.

1. You can pass only first-order reference as a parameter.
For example, you can pass `&T` only if the `T` doesn't a contain reference at all.
Note that T must be `Sized`. There are two exceptions that accept `?Sized` `T`s: `str` and `[U]` where `U` doesn't contain reference at all.

### Example
```
use remote_trait_object as rto;

#[remote_trait_object_macro::service]
pub trait PizzaStore : rto::Service {
    fn order_pizza(&mut self, menu: &str, money: u64);
    fn ask_pizza_price(&self, menu: &str) -> u64;
}
```

### Service Compatibility
Although it is common to use the same trait for both proxy object and service object, it is possible to import a service into another trait.

TODO: We have not strictly designed the compatibility model but will be provided in the next version.

Roughly, in current version, trait `P` is considered to be compatible to be proxy of trait `S`, only if
1. `P` has exactly the same methods as `S` declared in the same order, that differ only in types of parameter and return value.
2. Such different types must be compatible.
3. Types are considered to be compatible if both are serialized and deserialized with exactly the same value.

`remote-trait-object` always guarantees 3. between [`ServiceToExport`], [`ServiceToImport`] and [`ServiceRef`].

## Export & Import services
One of the core features of `remote-trait-object` is its simple and straightforward but extensive export & import of services.
Of course this library doesn't make you manually register a service object, passing handle and so on, but provides you a much simpler and abstracted way.

There are three ways of exporting and importing a service.

### During Initialization
When you create new `remote-trait-object` contexts, you can export and import one as initial services.
See details [here](./struct.Context.html#method.with_initial_service)

### As a Parameter or a Return Value
This is the most common way of exporting / importing services.

See [`ServiceToExport`], [`ServiceToImport`] and [`ServiceRef`] for more.

### Raw Exchange
You will be **rarely** needed to perform a service exchange using a raw _skeleton_ and _handle_.
If you use this method, you will do basically the same thing as what the above methods would do internally, but have some extra controls over it.
Raw exchange is not that frequently required. In most cases using only method 1. and 2. will be sufficient.

See the [module-level documentation](./raw_exchange/index.html) for more.

## Example
```ignore
use remote_trait_object::*;

#[service]
pub trait CreditCard: Service {
    fn pay(&mut self, ammount: u64) -> Result<(), ()>;
}
struct SomeCreditCard { money: u64 }
impl CreditCard for SomeCreditCard {
    fn pay(&mut self, ammount: u64) -> Result<(), ()> {
        if ammount <= self.money {
            self.money -= ammount;
            Ok(())
        } else { Err(()) }
    }
}

#[service]
pub trait PizzaStore: Service {
    fn order_pizza(&self, credit_card: ServiceRef<dyn CreditCard>) -> Result<String, ()>;
}
struct SomePizzaStore;
impl PizzaStore for SomePizzaStore {
    fn order_pizza(&self, credit_card: ServiceRef<dyn CreditCard>) -> Result<String, ()> {
        let mut credit_card_proxy: Box<dyn CreditCard> = credit_card.unwrap_import().into_proxy();
        credit_card_proxy.pay(10)?;
        Ok("Tasty Pizza".to_owned())
    }
}

// PROGRAM 1
let (send, recv) = unimplemented!("Implement your own transport medium and provide here!")
let _context_pizza_town = Context::with_initial_service_export(
    Config::default_setup(), send, recv,
    ServiceToExport::new(Box::new(SomePizzaStore) as Box<dyn PizzaStore>),
);

// PROGRAM 2
let (send, recv) = unimplemented!("Implement your own transport medium and provide here!")
let (_context_customer, pizza_store): (_, ServiceToImport<dyn PizzaStore>) =
    Context::with_initial_service_import(Config::default_setup(), send, recv);
let pizza_store_proxy: Box<dyn PizzaStore> = pizza_store.into_proxy();

let my_credit_card = Box::new(SomeCreditCard {money: 11}) as Box<dyn CreditCard>;
assert_eq!(pizza_store_proxy.order_pizza(
    ServiceRef::create_export(my_credit_card)).unwrap(), "Tasty Pizza");

let my_credit_card = Box::new(SomeCreditCard {money: 9}) as Box<dyn CreditCard>;
assert!(pizza_store_proxy.order_pizza(
    ServiceRef::create_export(my_credit_card)).is_err());
```
You can check the working code of this example [here](https://github.com/CodeChain-io/remote-trait-object/blob/master/remote-trait-object-tests/src/simple.rs).

See more examples [here](https://github.com/CodeChain-io/remote-trait-object/tree/master/remote-trait-object-tests/src).

[`Arc`]: https://doc.rust-lang.org/std/sync/struct.Arc.html
[`Skeleton`]: ./raw_exchange/struct.Skeleton.html
[`HandleToExchange`]: ./raw_exchange/struct.HandleToExchange.html
[`Serialize`]: https://docs.serde.rs/serde/trait.Serialize.html
[`Deserialize`]: https://docs.serde.rs/serde/trait.Deserialize.html
*/

#[macro_use]
extern crate log;

mod context;
mod forwarder;
mod packet;
mod port;
mod queue;
mod service;
#[cfg(test)]
mod tests;
pub mod transport;

pub use context::{Config, Context};
pub use service::id::setup_identifiers;
pub use service::serde_support::{ServiceRef, ServiceToExport, ServiceToImport};
pub use service::{SerdeFormat, Service};

pub mod raw_exchange {
    //! This module is needed only if you want to perform some raw exchange (or export/import) of services.
    //!
    //! You may have [`Skeleton`], which is a service to be registered, but **with its trait erased**.
    //! You can prepare one and hold it for a while, and register it on demand.
    //! Creating an instance of [`Skeleton`] doesn't involve any context.
    //! That means you can have a service object that both its trait and its context (to be exported later) remains undecided.
    //!
    //! You may also have [`HandleToExchange`], which is a raw handle as a result of exporting a [`Skeleton`].
    //! It should be imported as a proxy object on the other side, but you can manage it freely until that moment.
    //! It is useful when there is a third party besides two contexts of a single connection, who wants to perform service exchange by itself, not directly between contexts.
    //!
    //! Raw exchange is not that frequently required. In most cases just using ordinary methods like [`ServiceToExport`], [`ServiceToImport`] or [`ServiceRef`] would be enough.
    //! Please check again that you surely need this module.
    //!
    //! [`ServiceToExport`]: ../struct.ServiceToExport.html
    //! [`ServiceToImport`]: ../struct.ServiceToImport.html
    //! [`ServiceRef`]: ../enum.ServiceRef.html

    pub use crate::service::export_import::{
        export_service_into_handle, import_service_from_handle, HandleToExchange, ImportProxy, IntoSkeleton, Skeleton,
    };
}

#[doc(hidden)]
pub mod macro_env {
    pub use super::raw_exchange::*;
    pub use super::*;
    pub use port::Port;
    pub use service::export_import::create_skeleton;
    pub use service::id::{IdMap, MethodIdAtomic, ID_ORDERING, MID_REG};
    pub use service::{Cbor as DefaultSerdeFormat, Dispatch, Handle, MethodId};
}

// Re-export macro
pub use remote_trait_object_macro::*;

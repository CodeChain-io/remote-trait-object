use super::Dispatch;
use super::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// An identifier of the skeleton.
///
/// This represents an identifier of a skeleton, and can create a proxy object using [`import_service_from_handle()`]
///
/// Note that you will never need this if you do only plain export & import using [`ServiceRef`], [`ServiceToExport`], or [`ServiceToImport`].
/// See the [module-level documentation] to understand when to use this.
///
/// [`import_service_from_handle()`]: ../raw_exchange/fn.import_service_from_handle.html
/// [`ServiceToExport`]: ../struct.ServiceToExport.html
/// [`ServiceToImport`]: ../struct.ServiceToImport.html
/// [`ServiceRef`]: ../enum.ServiceRef.html
/// [module-level documentation]: ../raw_exchange/index.html
#[derive(PartialEq, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct HandleToExchange(pub(crate) ServiceObjectId);

impl HandleToExchange {
    /// Creates a null handle.
    ///
    /// Any proxy object made from this will always panic for all methods.
    /// If such proxy object is dropped, it won't send any delete request, so never fails.
    ///
    /// It is useful when you have a proxy object which has to be initialized later.
    pub fn create_null() -> Self {
        Self(crate::forwarder::NULL_ID)
    }
}

/// An opaque service to register in the context.
///
/// See the general description of the concept _skeleton_ [here](https://github.com/CodeChain-io/remote-trait-object)
/// and the definition in this crate [here](https://github.com/CodeChain-io/remote-trait-object).
///
/// It is constructed with a service object with whichever smart pointer you want.
/// Depending on use of `&mut self` in the methods in the service trait, some or all `Box<>`, `Arc<>`, `Arc<RwLock<>>` will implement
/// [`IntoSkeleton`] automatically by the proc macro.
/// Please see [this section](https://github.com/CodeChain-io/remote-trait-object) for more detail about smart pointers.
///
/// `Skeleton` is useful when you want to erase the trait, and hold it as an opaque service that will be registered later.
///
/// Note that you will never need this if you do only plain export & import using [`ServiceRef`], [`ServiceToExport`], or [`ServiceToImport`].
/// See the [module-level documentation] to understand when to use this.
///
/// [`IntoSkeleton`]: trait.IntoSkeleton.html
/// [`ServiceToExport`]: ../struct.ServiceToExport.html
/// [`ServiceToImport`]: ../struct.ServiceToImport.html
/// [`ServiceRef`]: ../enum.ServiceRef.html
/// [module-level documentation]: ../raw_exchange/index.html
pub struct Skeleton {
    pub(crate) raw: Arc<dyn Dispatch>,
}

impl Clone for Skeleton {
    /// Clones a `Skeleton`, while sharing the actual single service object.
    ///
    /// This is useful when you want to export a single service object to multiple connections.
    fn clone(&self) -> Self {
        Self {
            raw: Arc::clone(&self.raw),
        }
    }
}

impl Skeleton {
    pub fn new<T: ?Sized + Service>(service: impl IntoSkeleton<T>) -> Self {
        service.into_skeleton()
    }
}

// This belongs to macro_env
pub fn create_skeleton(raw: Arc<dyn Dispatch>) -> Skeleton {
    Skeleton {
        raw,
    }
}

// These traits are associated with some specific service trait.
// These tratis will be implement by `dyn ServiceTrait` where `T = dyn ServiceTrait` as well.
// Macro will implement this trait with the target(expanding) service trait.

/// Conversion into a [`Skeleton`], from a smart pointer of a service object.
///
/// By attaching `[remote_trait_object::service]` on a trait, smart pointers of the trait will automatically implement this.
/// This is required if you want to create a [`Skeleton`] or [`ServiceToExport`].
///
/// [`ServiceToExport`]: ../struct.ServiceToExport.html
// Unused T is for avoiding violation of the orphan rule
// T will be local type for the crate, and that makes it possible to
// impl IntoSkeleton<dyn MyService> for Arc<dyn MyService>
pub trait IntoSkeleton<T: ?Sized + Service> {
    fn into_skeleton(self) -> Skeleton;
}

/// Conversion into a smart pointer of a service object, from [`HandleToExchange`].
///
/// By attaching `[remote_trait_object::service]` on a trait, smart pointers of the trait will automatically implement this.
/// This is required if you want to create a proxy object from [`HandleToExchange`] or [`ServiceToImport`].
///
/// [`ServiceToImport`]: ../struct.ServiceToImport.html
// Unused T is for avoiding violation of the orphan rule, like `IntoSkeleton`
pub trait ImportProxy<T: ?Sized + Service>: Sized {
    fn import_proxy(port: Weak<dyn Port>, handle: HandleToExchange) -> Self;
}

/// Exports a skeleton and returns a handle to it.
///
/// Once you create an instance of skeleton, you will eventually export it calling this.
/// Take the handle to the other side's context and call [`import_service_from_handle`] to import it into a proxy object.
/// If not, the service object will remain in the Context forever doing nothing.
pub fn export_service_into_handle(context: &crate::context::Context, service: Skeleton) -> HandleToExchange {
    context.get_port().upgrade().unwrap().register_service(service.raw)
}

/// Imports a handle into a proxy object.
///
/// Once you receive an instance of [`HandleToExchange`], you will eventually import it calling this.
/// Such handles must be from the other side's context.
pub fn import_service_from_handle<T: ?Sized + Service, P: ImportProxy<T>>(
    context: &crate::context::Context,
    handle: HandleToExchange,
) -> P {
    P::import_proxy(context.get_port(), handle)
}

/// Create a proxy object that always panic for all methods.
///
/// This is same as using [`create_null()`] and [`import_service_from_handle()`] but you don't even have to specify the [`Context`] here.
///
/// [`create_null()`]: ./struct.HandleToExchange.html#method.create_null
/// [`import_service_from_handle()`]: ./fn.import_service_from_handle.html
/// [`Context`]: ../struct.Context.html
pub fn import_null_proxy<T: ?Sized + Service, P: ImportProxy<T>>() -> P {
    P::import_proxy(crate::port::null_weak_port(), HandleToExchange::create_null())
}

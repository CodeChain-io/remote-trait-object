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

use crate::packet::{PacketView, SlotType};
use crate::port::{client::Client, server::Server, BasicPort, Port};
use crate::transport::multiplex::{self, ForwardResult, MultiplexResult, Multiplexer};
use crate::transport::{TransportRecv, TransportSend};
use crate::{raw_exchange::*, Service, ServiceToExport, ServiceToImport};
use parking_lot::Mutex;
use std::sync::{Arc, Barrier, Weak};
use threadpool::ThreadPool;

mod meta_service {
    use super::*;
    /// This is required because of macro
    use crate as remote_trait_object;

    #[remote_trait_object_macro::service]
    pub trait MetaService: Service {
        fn firm_close(&self);
    }

    pub struct MetaServiceImpl {
        barrier: Arc<Barrier>,
    }

    impl MetaServiceImpl {
        pub fn new(barrier: Arc<Barrier>) -> Self {
            Self {
                barrier,
            }
        }
    }

    impl Service for MetaServiceImpl {}

    impl MetaService for MetaServiceImpl {
        fn firm_close(&self) {
            self.barrier.wait();
        }
    }
}
use meta_service::{MetaService, MetaServiceImpl};

/// A configuration of a `remote-trait-object` context.
#[derive(Clone, Debug)]
pub struct Config {
    /// A name that will be appended to the names of various threads spawned by `remote-trait-object`, for an easy debug.
    ///
    /// This can be helpful if you handle multiple contexts of `remote-trait-object`.
    pub name: String,

    /// Number of the maximum of concurrent calls.
    ///
    /// Value of this doesn't have anything to do with the number of threads that would be spawned.
    /// Having a large number of this wouldn't charge any cost except really small additional memory allocation.
    ///
    /// [`server_threads`]: ./struct.Config.html#field.server_threads
    pub call_slots: usize,

    /// A timeout for a remote method call.
    ///
    /// All remote method invocations through your remote object and delete requests (that happens when you drop a remote object)
    /// will have this timeout. If it exceeds, it will cause an error.
    ///
    /// Use `None` for to wait indefinitely.
    pub call_timeout: Option<std::time::Duration>,

    /// A maximum number of services that this context can export.
    pub maximum_services_num: usize,

    /// A shared instance of a thread pool that will be used in call handling
    ///
    /// A `remote-trait-object` context will use this thread pool to handle an incoming method call.
    /// Size of this pool determines the maximum number of concurrent calls that the context can handle.
    /// Note that this pool is wrapped in `Arc`, which means that it can be possibly shared with other places.
    pub thread_pool: Arc<Mutex<threadpool::ThreadPool>>,
}

impl Config {
    pub fn default_setup() -> Self {
        Self {
            name: "my rto".to_owned(),
            call_slots: 512,
            maximum_services_num: 65536,
            call_timeout: Some(std::time::Duration::from_millis(1000)),

            thread_pool: Arc::new(Mutex::new(ThreadPool::new(8))),
        }
    }
}

pub struct Context {
    config: Config,
    multiplexer: Option<Multiplexer>,
    server: Option<Server>,
    port: Option<Arc<BasicPort>>,
    meta_service: Option<Box<dyn MetaService>>,
    firm_close_barrier: Arc<Barrier>,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context").field("config", &self.config).finish()
    }
}

impl Context {
    pub fn new<S: TransportSend + 'static, R: TransportRecv + 'static>(
        config: Config,
        transport_send: S,
        transport_recv: R,
    ) -> Self {
        let null_to_export = crate::service::create_null_service();
        let (ctx, null_to_import): (Self, ServiceToImport<dyn crate::service::NullService>) =
            Self::with_initial_service(config, transport_send, transport_recv, ServiceToExport::new(null_to_export));
        let _null_to_import: Box<dyn crate::service::NullService> = null_to_import.into_proxy();
        ctx
    }

    pub fn with_initial_service_export<S: TransportSend + 'static, R: TransportRecv + 'static, A: ?Sized + Service>(
        config: Config,
        transport_send: S,
        transport_recv: R,
        initial_service: ServiceToExport<A>,
    ) -> Self {
        let (ctx, null_to_import): (Self, ServiceToImport<dyn crate::service::NullService>) =
            Self::with_initial_service(config, transport_send, transport_recv, initial_service);
        let _null_to_import: Box<dyn crate::service::NullService> = null_to_import.into_proxy();
        ctx
    }

    pub fn with_initial_service_import<S: TransportSend + 'static, R: TransportRecv + 'static, B: ?Sized + Service>(
        config: Config,
        transport_send: S,
        transport_recv: R,
    ) -> (Self, ServiceToImport<B>) {
        let null_to_export = crate::service::create_null_service();
        let (ctx, import) =
            Self::with_initial_service(config, transport_send, transport_recv, ServiceToExport::new(null_to_export));
        (ctx, import)
    }

    pub fn with_initial_service<
        S: TransportSend + 'static,
        R: TransportRecv + 'static,
        A: ?Sized + Service,
        B: ?Sized + Service,
    >(
        config: Config,
        transport_send: S,
        transport_recv: R,
        initial_service: ServiceToExport<A>,
    ) -> (Self, ServiceToImport<B>) {
        let firm_close_barrier = Arc::new(Barrier::new(2));

        let MultiplexResult {
            multiplexer,
            request_recv,
            response_recv,
        } = Multiplexer::multiplex::<R, PacketForward>(config.clone(), transport_recv);
        let transport_send = Arc::new(transport_send) as Arc<dyn TransportSend>;

        let client = Client::new(config.clone(), Arc::clone(&transport_send), Box::new(response_recv));
        let port = BasicPort::new(
            config.clone(),
            client,
            (Box::new(MetaServiceImpl::new(Arc::clone(&firm_close_barrier))) as Box<dyn MetaService>).into_skeleton(),
            initial_service.get_raw_export(),
        );
        let server = Server::new(config.clone(), port.get_registry(), transport_send, Box::new(request_recv));

        let port_weak = Arc::downgrade(&port) as Weak<dyn Port>;
        let meta_service = <Box<dyn MetaService> as ImportProxy<dyn MetaService>>::import_proxy(
            Weak::clone(&port_weak),
            crate::service::HandleToExchange(crate::forwarder::META_SERVICE_OBJECT_ID),
        );
        let initial_handle = crate::service::HandleToExchange(crate::forwarder::INITIAL_SERVICE_OBJECT_ID);

        let ctx = Context {
            config,
            multiplexer: Some(multiplexer),
            server: Some(server),
            port: Some(port),
            meta_service: Some(meta_service),
            firm_close_barrier,
        };
        let initial_service = ServiceToImport::from_raw_import(initial_handle, port_weak);
        (ctx, initial_service)
    }

    pub fn register_service(&self, service: Skeleton) -> HandleToExchange {
        self.port.as_ref().unwrap().register_service(service.raw)
    }

    pub(crate) fn get_port(&self) -> Weak<dyn Port> {
        Arc::downgrade(&self.port.clone().expect("It becomes None only when the context is dropped.")) as Weak<dyn Port>
    }

    pub fn clear_service_registry(&mut self) {
        self.port.as_mut().unwrap().clear_registry();
    }

    pub fn disable_garbage_collection(&self) {
        self.port.as_ref().expect("It becomes None only when the context is dropped.").set_no_drop();
    }

    /// TODO: write a good explanation
    /// FIXME: use timeout
    pub fn firm_close(self, _timeout: Option<std::time::Duration>) -> Result<(), Self> {
        let barrier = Arc::clone(&self.firm_close_barrier);
        let t = std::thread::spawn(move || {
            barrier.wait();
        });
        self.meta_service.as_ref().unwrap().firm_close();
        t.join().unwrap();

        Ok(())
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // We have to clean all registered service, as some might hold another proxy object inside, which refers this context's port.
        // For such case, we have to make them be dropped first before we unwrap the Arc<BasicPort>
        self.port.as_ref().unwrap().set_no_drop();
        self.port.as_ref().unwrap().clear_registry();
        drop(self.meta_service.take().unwrap());

        self.multiplexer.take().expect("It becomes None only when the context is dropped.").shutdown();
        // Shutdown server after multiplexer
        self.server.take().expect("It becomes None only when the context is dropped.").shutdown();
        // Shutdown port after multiplexer
        Arc::try_unwrap(self.port.take().expect("It becomes None only when the context is dropped."))
            .unwrap()
            .shutdown();
    }
}

pub struct PacketForward;

impl multiplex::Forward for PacketForward {
    fn forward(packet: PacketView) -> ForwardResult {
        match packet.slot().get_type() {
            SlotType::Request => ForwardResult::Request,
            SlotType::Response => ForwardResult::Response,
        }
    }
}

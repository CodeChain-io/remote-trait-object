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
    /// All remote method invocations through your proxy object and delete requests (that happens when you drop a proxy object)
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

/// One end of a `remote-trait-object` connection.
///
/// If you establish a remote-trait-object connection,
/// there must be two ends and each will be provided as a `Context` to each user on both sides.
///
/// A context holds multiple things to function as a `remote-trait-object` connection end.
/// Since the connection is symmetric, it manages both _server_ and _client_ toward the other end.
/// It also manages a _registry_ that contains all exported services.
/// The server will look up the registry to find a target object for handling an incoming method invocation.
///
/// Note that `remote-trait-object` is a point-to-point connection protocol.
/// Exporting & importing a service are always performed on a specific connection,
/// which is toward the other side, or another instance of `Context`.
///
/// If you created an instance of this, that means you have a connection that has been successfully established **once**,
/// but is not guaranteed to be alive.
/// If the other end (or the other `Context`) is closed, most operations performed on `Context` will just cause an error.
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
    /// Creates a new context without any initial services.
    ///
    /// If you decide to use this, you have to exchange raw [`HandleToExchange`] at least once using a secondary transportation means.
    /// It is really rarely needed, so please consider introducing an initializing service as an initial service, to avoid any raw exchange.
    ///
    /// Please see [`with_initial_service()`] for a general explanation of creation of `Context`.
    ///
    /// [`with_initial_service()`]: ./struct.Context.html#method.with_initial_service
    pub fn new<S: TransportSend + 'static, R: TransportRecv + 'static>(
        config: Config,
        transport_send: S,
        transport_recv: R,
    ) -> Self {
        let null_to_export = crate::service::create_null_service();
        let (ctx, _null_to_import): (Self, ServiceToImport<dyn crate::service::NullService>) =
            Self::with_initial_service(config, transport_send, transport_recv, ServiceToExport::new(null_to_export));
        ctx
    }

    /// Creates a new context only exporting a service, but importing nothing.
    ///
    /// The other end's context must be initialized with `with_initial_service_import()`.
    /// Please see [`with_initial_service()`] for a general explanation of creation of `Context`.
    ///
    /// [`with_initial_service()`]: ./struct.Context.html#method.with_initial_service
    pub fn with_initial_service_export<S: TransportSend + 'static, R: TransportRecv + 'static, A: ?Sized + Service>(
        config: Config,
        transport_send: S,
        transport_recv: R,
        initial_service: ServiceToExport<A>,
    ) -> Self {
        let (ctx, _null_to_import): (Self, ServiceToImport<dyn crate::service::NullService>) =
            Self::with_initial_service(config, transport_send, transport_recv, initial_service);
        ctx
    }

    /// Creates a new context only importing a service, but exporting nothing.
    ///
    /// The other end's context must be initialized with `with_initial_service_export()`.
    /// Please see [`with_initial_service()`] for a general explanation of creation of `Context`.
    ///
    /// [`with_initial_service()`]: ./struct.Context.html#method.with_initial_service
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

    /// Creates a new context exchanging two services, one for export and one for import.
    ///
    /// It takes `initial_service` and registers in it, and passes a `HandleToExchange` internally. (_export_).
    /// Also it receives a `HandleToExchange` from the other side, and makes it into a proxy object. (_import_)
    ///
    /// The other end's context must be initialized with `with_initial_service()` as well, and
    /// such processes will be symmetric for both.
    ///
    /// [`HandleToExchange`]: ../raw_exchange/struct.HandleToExchange.html
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
            HandleToExchange(crate::forwarder::META_SERVICE_OBJECT_ID),
        );
        let initial_handle = HandleToExchange(crate::forwarder::INITIAL_SERVICE_OBJECT_ID);

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

    pub(crate) fn get_port(&self) -> Weak<dyn Port> {
        Arc::downgrade(&self.port.clone().expect("It becomes None only when the context is dropped.")) as Weak<dyn Port>
    }

    /// Clears all service objects in its registry.
    ///
    /// The most usual way of deleting a service object is dropping its proxy object on the client side, and letting it request a delete to the exporter side.
    /// However, in some cases (especially while you're trying to shut down the connection) it is useful to clear all exported service objects
    /// **by the exporter side itself**.
    ///
    /// Note that it will cause an error if the client side drops a proxy object of an already deleted (by this method) service object.
    /// Consider calling [`disable_garbage_collection()`] on the other end if there's such an issue.
    ///
    /// Note also that this might trigger _delete request_ as a side effect since the service object might own a proxy object.
    ///
    /// [`disable_garbage_collection()`]: ./struct.Context.html#method.disable_garbage_collection
    pub fn clear_service_registry(&mut self) {
        self.port.as_mut().unwrap().clear_registry();
    }

    /// Disables all _delete request_ from this end to the other end.
    ///
    /// If you call this, all `drop()` of proxy objects imported from this context won't send a delete request anymore.
    /// This is useful when you're not sure if the connection is still alive, but you have to close your side's context anyway.
    pub fn disable_garbage_collection(&self) {
        self.port.as_ref().expect("It becomes None only when the context is dropped.").set_no_drop();
    }

    /// Closes a context with a firm synchronization with the other end.
    ///
    /// If you call this method, it will block until the other end calls `firm_close()` too.
    /// This is useful when you want to assure that two ends never suffer from 'other end has been closed' error.
    /// If one of the contexts dropped too early, all remote calls (including delete request) from the other end will fail.
    /// To avoid such a situation, consider using this to stay alive as long as it is required.
    ///
    /// FIXME: currently it doesn't use `timeout` and blocks indefinitely.
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
    /// This will delete all service objects after calling `disable_garbage_collection()` internally.
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

use crossbeam::channel::{bounded, Receiver, Select, SelectTimeoutError, Sender};
use remote_trait_object::transport::*;

#[derive(Debug)]
pub struct IntraSend(Sender<Vec<u8>>);

impl TransportSend for IntraSend {
    fn send(&self, data: &[u8], timeout: Option<std::time::Duration>) -> Result<(), TransportError> {
        if let Some(timeout) = timeout {
            // FIXME: Discern timeout error
            self.0.send_timeout(data.to_vec(), timeout).map_err(|_| TransportError::Custom)
        } else {
            self.0.send(data.to_vec()).map_err(|_| TransportError::Custom)
        }
    }

    fn create_terminator(&self) -> Box<dyn Terminate> {
        unimplemented!()
    }
}

pub struct IntraRecv {
    data_receiver: Receiver<Vec<u8>>,
    terminator_receiver: Receiver<()>,
    terminator: Sender<()>,
}

pub struct Terminator(Sender<()>);

impl Terminate for Terminator {
    fn terminate(&self) {
        if let Err(err) = self.0.send(()) {
            debug!("Terminate is called after receiver is closed {}", err);
        };
    }
}

impl TransportRecv for IntraRecv {
    fn recv(&self, timeout: Option<std::time::Duration>) -> Result<Vec<u8>, TransportError> {
        let mut selector = Select::new();
        let data_receiver_index = selector.recv(&self.data_receiver);
        let terminator_index = selector.recv(&self.terminator_receiver);

        let selected_op = if let Some(timeout) = timeout {
            match selector.select_timeout(timeout) {
                Err(SelectTimeoutError) => return Err(TransportError::TimeOut),
                Ok(op) => op,
            }
        } else {
            selector.select()
        };

        let data = match selected_op.index() {
            i if i == data_receiver_index => match selected_op.recv(&self.data_receiver) {
                Ok(data) => data,
                Err(_) => {
                    debug!("Counterparty connection is closed in Intra");
                    return Err(TransportError::Custom)
                }
            },
            i if i == terminator_index => {
                let _ = selected_op
                    .recv(&self.terminator_receiver)
                    .expect("Terminator should be dropped after this thread");
                return Err(TransportError::Termination)
            }
            _ => unreachable!(),
        };

        Ok(data)
    }

    fn create_terminator(&self) -> Box<dyn Terminate> {
        Box::new(Terminator(self.terminator.clone()))
    }
}

pub struct TransportEnds {
    pub send1: IntraSend,
    pub recv1: IntraRecv,
    pub send2: IntraSend,
    pub recv2: IntraRecv,
}

pub fn create() -> TransportEnds {
    let (a_sender, a_receiver) = bounded(256);
    let (a_termination_sender, a_termination_receiver) = bounded(1);
    let (b_sender, b_receiver) = bounded(256);
    let (b_termination_sender, b_termination_receiver) = bounded(1);

    let send1 = IntraSend(b_sender);
    let recv1 = IntraRecv {
        data_receiver: a_receiver,
        terminator_receiver: a_termination_receiver,
        terminator: a_termination_sender,
    };

    let send2 = IntraSend(a_sender);
    let recv2 = IntraRecv {
        data_receiver: b_receiver,
        terminator_receiver: b_termination_receiver,
        terminator: b_termination_sender,
    };

    TransportEnds {
        recv1,
        send1,
        recv2,
        send2,
    }
}

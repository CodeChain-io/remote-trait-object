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

use crossbeam::channel::{bounded, Receiver, Select, SelectTimeoutError, Sender};
use remote_trait_object::transport::*;

pub struct IntraSend(Sender<Vec<u8>>);

impl TransportSend for IntraSend {
    fn send(&self, data: &[u8]) {
        self.0.send(data.to_vec()).unwrap();
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
    type Terminator = Terminator;

    fn recv(&self, timeout: Option<std::time::Duration>) -> Result<Vec<u8>, RecvError> {
        let mut selector = Select::new();
        let data_receiver_index = selector.recv(&self.data_receiver);
        let terminator_index = selector.recv(&self.terminator_receiver);

        let selected_op = if let Some(timeout) = timeout {
            match selector.select_timeout(timeout) {
                Err(SelectTimeoutError) => return Err(RecvError::TimeOut),
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
                    return Err(RecvError::Termination)
                }
            },
            i if i == terminator_index => {
                let _ = selected_op
                    .recv(&self.terminator_receiver)
                    .expect("Terminator should be dropped after this thread");
                return Err(RecvError::Termination)
            }
            _ => unreachable!(),
        };

        Ok(data)
    }

    fn create_terminator(&self) -> Self::Terminator {
        Terminator(self.terminator.clone())
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

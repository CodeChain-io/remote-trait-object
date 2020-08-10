use crate::packet::PacketView;

pub trait Handler: Send + Sync {
    fn handle(&self, input: PacketView) -> Vec<u8>;
}

impl<F> Handler for F
where
    F: Fn(PacketView) -> Vec<u8> + Send + Sync,
{
    fn handle(&self, input: PacketView) -> Vec<u8> {
        self(input)
    }
}

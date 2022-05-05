const MAX_DEVICES: u8 = 127;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[derive(defmt::Format)]
#[derive(Hash32)]
pub struct DevAddress(u8);

pub struct AddressPool {
    pool_bits: u128,
}

impl From<u8> for DevAddress {
    fn from(addr: u8) -> Self {
        if addr > MAX_DEVICES { panic!("USB addr out of range") }
        DevAddress(addr)
    }
}

impl From<DevAddress> for u8 {
    fn from(addr: DevAddress) -> Self {
        addr.0
    }
}

const FULL_POOL: u128 = u128::MAX >> 1;

impl AddressPool {
    pub fn new() -> Self {
        Self {
            pool_bits: FULL_POOL,
        }
    }

    pub fn reset(&mut self) {
        self.pool_bits = FULL_POOL
    }

    pub fn take_next(&mut self) -> Option<DevAddress> {
        let next = self.pool_bits.leading_zeros() as u8;
        if next <= MAX_DEVICES {
            self.pool_bits &= !(1 << (128 - (next + 1)));
            return Some(DevAddress::from(next));
        }
        None
    }

    pub fn put_back(&mut self, addr: DevAddress) {
        let addr: u8 = addr.into();
        if addr <= MAX_DEVICES {
            self.pool_bits |= 1 << addr
        }
    }
}

#[cfg(test)]
mod test {

    use crate::address::AddressPool;

    #[test]
    fn take_one() {
        let mut pool = AddressPool::new();
        assert_eq!(1u8, pool.take_next().unwrap().0)
    }

    #[test]
    fn take_all() {
        let mut pool = AddressPool::new();
        for i in 1u8..127 {
            assert_eq!(i, pool.take_next().unwrap().0);
        }
        assert_eq!(None, pool.take_next())
    }
}
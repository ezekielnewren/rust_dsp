use std::io::ErrorKind;
use std::sync::{Arc, Condvar, Mutex};
use crate::ringbuf::RingBuf;


struct StreamBufInner<T: Copy> {
    ring: RingBuf<T>,
    block_write: bool,
    block_read: bool,
    eof: bool,
}


pub struct StreamBuf<T: Copy> {
    inner: Arc<Mutex<StreamBufInner<T>>>,
    condvar: Condvar,
}


impl<T: Copy> StreamBuf<T> {
    
    pub fn new(capacity: usize, overwrite: bool, block_write: bool, block_read: bool) -> std::io::Result<Self> {
        if overwrite && block_write {
            return Err(std::io::Error::new(ErrorKind::InvalidInput, "overwrite and block_write are mutually exclusive"));
        }
        
        Ok(Self {
            inner: Arc::new(Mutex::new(StreamBufInner {
                ring: RingBuf::new(capacity, overwrite),
                block_write,
                block_read,
                eof: false,
            })),
            condvar: Default::default(),
        })
    }
}

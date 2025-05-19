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
    
    pub fn put(&self, buf: &[T]) -> std::io::Result<usize> {
        let mut inner = self.inner.lock().unwrap();
        if inner.eof {
            return Err(std::io::Error::new(ErrorKind::Other, "output is closed"));
        }
        
        if inner.block_write {
            while inner.ring.len() == inner.ring.capacity() {
                inner = self.condvar.wait(inner).unwrap();
            }
        }
        
        let write = inner.ring.put(buf);
        if write > 0 {
            self.condvar.notify_all();
        }
        Ok(write)
    }
    
    pub fn get(&self, buf: &mut [T]) -> std::io::Result<usize> {
        let mut inner = self.inner.lock().unwrap();
        if inner.block_read {
            while inner.ring.len() == 0 {
                if inner.eof {
                    return Ok(0);
                }
                inner = self.condvar.wait(inner).unwrap();
            }
        }
        
        let read = inner.ring.get(buf);
        if read > 0 {
            self.condvar.notify_all();
        }
        Ok(read)
    }
    
    pub fn set_eof(&self) -> std::io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.eof = true;
        self.condvar.notify_all();
        Ok(())
    }
    
    pub fn is_eof(&self) -> std::io::Result<bool> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.eof)
    }
    
}

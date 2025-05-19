use std::io::{ErrorKind, Read, Write};
use std::sync::{Arc, Condvar, Mutex};
use crate::ringbuf::RingBuf;


struct StreamBuf<T: Copy> {
    ring: RingBuf<T>,
    block_read: bool,
    block_write: bool,
    read_closed: bool,
    write_closed: bool,
}


pub struct StreamReader<T: Copy> {
    reader: Arc<Mutex<StreamBuf<T>>>,
    condvar: Arc<Condvar>,
}


pub struct StreamWriter<T: Copy> {
    writer: Arc<Mutex<StreamBuf<T>>>,
    condvar: Arc<Condvar>,
}



pub fn new_stream<T: Copy>(capacity: usize, overwrite: bool, block_write: bool, block_read: bool) -> std::io::Result<(StreamReader<T>, StreamWriter<T>)> {
    if overwrite && block_write {
        return Err(std::io::Error::new(ErrorKind::InvalidInput, "overwrite and block_write are mutually exclusive"));
    }

    let stream = Arc::new(Mutex::new(StreamBuf {
        ring: RingBuf::new(capacity, overwrite),
        block_read,
        block_write,
        read_closed: false,
        write_closed: false,
    }));

    let condvar = Arc::new(Condvar::default());

    let read = StreamReader {
        reader: Arc::clone(&stream),
        condvar: Arc::clone(&condvar),
    };

    let write = StreamWriter {
        writer: Arc::clone(&stream),
        condvar: Arc::clone(&condvar),
    };

    Ok((read, write))
}


impl<T: Copy> StreamReader<T> {
    pub fn get(&self, buf: &mut [T]) -> std::io::Result<usize> {
        if buf.len() == 0 {
            return Err(std::io::Error::new(ErrorKind::InvalidInput, "buffer is zero length"));
        }

        let mut inner = self.reader.lock().unwrap();
        if inner.block_read {
            while inner.ring.len() == 0 {
                if inner.write_closed {
                    return Ok(0);
                }
                inner = self.condvar.wait(inner).unwrap();
            }
        } else if inner.ring.len() == 0 {
            return Err(std::io::Error::new(ErrorKind::WouldBlock, "buffer empty"));
        }

        let read = inner.ring.get(buf);
        if read > 0 {
            self.condvar.notify_all();
        }
        Ok(read)
    }
}


impl<T: Copy> Drop for StreamReader<T> {
    fn drop(&mut self) {
        let mut inner = self.reader.lock().unwrap();
        inner.read_closed = true;
        self.condvar.notify_all();
    }
}


impl Read for StreamReader<u8> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.get(buf)
    }
}


impl<T: Copy> StreamWriter<T> {
    pub fn put(&self, buf: &[T]) -> std::io::Result<usize> {
        if buf.len() == 0 {
            return Err(std::io::Error::new(ErrorKind::InvalidInput, "buffer is zero length"));
        }

        let mut inner = self.writer.lock().unwrap();
        if inner.write_closed {
            return Err(std::io::Error::new(ErrorKind::Other, "output is closed"));
        }

        if inner.block_write {
            while inner.ring.len() == inner.ring.capacity() {
                inner = self.condvar.wait(inner).unwrap();
            }
        } else if inner.ring.len() == inner.ring.capacity() {
            return Err(std::io::Error::new(ErrorKind::WouldBlock, "buffer full"));
        }

        let write = inner.ring.put(buf);
        if write > 0 {
            self.condvar.notify_all();
        }
        Ok(write)
    }
}

impl<T: Copy> Drop for StreamWriter<T> {
    fn drop(&mut self) {
        let mut inner = self.writer.lock().unwrap();
        inner.write_closed = true;
        self.condvar.notify_all();
    }
}

impl Write for StreamWriter<u8> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.put(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut inner = self.writer.lock().unwrap();
        if inner.block_write {
            while inner.ring.len() > 0 {
                inner = self.condvar.wait(inner).unwrap();
            }
        } else if inner.ring.len() > 0 {
            return Err(std::io::Error::new(ErrorKind::WouldBlock, "buffer is not empty yet"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::streambuf::new_stream;

    #[test]
    fn test_create() -> std::io::Result<()> {
        let (reader, writer) = new_stream::<f32>(1024, true, false, true)?;
        
        let mut buff = Vec::<f32>::new();
        buff.push(0.0);
        
        writer.put(buff.as_slice())?;
        reader.get(buff.as_mut_slice())?;
        
        Ok(())
    }


    #[test]
    fn test_send() -> std::io::Result<()> {
        let (reader, writer) = new_stream::<f32>(1024, true, false, true)?;
        
        let writer_thread = std::thread::spawn(move || {
            let buff = [0f32];
            writer.put(&buff).unwrap();
        });
        
        let reader_thread = std::thread::spawn(move || {
            let mut buff = [0f32];
            reader.get(&mut buff).unwrap();
        });
        
        writer_thread.join().unwrap();
        reader_thread.join().unwrap();
        
        Ok(())
    }
    
}

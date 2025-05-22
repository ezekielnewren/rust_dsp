use std::io::{ErrorKind, Read, Write};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use crate::util::resize_unchecked;

struct StreamBuf<T: Copy> {
    mem: Vec<T>,
    rp: usize,
    wp: usize,
    size: usize,
    overwrite: bool,
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



pub fn new_stream<'a, T: Copy>(capacity: usize, overwrite: bool, block_write: bool, block_read: bool) -> std::io::Result<(StreamReader<T>, StreamWriter<T>)> {
    if overwrite && block_write {
        return Err(std::io::Error::new(ErrorKind::InvalidInput, "overwrite and block_write are mutually exclusive"));
    }

    let mut stream = StreamBuf {
        mem: Vec::new(),
        rp: 0,
        wp: 0,
        size: 0,
        overwrite,
        block_read,
        block_write,
        read_closed: false,
        write_closed: false,
    };
    unsafe { resize_unchecked(&mut stream.mem, capacity); }
    let stream = Arc::new(Mutex::new(stream));

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


pub struct PeekIter<'a, T: Copy> {
    stream: Option<MutexGuard<'a, StreamBuf<T>>>,
    off: usize,
    consume: usize,
}


impl<'a, T: Copy> Iterator for PeekIter<'a, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        let stream = self.stream.as_deref_mut().unwrap();
        if self.off == stream.size {
            return None;
        }

        let rp = (stream.rp + self.off) % stream.mem.capacity();
        let read = std::cmp::min(stream.mem.capacity() - rp, stream.size);
        let ptr = &stream.mem[rp] as *const T;
        self.off += read;
        Some(unsafe { std::slice::from_raw_parts(ptr, read) })
    }
}


impl<'a, T: Copy> PeekIter<'a, T> {

    fn new(mutex: &'a Mutex<StreamBuf<T>>) -> Self {
        Self {
            stream: Some(mutex.lock().unwrap()),
            off: 0,
            consume: 0,
        }
    }
    
    /// Set the total number of items to be marked as consumed when this Iterator is dropped.
    pub fn consume(&mut self, consume: usize) {
        let stream = self.stream.as_deref_mut().unwrap();
        if self.consume > stream.size {
            panic!("consume > len");
        }
        self.consume = consume;
    }
    
    pub fn len(&self) -> usize {
        let stream = self.stream.as_deref().unwrap();
        stream.size
    }
}


impl<'a, T: Copy> Drop for PeekIter<'a, T> {
    fn drop(&mut self) {
        let stream = self.stream.as_deref_mut().unwrap();
        if self.consume > 0 {
            stream.rp = (stream.rp + self.consume) % stream.mem.capacity();
            stream.size -= self.consume;
        }
    }
}


impl<T: Copy> StreamReader<T> {
    pub fn get(&self, buffer: &mut [T]) -> std::io::Result<usize> {
        if buffer.len() == 0 {
            return Err(std::io::Error::new(ErrorKind::InvalidInput, "buffer is zero length"));
        }


        let mut inner = self.reader.lock().unwrap();
        let len = buffer.len();
        let buf = &mut buffer[0..std::cmp::min(len, inner.size)];
        if inner.block_read {
            while inner.size == 0 {
                if inner.write_closed {
                    return Ok(0);
                }
                inner = self.condvar.wait(inner).unwrap();
            }
        } else if inner.size == 0 {
            return Err(std::io::Error::new(ErrorKind::WouldBlock, "buffer empty"));
        }

        let mut off = 0;
        while off < buf.len() {
            let read = std::cmp::min(buf.len() - off, inner.mem.capacity() - inner.rp);
            buf[off..off + read].copy_from_slice(&inner.mem.as_slice()[inner.rp..inner.rp + read]);
            inner.rp = (inner.rp + read) % inner.mem.capacity();
            inner.size -= read;
            off += read;
        }
        if off > 0 {
            self.condvar.notify_all();
        }
        Ok(off)
    }
    
    pub fn peek(&mut self) -> std::io::Result<PeekIter<T>> {
        let mut it = PeekIter::new(self.reader.deref());
        if it.stream.as_ref().unwrap().block_read {
            while it.stream.as_ref().unwrap().size == 0 {
                it.stream = Some(self.condvar.wait(it.stream.take().unwrap()).unwrap());
            }
        } else {
            return Err(std::io::Error::new(ErrorKind::WouldBlock, "buffer is empty"));
        }
        
        Ok(it)
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
    pub fn put(&self, buffer: &[T]) -> std::io::Result<usize> {
        if buffer.len() == 0 {
            return Err(std::io::Error::new(ErrorKind::InvalidInput, "buffer is zero length"));
        }

        let mut inner = self.writer.lock().unwrap();
        let buf = if !inner.overwrite {
            &buffer[..std::cmp::min(buffer.len(), inner.mem.capacity() - inner.size)]
        } else {
            buffer
        };
        if inner.write_closed {
            return Err(std::io::Error::new(ErrorKind::Other, "output is closed"));
        }

        if inner.block_write {
            while inner.size == inner.mem.capacity() {
                inner = self.condvar.wait(inner).unwrap();
            }
        } else if !inner.overwrite && inner.size == inner.mem.capacity() {
            return Err(std::io::Error::new(ErrorKind::WouldBlock, "buffer full"));
        }

        let mut off = 0;
        while off < buf.len() {
            let write = std::cmp::min(buf.len() - off, inner.mem.capacity() - inner.wp);
            let wp = inner.wp;
            inner.mem.as_mut_slice()[wp..wp + write].copy_from_slice(&buf[off..off + write]);
            off += write;
            inner.wp = (inner.wp + write) % inner.mem.capacity();
            inner.size += write;
            if inner.size > inner.mem.capacity() {
                debug_assert!(inner.overwrite);
                inner.rp = (inner.rp + (inner.size - inner.mem.capacity())) % inner.mem.capacity();
                inner.size = inner.mem.capacity();
            }
        }
        if off > 0 {
            self.condvar.notify_all();
        }
        Ok(off)
    }
    
    pub fn drain(&mut self) -> std::io::Result<()> {
        let mut inner = self.writer.lock().unwrap();
        if inner.block_write {
            while inner.size > 0 {
                inner = self.condvar.wait(inner).unwrap();
            }
        } else if inner.size > 0 {
            return Err(std::io::Error::new(ErrorKind::WouldBlock, "buffer is not empty yet"));
        }

        Ok(())
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
        self.drain()
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

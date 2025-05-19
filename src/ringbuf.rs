use std::io::{Read, Write};

pub struct RingBuf<T: Copy> {
    mem: Vec<T>,
    rp: usize,
    wp: usize,
    size: usize,
    overwrite: bool,
}


impl<T: Copy> RingBuf<T> {
    pub fn new(capacity: usize, overwrite: bool) -> Self {
        let mut it = Self {
            mem: Vec::with_capacity(capacity),
            rp: 0,
            wp: 0,
            size: 0,
            overwrite,
        };
        unsafe {
            it.mem.set_len(it.mem.capacity());
        }
        it
    }

    pub fn put(&mut self, mut buf: &[T]) -> usize {
        if !self.overwrite {
            let write = std::cmp::min(buf.len(), self.capacity() - self.size);
            
            if write <= self.capacity() - self.wp {
                let dst = &mut self.mem.as_mut_slice()[self.wp..self.wp + write];
                dst.copy_from_slice(&buf[..write]);
            } else {
                let first = self.capacity() - self.wp;
                let dst = &mut self.mem.as_mut_slice()[self.wp..];
                dst.copy_from_slice(&buf[..first]);
                
                let dst = &mut self.mem.as_mut_slice()[..write - first];
                dst.copy_from_slice(&buf[first..write]);
            }
            
            self.wp = (self.wp + write) % self.capacity();
            self.size += write;
            write
        } else {
            todo!()
        }
    }

    pub fn get(&mut self, buf: &mut [T]) -> usize {
        if !self.overwrite {
            let read = std::cmp::min(buf.len(), self.size);
            
            if read <= self.capacity() - self.rp {
                let src = &self.mem.as_slice()[self.rp..self.rp + read];
                buf[..read].copy_from_slice(src);
            } else {
                let first = self.capacity() - self.rp;
                let src = &self.mem.as_slice()[self.rp..];
                buf[..first].copy_from_slice(src);
                
                let src = &self.mem.as_slice()[..read - first];
                buf[first..read].copy_from_slice(src);
            }
            
            self.rp = (self.rp + read) % self.capacity();
            self.size -= read;
            read
        } else {
            todo!()
        }
    }
    
    pub fn len(&self) -> usize {
        self.size
    }
    
    pub fn capacity(&self) -> usize {
        self.mem.capacity()
    }
    
}


impl Read for RingBuf<u8> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(self.get(buf))
    }
}

impl Write for RingBuf<u8> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(self.put(buf))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}



#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use crate::ringbuf::RingBuf;

    #[test]
    fn test_ringbuf_wrap_around() -> std::io::Result<()> {
        let mut ring = RingBuf::<u8>::new(50, false);
        
        let message = "hello world, this is your programmer writing";
        
        let mut buff = Vec::<u8>::new();
        buff.resize(message.as_bytes().len(), 0);
        
        let _ = ring.write(message.as_bytes())?;
        let _ = ring.read(buff.as_mut_slice())?;

        let _ = ring.write(message.as_bytes())?;
        let _ = ring.read(buff.as_mut_slice())?;
        
        assert_eq!(message.as_bytes(), buff.as_slice());
        
        Ok(())
    }

    #[test]
    fn test_ringbuf_too_big() -> std::io::Result<()> {
        let mut ring = RingBuf::<u8>::new(10, false);

        let expected = "hello world, this is your programmer writing".as_bytes();

        let mut actual = Vec::<u8>::new();
        
        let mut off = 0;
        while off < expected.len() {
            let w = ring.write(&expected[off..])?;
            let end = off + w;
            if actual.capacity() < end {
                actual.reserve(end - actual.capacity());
            }
            unsafe { actual.set_len(end); }
            
            let r = ring.read(&mut actual.as_mut_slice()[off..end])?;
            assert_eq!(w, r);
            off += w;
        }
        
        assert_eq!(expected, actual.as_slice());

        Ok(())
    }
    
}




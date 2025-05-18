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
        let dst = &mut self.mem.as_mut_slice()[self.wp..self.wp + buf.len()];
        dst.copy_from_slice(buf);
        self.wp += buf.len();
        self.size += buf.len();
        buf.len()
    }

    pub fn get(&mut self, buf: &mut [T]) -> usize {
        let src = &self.mem.as_slice()[self.rp..self.rp + buf.len()];
        buf.copy_from_slice(src);
        self.rp += buf.len();
        self.size -= buf.len();
        buf.len()
    }
    
    pub fn len(&self) -> usize {
        self.size
    }
}


impl Read for RingBuf<u8> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.get(buf);
        Ok(buf.len())
    }
}

impl Write for RingBuf<u8> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.put(buf);
        Ok(buf.len())
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
    fn test_ringbuf() -> std::io::Result<()> {
        let mut ring = RingBuf::<u8>::new(1000, true);
        
        let message = "hello world, this is your programmer writing";
        
        let mut buff = Vec::<u8>::new();
        
        let r = ring.write(message.as_bytes())?;
        buff.resize(r, 0);
        
        let w = ring.read(buff.as_mut_slice())?;
        
        assert_eq!(message.as_bytes(), buff.as_slice());
        
        Ok(())
    }
    
}




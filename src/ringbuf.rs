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
            buf = &buf[..std::cmp::min(buf.len(), self.capacity() - self.size)];
        }

        let mut off = 0;
        while off < buf.len() {
            let write = std::cmp::min(buf.len() - off, self.capacity() - self.wp);
            self.mem.as_mut_slice()[self.wp..self.wp + write].copy_from_slice(&buf[off..off + write]);
            off += write;
            self.wp = (self.wp + write) % self.capacity();
            self.size += write;
            if self.size > self.capacity() {
                debug_assert!(self.overwrite);
                self.rp = (self.rp + (self.size - self.capacity())) % self.capacity();
                self.size = self.capacity();
            }
        }

        buf.len()
    }

    pub fn get(&mut self, mut buf: &mut [T]) -> usize {
        let len = buf.len();
        buf = &mut buf[0..std::cmp::min(len, self.size)];
        
        let mut off = 0;
        while off < buf.len() {
            let read = std::cmp::min(buf.len() - off, self.capacity() - self.rp);
            buf[off..off + read].copy_from_slice(&self.mem.as_slice()[self.rp..self.rp + read]);
            self.rp = (self.rp + read) % self.capacity();
            self.size -= read;
            off += read;
        }
        
        buf.len()
    }
    
    pub fn len(&self) -> usize {
        self.size
    }
    
    pub fn capacity(&self) -> usize {
        self.mem.capacity()
    }
    
}


#[cfg(test)]
mod tests {
    use crate::ringbuf::RingBuf;


    #[test]
    fn test_ringbuf_overwrite() -> std::io::Result<()> {
        let capacity = 5;
        let mut ring = RingBuf::<u8>::new(capacity, true);
        ring.rp = 0;
        ring.wp = 0;
        ring.size = 0;
        
        let message = "hello world, this is your programmer writing".as_bytes();

        let mut buff = Vec::<u8>::new();
        buff.resize(message.len(), 0);

        let w = ring.put(message);
        assert_eq!(message.len(), w);
        let r = ring.get(buff.as_mut_slice());
        assert_eq!(capacity, r);
        
        let expected = unsafe {
            std::str::from_utf8_unchecked(&message[message.len() - capacity..])
        };
        
        let actual = unsafe {
            std::str::from_utf8_unchecked(&buff.as_slice()[..capacity])
        };
        
        assert_eq!(expected, actual);

        Ok(())
    }
    
    #[test]
    fn test_ringbuf_wrap_around() -> std::io::Result<()> {
        let mut ring = RingBuf::<u8>::new(50, false);
        
        let message = "hello world, this is your programmer writing";
        
        let mut buff = Vec::<u8>::new();
        buff.resize(message.as_bytes().len(), 0);
        
        let _ = ring.put(message.as_bytes());
        let _ = ring.get(buff.as_mut_slice());

        let _ = ring.put(message.as_bytes());
        let _ = ring.get(buff.as_mut_slice());
        
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
            let w = ring.put(&expected[off..]);
            let end = off + w;
            if actual.capacity() < end {
                actual.reserve(end - actual.capacity());
            }
            unsafe { actual.set_len(end); }
            
            let r = ring.get(&mut actual.as_mut_slice()[off..end]);
            assert_eq!(w, r);
            off += w;
        }
        
        assert_eq!(expected, actual.as_slice());

        Ok(())
    }
    
    #[test]
    fn test_ringbuf_write_to_full_buf_without_overwrite() -> std::io::Result<()> {
        let capacity = 10;
        
        let mut ring = RingBuf::<u8>::new(capacity, false);

        let expected = "hello world, this is your programmer writing".as_bytes();
        
        ring.rp = 3;
        ring.wp = 3;
        ring.size = 0;
        
        let mut off = 0;
        
        off += ring.put(expected);
        assert_eq!(capacity, off);
        
        let w = ring.put(&expected[off..]);
        assert_eq!(0, w);
        
        Ok(())
    }
    
}




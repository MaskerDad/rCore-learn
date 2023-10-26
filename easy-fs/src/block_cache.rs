//!BlockCache => Memory
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
use spin::Mutex;
use super::{BlockDevice, BLOCK_SZ};

pub struct BlockCache {
    cache: [u8; BLOCK_SZ],
    block_id: usize,
    modified: bool,
    block_device: Arc<dyn BlockDevice>,
}

impl BlockCache {
    /// Load a new BlockCache form disk
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>)
        -> Self
    {
        let mut cache: [u8; BLOCK_SZ] = [0u8; BLOCK_SZ];
        block_device.read_lock(block_id, &mut cache);
        Self {
            cache,
            block_id,
            modified: false,
            block_device,
        }
    }

    /// Get the address of an offset inside the cache block data
    fn addr_of_offset(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }

    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size: usize = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr: usize = self.addr_of_offset(offset);
        unsafe {
            &*(addr as *const T)
        }
    }
    
    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr: usize = self.addr_of_offset(offset);
        unsafe {
            &mut *(addr as *mut T)
        }
    }

    pub fn read<T, V>(&self, offset: usize,
                      f: impl FnOnce(&T) -> V) -> V
    {
        f(self.get_ref(offset))
    }

    pub fn modify<T, V>(&mut self, offset: usize,
                        f: impl FnOnce(&mut T) -> V) -> V
    {
        f(self.get_mut(offset))
    }

    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device.write_block(
                self.block_id, 
                &self.cache
            );
        }
    }

}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync()
    }
}

const BLOCK_CACHE_SIZE: usize = 16;
pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
    pub fn new() -> Self {
        queue: VecDeque::new(),
    }

    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>
    ) -> Arc<Mutex<BlockCache>> {
        if let Some(pair) = self.queue.iter()
            .find(|pair| pair.0 == block_id)
        {
            Arc::clone(&pair.1)
        } else {
            // the maximum number of cache blocks is exceeded.
            if self.queue.len() == BLOCK_CACHE_SIZE {
                if let Some((idx, _)) = self
                    .queue.iter().enumerate()
                    .find(|(_, pair)| Arc::strong_count(&pair.1) == 1)
                {
                    self.queue.drain(idx..=idx);
                } else {
                    panic!("Run out of BlockCache!");
                }
            }
            
            let block_cache = Arc::new(Mutex::new(BlockCache::new(
                block_id,
                Arc::clone(&block_device),
            )));
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

lazy_static! {
    /// The global block cache manager
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> =
        Mutex::new(BlockCacheManager::new());
}

pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER
        .lock()
        .get_block_cache(block_id, block_device)
}

/// Sync all block_cache to block_device
pub fn block_cache_sync_all() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in manager.queue.iter() {
        cache.lock().sync();
    }
}

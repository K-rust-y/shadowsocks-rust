// The MIT License (MIT)

// Copyright (c) 2014 Y. T. CHUNG <zonyitoo@gmail.com>

// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::sync::{Arc, Mutex, TaskPool};
use std::thread::Thread;
// use std::sync::atomic::{AtomicOption, SeqCst};
use std::io::net::addrinfo::get_host_addresses;
use std::io::net::ip::IpAddr;

use collect::LruCache;

use config::DEFAULT_DNS_CACHE_CAPACITY;

const TASK_POOL_SIZE: usize = 4;

struct DnsLruCache {
    cache: LruCache<String, Vec<IpAddr>>,
    totally_matched: usize,
    totally_missed: usize,
}

pub struct CachedDns {
    lru_cache: Arc<Mutex<DnsLruCache>>,
    pool: TaskPool,
}

impl CachedDns {
    pub fn new() -> CachedDns {
        CachedDns {
            lru_cache: Arc::new(Mutex::new(DnsLruCache {
                cache: LruCache::new(DEFAULT_DNS_CACHE_CAPACITY),
                totally_missed: 0,
                totally_matched: 0,
            })),
            pool: TaskPool::new(TASK_POOL_SIZE),
        }
    }

    pub fn with_capacity(cache_capacity: usize) -> CachedDns {
        CachedDns {
            lru_cache: Arc::new(Mutex::new(DnsLruCache {
                cache: LruCache::new(cache_capacity),
                totally_missed: 0,
                totally_matched: 0,
            })),
            pool: TaskPool::new(TASK_POOL_SIZE),
        }
    }

    pub fn resolve(&self, addr: &str) -> Option<Vec<IpAddr>> {
        let addr_string = addr.to_string();

        {
            let mut cache = self.lru_cache.lock().unwrap();
            match cache.cache.get(&addr_string).map(|x| x.clone()) {
                Some(addrs) => {
                    cache.totally_matched += 1;
                    debug!("DNS cache matched!: {}", addr_string);
                    debug!("DNS cache matched: {}, missed: {}", cache.totally_matched, cache.totally_missed);
                    return Some(addrs)
                },
                None => {
                    cache.totally_missed += 1;
                    debug!("DNS cache missed!: {}", addr_string);
                    debug!("DNS cache matched: {}, missed: {}", cache.totally_matched, cache.totally_missed);
                }
            }
        }

        let addrs = match get_host_addresses(addr) {
            Ok(addrs) => addrs,
            Err(err) => {
                error!("Failed to resolve {}: {}", addr, err);
                return None;
            }
        };

        let cloned_mutex = self.lru_cache.clone();
        let cloned_addr = addrs.clone();
        self.pool.execute(move || {
            let mut cache = cloned_mutex.lock().unwrap();
            cache.cache.insert(addr_string, cloned_addr);
        });
        Some(addrs)
    }
}

unsafe impl Send for CachedDns {}
unsafe impl Sync for CachedDns {}

#[derive(Clone, Debug, PartialEq)]
pub enum PageOwner {
    Free,
    Kernel,
    Process(u32),
    Reserved,
}

#[derive(Clone, Debug)]
pub struct PageInfo {
    pub owner: PageOwner,
}

pub struct MemStats {
    pub total: usize,
    pub used_kernel: usize,
    pub used_process: usize,
    pub free: usize,
    pub reserved: usize,
}

pub struct MemoryManager {
    pub pages: Vec<PageInfo>,
}

impl MemoryManager {
    pub fn new() -> Self {
        let mut pages = vec![PageInfo { owner: PageOwner::Free }; 256];
        // First 16 pages reserved for kernel
        for i in 0..16 {
            pages[i].owner = PageOwner::Kernel;
        }
        MemoryManager { pages }
    }

    pub fn allocate_pages(&mut self, owner: PageOwner, count: usize) -> Option<usize> {
        let mut start = None;
        let mut run = 0;
        for (i, page) in self.pages.iter().enumerate() {
            if page.owner == PageOwner::Free {
                if run == 0 {
                    start = Some(i);
                }
                run += 1;
                if run == count {
                    break;
                }
            } else {
                run = 0;
                start = None;
            }
        }
        if let Some(s) = start {
            if run == count {
                for i in s..s + count {
                    self.pages[i].owner = owner.clone();
                }
                return Some(s);
            }
        }
        None
    }

    pub fn free_pages(&mut self, start: usize, count: usize) {
        for i in start..std::cmp::min(start + count, self.pages.len()) {
            self.pages[i].owner = PageOwner::Free;
        }
    }

    pub fn free_process_pages(&mut self, pid: u32) {
        for page in self.pages.iter_mut() {
            if page.owner == PageOwner::Process(pid) {
                page.owner = PageOwner::Free;
            }
        }
    }

    pub fn get_stats(&self) -> MemStats {
        let mut used_kernel = 0;
        let mut used_process = 0;
        let mut free = 0;
        let mut reserved = 0;
        for page in &self.pages {
            match page.owner {
                PageOwner::Free => free += 1,
                PageOwner::Kernel => used_kernel += 1,
                PageOwner::Process(_) => used_process += 1,
                PageOwner::Reserved => reserved += 1,
            }
        }
        MemStats {
            total: self.pages.len(),
            used_kernel,
            used_process,
            free,
            reserved,
        }
    }
}

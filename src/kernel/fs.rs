use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum InodeType {
    File,
    Directory,
}

#[derive(Clone, Debug)]
pub struct Inode {
    pub id: u32,
    pub name: String,
    pub inode_type: InodeType,
    pub content: Vec<u8>,
    pub children: Vec<u32>,
    pub parent: Option<u32>,
    pub size: usize,
    pub created_at: u64,
}

pub struct FileSystem {
    pub inodes: HashMap<u32, Inode>,
    pub next_inode: u32,
}

impl FileSystem {
    pub fn new() -> Self {
        let mut fs = FileSystem {
            inodes: HashMap::new(),
            next_inode: 0,
        };

        // Create root directory
        let root_id = fs.alloc_inode("/", InodeType::Directory, None);
        assert_eq!(root_id, 0);

        // Create standard dirs
        fs.mkdir_under(0, "kernel");
        fs.mkdir_under(0, "home");
        fs.mkdir_under(0, "tmp");
        fs.mkdir_under(0, "dev");

        // Create files in /home
        fs.create_file_with_content("/home/readme.txt", "Welcome to rust-minios!\nA micro OS simulator built in Rust.\n");
        fs.create_file_with_content("/home/hello.txt", "Hello, World!\n");
        fs.create_file_with_content("/home/demo.sh",
"# demo.sh - rust-minios demo script
# Usage: run /home/demo.sh

echo === rust-minios Script Demo ===

# Variable assignment and expansion
NAME=rust-minios
echo Hello from $NAME!

# Create directory and file
mkdir /tmp/demo
touch /tmp/demo/output.txt
echo Script started > /tmp/demo/output.txt

# for loop: launch multiple processes
echo --- Launching workers ---
for W in alpha beta gamma
  exec $W
  echo Started worker: $W
end

# Show process list
echo --- Process list ---
ps

# if/else condition: check if file exists
if exists /tmp/demo/output.txt
  echo Output file exists!
  cat /tmp/demo/output.txt
else
  echo Output file not found.
end

# Memory allocation
echo --- Memory allocation ---
malloc 4096
free

echo === Demo complete ===
");

        fs
    }

    fn alloc_inode(&mut self, name: &str, inode_type: InodeType, parent: Option<u32>) -> u32 {
        let id = self.next_inode;
        self.next_inode += 1;
        self.inodes.insert(id, Inode {
            id,
            name: name.to_string(),
            inode_type,
            content: Vec::new(),
            children: Vec::new(),
            parent,
            size: 0,
            created_at: 0,
        });
        id
    }

    fn mkdir_under(&mut self, parent_id: u32, name: &str) -> u32 {
        let id = self.alloc_inode(name, InodeType::Directory, Some(parent_id));
        if let Some(parent) = self.inodes.get_mut(&parent_id) {
            parent.children.push(id);
        }
        id
    }

    fn create_file_under(&mut self, parent_id: u32, name: &str) -> u32 {
        let id = self.alloc_inode(name, InodeType::File, Some(parent_id));
        if let Some(parent) = self.inodes.get_mut(&parent_id) {
            parent.children.push(id);
        }
        id
    }

    fn create_file_with_content(&mut self, path: &str, content: &str) {
        let (dir_path, filename) = split_path(path);
        if let Some(dir_id) = self.resolve_path(dir_path) {
            let file_id = self.create_file_under(dir_id, filename);
            if let Some(inode) = self.inodes.get_mut(&file_id) {
                inode.content = content.as_bytes().to_vec();
                inode.size = content.len();
            }
        }
    }

    pub fn resolve_path(&self, path: &str) -> Option<u32> {
        let path = if path.is_empty() { "/" } else { path };
        if path == "/" {
            return Some(0);
        }
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect();
        let mut current = 0u32;
        for part in parts {
            let inode = self.inodes.get(&current)?;
            let mut found = false;
            for &child_id in &inode.children {
                if let Some(child) = self.inodes.get(&child_id) {
                    if child.name == part {
                        current = child_id;
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                return None;
            }
        }
        Some(current)
    }

    pub fn list_dir(&self, path: &str) -> Vec<String> {
        if let Some(id) = self.resolve_path(path) {
            if let Some(inode) = self.inodes.get(&id) {
                if inode.inode_type == InodeType::Directory {
                    return inode.children.iter()
                        .filter_map(|&cid| self.inodes.get(&cid))
                        .map(|c| {
                            if c.inode_type == InodeType::Directory {
                                format!("{}/", c.name)
                            } else {
                                c.name.clone()
                            }
                        })
                        .collect();
                }
            }
        }
        Vec::new()
    }

    pub fn read_file(&self, path: &str) -> Option<String> {
        let id = self.resolve_path(path)?;
        let inode = self.inodes.get(&id)?;
        if inode.inode_type == InodeType::File {
            Some(String::from_utf8_lossy(&inode.content).to_string())
        } else {
            None
        }
    }

    pub fn write_file(&mut self, path: &str, content: &str) -> bool {
        if let Some(id) = self.resolve_path(path) {
            if let Some(inode) = self.inodes.get_mut(&id) {
                if inode.inode_type == InodeType::File {
                    inode.content = content.as_bytes().to_vec();
                    inode.size = content.len();
                    return true;
                }
            }
        }
        false
    }

    pub fn create_file(&mut self, path: &str) -> bool {
        let (dir_path, filename) = split_path(path);
        if let Some(dir_id) = self.resolve_path(dir_path) {
            // Check not already exists
            if self.resolve_path(path).is_some() {
                return false;
            }
            self.create_file_under(dir_id, filename);
            true
        } else {
            false
        }
    }

    pub fn create_dir(&mut self, path: &str) -> bool {
        let (dir_path, dirname) = split_path(path);
        if let Some(parent_id) = self.resolve_path(dir_path) {
            if self.resolve_path(path).is_some() {
                return false;
            }
            self.mkdir_under(parent_id, dirname);
            true
        } else {
            false
        }
    }

    pub fn delete(&mut self, path: &str) -> bool {
        if path == "/" {
            return false;
        }
        if let Some(id) = self.resolve_path(path) {
            let parent_id = self.inodes.get(&id).and_then(|i| i.parent);
            if let Some(pid) = parent_id {
                if let Some(parent) = self.inodes.get_mut(&pid) {
                    parent.children.retain(|&c| c != id);
                }
            }
            self.inodes.remove(&id);
            true
        } else {
            false
        }
    }

    pub fn get_tree(&self) -> String {
        let mut result = String::new();
        self.build_tree(0, "", &mut result);
        result
    }

    fn build_tree(&self, id: u32, prefix: &str, result: &mut String) {
        if let Some(inode) = self.inodes.get(&id) {
            let display = if inode.inode_type == InodeType::Directory {
                format!("{}/", inode.name)
            } else {
                inode.name.clone()
            };
            if id == 0 {
                result.push_str(&format!("{}\n", display));
            } else {
                result.push_str(&format!("{}{}\n", prefix, display));
            }
            if inode.inode_type == InodeType::Directory {
                let children: Vec<u32> = inode.children.clone();
                for (i, &child_id) in children.iter().enumerate() {
                    let is_last = i == children.len() - 1;
                    let child_prefix = if id == 0 {
                        if is_last { "└── ".to_string() } else { "├── ".to_string() }
                    } else {
                        let continuation = if is_last { "    " } else { "│   " };
                        format!("{}{}", 
                            prefix.trim_end_matches("└── ").trim_end_matches("├── "),
                            if is_last { format!("{}└── ", continuation.trim_end_matches("└── ").trim_end_matches("├── ")) } else { format!("{}├── ", continuation.trim_end_matches("└── ").trim_end_matches("├── ")) }
                        )
                    };
                    self.build_tree(child_id, &child_prefix, result);
                }
            }
        }
    }
}

fn split_path(path: &str) -> (&str, &str) {
    let path = path.trim_end_matches('/');
    if let Some(pos) = path.rfind('/') {
        let dir = if pos == 0 { "/" } else { &path[..pos] };
        let file = &path[pos + 1..];
        (dir, file)
    } else {
        ("/", path)
    }
}

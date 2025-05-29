use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Clone, Debug)]
pub struct FileItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: DateTime<Local>,
    pub permissions: String,
    pub is_symlink: bool,
    pub requires_sudo: bool,
}

pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub items: Vec<FileItem>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub show_hidden: bool,
    pub sort_by: SortBy,
    pub use_sudo: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum SortBy {
    Name,
    Size,
    Modified,
    Type,
}

impl FileBrowser {
    pub fn new() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        let mut browser = Self {
            current_dir: current_dir.clone(),
            items: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            show_hidden: false,
            sort_by: SortBy::Name,
            use_sudo: false,
        };
        browser.refresh()?;
        Ok(browser)
    }
    
    pub fn refresh(&mut self) -> Result<()> {
        self.items = self.read_directory(&self.current_dir)?;
        self.sort_items();
        self.selected_index = self.selected_index.min(self.items.len().saturating_sub(1));
        Ok(())
    }
    
    fn read_directory(&self, path: &Path) -> Result<Vec<FileItem>> {
        let mut items = Vec::new();
        
        // Try normal read first
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied && self.use_sudo => {
                // Try with sudo if permission denied and sudo is enabled
                return self.read_directory_with_sudo(path);
            }
            Err(e) => return Err(e.into()),
        };
        
        // Add parent directory entry if not at root
        if path.parent().is_some() {
            items.push(FileItem {
                name: "..".to_string(),
                path: path.parent().unwrap().to_path_buf(),
                is_dir: true,
                size: 0,
                modified: Local::now(),
                permissions: String::new(),
                is_symlink: false,
                requires_sudo: false,
            });
        }
        
        for entry in entries {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let name = entry.file_name().to_string_lossy().to_string();
            
            // Skip hidden files unless show_hidden is true
            if !self.show_hidden && name.starts_with('.') {
                continue;
            }
            
            let modified = metadata.modified()
                .map(DateTime::<Local>::from)
                .unwrap_or_else(|_| Local::now());
            
            let permissions = self.format_permissions(&metadata);
            
            items.push(FileItem {
                name,
                path: entry.path(),
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified,
                permissions,
                is_symlink: metadata.file_type().is_symlink(),
                requires_sudo: false,
            });
        }
        
        Ok(items)
    }
    
    fn read_directory_with_sudo(&self, path: &Path) -> Result<Vec<FileItem>> {
        // Use sudo ls to read directory contents
        let output = Command::new("sudo")
            .args(["ls", "-la", path.to_str().unwrap()])
            .output()
            .context("Failed to execute sudo ls")?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("sudo ls failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
        
        let mut items = Vec::new();
        
        // Add parent directory
        if path.parent().is_some() {
            items.push(FileItem {
                name: "..".to_string(),
                path: path.parent().unwrap().to_path_buf(),
                is_dir: true,
                size: 0,
                modified: Local::now(),
                permissions: String::new(),
                is_symlink: false,
                requires_sudo: false,
            });
        }
        
        // Parse ls output
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines().skip(1) { // Skip total line
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 9 {
                continue;
            }
            
            let permissions = parts[0].to_string();
            let size: u64 = parts[4].parse().unwrap_or(0);
            let name = parts[8..].join(" ");
            
            // Skip . and .. entries
            if name == "." || name == ".." {
                continue;
            }
            
            // Skip hidden files unless show_hidden is true
            if !self.show_hidden && name.starts_with('.') {
                continue;
            }
            
            let is_dir = permissions.starts_with('d');
            let is_symlink = permissions.starts_with('l');
            
            items.push(FileItem {
                name: name.clone(),
                path: path.join(&name),
                is_dir,
                size,
                modified: Local::now(), // Can't easily parse date from ls
                permissions,
                is_symlink,
                requires_sudo: true,
            });
        }
        
        Ok(items)
    }
    
    fn format_permissions(&self, metadata: &fs::Metadata) -> String {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            format!("{:o}", mode & 0o777)
        }
        #[cfg(not(unix))]
        {
            if metadata.permissions().readonly() {
                "r--".to_string()
            } else {
                "rw-".to_string()
            }
        }
    }
    
    fn sort_items(&mut self) {
        self.items.sort_by(|a, b| {
            // Always keep .. at the top
            if a.name == ".." {
                return std::cmp::Ordering::Less;
            }
            if b.name == ".." {
                return std::cmp::Ordering::Greater;
            }
            
            // Then sort directories before files
            match (a.is_dir, b.is_dir) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            
            // Then sort by selected criteria
            match self.sort_by {
                SortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortBy::Size => b.size.cmp(&a.size),
                SortBy::Modified => b.modified.cmp(&a.modified),
                SortBy::Type => {
                    let ext_a = Path::new(&a.name).extension().and_then(|s| s.to_str()).unwrap_or("");
                    let ext_b = Path::new(&b.name).extension().and_then(|s| s.to_str()).unwrap_or("");
                    ext_a.cmp(ext_b)
                }
            }
        });
    }
    
    pub fn navigate_to(&mut self, path: PathBuf) -> Result<()> {
        self.current_dir = path;
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.refresh()
    }
    
    pub fn enter_selected(&mut self) -> Result<Option<PathBuf>> {
        if let Some(item) = self.get_selected() {
            if item.is_dir {
                self.navigate_to(item.path.clone())?;
                Ok(None)
            } else {
                Ok(Some(item.path.clone()))
            }
        } else {
            Ok(None)
        }
    }
    
    pub fn get_selected(&self) -> Option<&FileItem> {
        self.items.get(self.selected_index)
    }
    
    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }
    
    pub fn move_down(&mut self) {
        if self.selected_index < self.items.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }
    
    pub fn page_up(&mut self, page_size: usize) {
        self.selected_index = self.selected_index.saturating_sub(page_size);
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }
    
    pub fn page_down(&mut self, page_size: usize) {
        let max_index = self.items.len().saturating_sub(1);
        self.selected_index = (self.selected_index + page_size).min(max_index);
    }
    
    pub fn toggle_hidden(&mut self) -> Result<()> {
        self.show_hidden = !self.show_hidden;
        self.refresh()
    }
    
    pub fn toggle_sudo(&mut self) -> Result<()> {
        self.use_sudo = !self.use_sudo;
        self.refresh()
    }
    
    pub fn change_sort(&mut self, sort_by: SortBy) {
        self.sort_by = sort_by;
        self.sort_items();
    }
    
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<Option<PathBuf>> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_up();
                Ok(None)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_down();
                Ok(None)
            }
            KeyCode::PageUp => {
                self.page_up(10);
                Ok(None)
            }
            KeyCode::PageDown => {
                self.page_down(10);
                Ok(None)
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                self.enter_selected()
            }
            KeyCode::Char('h') => {
                // Go to parent directory
                if let Some(parent) = self.current_dir.parent() {
                    self.navigate_to(parent.to_path_buf())?;
                }
                Ok(None)
            }
            KeyCode::Char('.') => {
                self.toggle_hidden()?;
                Ok(None)
            }
            KeyCode::Char('s') => {
                self.toggle_sudo()?;
                Ok(None)
            }
            KeyCode::Char('n') => {
                self.change_sort(SortBy::Name);
                Ok(None)
            }
            KeyCode::Char('S') => {
                self.change_sort(SortBy::Size);
                Ok(None)
            }
            KeyCode::Char('m') => {
                self.change_sort(SortBy::Modified);
                Ok(None)
            }
            KeyCode::Char('t') => {
                self.change_sort(SortBy::Type);
                Ok(None)
            }
            _ => Ok(None),
        }
    }
    
    pub fn format_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = size as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}
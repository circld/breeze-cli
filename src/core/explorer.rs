use crate::error::ExplorerError;
use crate::fs::list_directory;
use std::fs::DirEntry;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Explorer {
    pub current_dir: PathBuf,
}

impl Explorer {
    pub fn new(directory: PathBuf) -> Result<Self, ExplorerError> {
        if !directory.exists() {
            return Err(ExplorerError::InvalidDirectory(
                directory.to_string_lossy().to_string(),
            ));
        }

        Ok(Explorer {
            current_dir: directory.canonicalize()?,
        })
    }

    pub fn ls(&self) -> Result<Vec<DirEntry>, ExplorerError> {
        list_directory(&self.current_dir)
    }

    pub fn cd(&mut self, directory: PathBuf) -> Result<Vec<DirEntry>, ExplorerError> {
        self.current_dir = directory.canonicalize()?;
        self.ls()
    }

    pub fn cwd(&self) -> String {
        self.current_dir.to_string_lossy().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_new_with_valid_directory() {
        let temp_dir = TempDir::new().unwrap();
        let explorer = Explorer::new(temp_dir.path().to_path_buf());
        assert!(explorer.is_ok());
    }

    #[test]
    fn test_new_with_nonexistent_directory() {
        let result = Explorer::new(PathBuf::from("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err());
        match result {
            Err(ExplorerError::InvalidDirectory(path)) => {
                assert_eq!(path, "/nonexistent/path/that/does/not/exist");
            }
            _ => panic!("Expected InvalidDirectory error"),
        }
    }

    #[test]
    fn test_new_canonicalizes_path() {
        let temp_dir = TempDir::new().unwrap();
        let relative_path = temp_dir.path().join(".");
        let explorer = Explorer::new(relative_path).unwrap();
        let cwd = explorer.cwd();
        let expected = temp_dir.path().canonicalize().unwrap().to_string_lossy().to_string();
        assert_eq!(cwd, expected);
    }

    #[test]
    fn test_ls_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let entries = explorer.ls().unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_ls_with_files_and_directories() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let entries = explorer.ls().unwrap();
        assert_eq!(entries.len(), 3);

        let names: Vec<String> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"subdir".to_string()));
        assert!(names.contains(&"file1.txt".to_string()));
        assert!(names.contains(&"file2.txt".to_string()));
    }

    #[test]
    fn test_ls_sorted_alphabetically() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("zebra.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("apple.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("banana.txt"), "content").unwrap();

        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let entries = explorer.ls().unwrap();
        let names: Vec<String> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        assert_eq!(names, vec!["apple.txt", "banana.txt", "zebra.txt"]);
    }

    #[test]
    fn test_cd_to_valid_subdirectory() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let mut explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let result = explorer.cd(subdir.clone());
        assert!(result.is_ok());
        assert_eq!(explorer.cwd(), subdir.canonicalize().unwrap().to_string_lossy());
    }

    #[test]
    fn test_cd_to_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let mut explorer = Explorer::new(subdir.clone()).unwrap();
        let result = explorer.cd(temp_dir.path().to_path_buf());
        assert!(result.is_ok());
        assert_eq!(
            explorer.cwd(),
            temp_dir.path().canonicalize().unwrap().to_string_lossy()
        );
    }

    #[test]
    fn test_cd_to_nonexistent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let result = explorer.cd(PathBuf::from("/nonexistent/directory"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cd_returns_new_directory_listing() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file_in_subdir.txt"), "content").unwrap();

        let mut explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        let entries = explorer.cd(subdir).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].file_name().to_string_lossy(),
            "file_in_subdir.txt"
        );
    }

    #[test]
    fn test_cwd_returns_current_directory() {
        let temp_dir = TempDir::new().unwrap();
        let explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(
            explorer.cwd(),
            temp_dir.path().canonicalize().unwrap().to_string_lossy()
        );
    }

    #[test]
    fn test_cwd_updates_after_cd() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();

        let mut explorer = Explorer::new(temp_dir.path().to_path_buf()).unwrap();
        explorer.cd(subdir.clone()).unwrap();
        assert_eq!(explorer.cwd(), subdir.canonicalize().unwrap().to_string_lossy());
    }
}

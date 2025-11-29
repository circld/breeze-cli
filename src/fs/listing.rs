use crate::error::ExplorerError;
use std::fs::{self, DirEntry};
use std::path::Path;

pub fn list_directory<P: AsRef<Path>>(path: P) -> Result<Vec<DirEntry>, ExplorerError> {
    let entries = fs::read_dir(path)?;
    let mut files = Vec::new();

    for entry in entries {
        files.push(entry?);
    }

    files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_list_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = list_directory(temp_dir.path());
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_list_directory_with_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("file3.txt"), "content").unwrap();

        let entries = list_directory(temp_dir.path()).unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_list_directory_with_subdirectories() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join("dir1")).unwrap();
        fs::create_dir(temp_dir.path().join("dir2")).unwrap();

        let entries = list_directory(temp_dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_list_directory_mixed_files_and_dirs() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("file.txt"), "content").unwrap();

        let entries = list_directory(temp_dir.path()).unwrap();
        assert_eq!(entries.len(), 2);

        let names: Vec<String> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"subdir".to_string()));
        assert!(names.contains(&"file.txt".to_string()));
    }

    #[test]
    fn test_list_directory_sorted_alphabetically() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("zebra.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("apple.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("banana.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("cherry.txt"), "content").unwrap();

        let entries = list_directory(temp_dir.path()).unwrap();
        let names: Vec<String> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        assert_eq!(
            names,
            vec!["apple.txt", "banana.txt", "cherry.txt", "zebra.txt"]
        );
    }

    #[test]
    fn test_list_directory_sorted_case_sensitive() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("Zebra.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("apple.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("Banana.txt"), "content").unwrap();

        let entries = list_directory(temp_dir.path()).unwrap();
        let names: Vec<String> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        assert_eq!(names, vec!["Banana.txt", "Zebra.txt", "apple.txt"]);
    }

    #[test]
    fn test_list_nonexistent_directory() {
        let result = list_directory("/nonexistent/directory/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_directory_with_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join(".hidden"), "content").unwrap();
        fs::write(temp_dir.path().join("visible.txt"), "content").unwrap();

        let entries = list_directory(temp_dir.path()).unwrap();
        assert_eq!(entries.len(), 2);

        let names: Vec<String> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&".hidden".to_string()));
        assert!(names.contains(&"visible.txt".to_string()));
    }

    #[test]
    fn test_list_directory_with_special_characters() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file with spaces.txt"), "content").unwrap();
        fs::write(temp_dir.path().join("file-with-dashes.txt"), "content").unwrap();

        let entries = list_directory(temp_dir.path()).unwrap();
        assert_eq!(entries.len(), 2);

        let names: Vec<String> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"file with spaces.txt".to_string()));
        assert!(names.contains(&"file-with-dashes.txt".to_string()));
    }

    #[test]
    fn test_list_directory_accepts_string_path() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file.txt"), "content").unwrap();

        let path_str = temp_dir.path().to_str().unwrap();
        let entries = list_directory(path_str).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_list_directory_accepts_pathbuf() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file.txt"), "content").unwrap();

        let path_buf = temp_dir.path().to_path_buf();
        let entries = list_directory(path_buf).unwrap();
        assert_eq!(entries.len(), 1);
    }
}

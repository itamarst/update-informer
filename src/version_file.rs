use crate::{Error, Package};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::Duration;
use std::{fs, io};

#[derive(Debug, PartialEq)]
pub(crate) struct VersionFile<'a> {
    path: PathBuf,
    version: &'a str,
}

const VERSION_SUFFIX: &str = "latest-version";

impl<'a> VersionFile<'a> {
    pub(crate) fn new(registry: &str, pkg: &Package, version: &'a str) -> Result<Self, Error> {
        let owner = if let Some(owner) = pkg.owner {
            format!("{}-", owner)
        } else {
            "".to_string()
        };
        let file_name = format!("{}-{}{}-{}", registry, owner, pkg.name, VERSION_SUFFIX);
        let path = cache_path()?.join(file_name);

        Ok(Self { path, version })
    }

    pub(crate) fn last_modified(&self) -> Result<Duration, Error> {
        let metadata = match fs::metadata(&self.path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                self.write_version(&self.version)?;
                return Ok(Duration::default());
            }
            Err(e) => return Err(e.into()),
        };

        let last_modified = metadata.modified()?.elapsed();
        Ok(last_modified.unwrap_or_default())
    }

    pub(crate) fn recreate_file(&self) -> io::Result<()> {
        fs::remove_file(&self.path)?;
        self.write_version(&self.version)
    }

    pub(crate) fn write_version<V: AsRef<str>>(&self, version: V) -> io::Result<()> {
        fs::write(&self.path, version.as_ref())
    }

    pub(crate) fn get_version(&self) -> io::Result<String> {
        fs::read_to_string(&self.path)
    }
}

#[cfg(not(test))]
fn cache_path() -> Result<PathBuf, Error> {
    let project_dir = directories::ProjectDirs::from("", "", "update-informer-rs")
        .map_or(Err("Unable to find cache directory"), Ok)?;
    let directory = project_dir.cache_dir().to_path_buf();
    fs::create_dir_all(&directory)?;
    Ok(directory)
}

#[cfg(test)]
fn cache_path() -> Result<PathBuf, Error> {
    Ok(std::env::temp_dir().join("update-informer-test"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::within_test_dir;

    #[test]
    fn new_test() {
        let pkg = Package::new("repo");
        let version_file1 = VersionFile::new("myreg", &pkg, "0.1.0").unwrap();
        let version_file2 = VersionFile {
            path: cache_path().unwrap().join("myreg-repo-latest-version"),
            version: "0.1.0",
        };

        assert_eq!(version_file1, version_file2);
    }

    #[test]
    fn create_version_file_twice_test() {
        let pkg = Package::new("repo");
        let version_file1 = VersionFile::new("reg", &pkg, "0.1.0").unwrap();
        let version_file2 = VersionFile::new("reg", &pkg, "0.1.0").unwrap();
        assert_eq!(version_file1, version_file2);
    }

    #[test]
    fn last_modified_file_exists_test() {
        within_test_dir(|path| {
            fs::write(&path, "0.1.0").expect("creates test file");

            let version_file = VersionFile {
                path,
                version: "0.1.0",
            };

            let last_modified = version_file.last_modified();
            assert!(last_modified.is_ok());
            assert!(!last_modified.unwrap().is_zero());
        });
    }

    #[test]
    fn last_modified_file_not_exists_test() {
        within_test_dir(|path| {
            let version_file = VersionFile {
                path: path.clone(),
                version: "0.1.0",
            };

            let last_modified = version_file.last_modified();
            assert!(last_modified.is_ok());
            assert!(last_modified.unwrap().is_zero());

            let version = fs::read_to_string(&path).expect("read test file");
            assert_eq!(version, "0.1.0");
        });
    }

    #[test]
    fn recreate_file_test() {
        within_test_dir(|path| {
            fs::write(&path, "0.1.0").expect("creates test file");

            let version_file = VersionFile {
                path: path.clone(),
                version: "1.0.0",
            };

            let result = version_file.recreate_file();
            assert!(result.is_ok());

            let version = fs::read_to_string(&path).expect("read test file");
            assert_eq!(version, "1.0.0");
        });
    }

    #[test]
    fn write_version_test() {
        within_test_dir(|path| {
            fs::write(&path, "1.0.0").expect("creates test file");

            let version_file = VersionFile {
                path: path.clone(),
                version: "1.0.0",
            };

            let result = version_file.write_version("2.0.0");
            assert!(result.is_ok());

            let version = fs::read_to_string(&path).expect("read test file");
            assert_eq!(version, "2.0.0");
        });
    }

    #[test]
    fn get_version_file_exists_test() {
        within_test_dir(|path| {
            fs::write(&path, "1.0.0").expect("creates test file");

            let version_file = VersionFile {
                path: path.clone(),
                version: "1.0.0",
            };

            let result = version_file.get_version();
            assert!(result.is_ok());

            let version = fs::read_to_string(&path).expect("read test file");
            assert_eq!(version, "1.0.0");
        });
    }

    #[test]
    fn get_version_file_not_exists_test() {
        within_test_dir(|path| {
            let version_file = VersionFile {
                path: path.clone(),
                version: "1.0.0",
            };

            let result = version_file.get_version();
            assert!(result.is_err());
        });
    }
}

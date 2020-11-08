use std::fs;
use std::path::Path;

use anyhow::{anyhow, ensure, Context as _, Result};
use toml_edit::{self, Document};

use crate::error::Ros2wsError::InvalidManifestFile;

const WORKSPACE_KEY: &str = "workspace";
const MEMBERS_KEY: &str = "members";
const PATCH_KEY: &str = "patch";
const CRATES_IO_KEY: &str = "crates-io";

#[derive(Debug)]
pub struct Manifest {
    data: Document,
}

impl Manifest {
    pub(crate) fn read_from(src: &impl AsRef<Path>) -> Result<Self> {
        ensure_abs_path(&src)?;
        ensure_is_file(&src)?;

        let src = src.as_ref();
        let data = fs::read_to_string(src)
            .with_context(|| format!("failed to read file {}", src.display()))?
            .parse::<Document>()
            .with_context(|| format!("failed to parse toml file {}", src.display()))?;
        Ok(Self { data })
    }

    pub(crate) fn write_to(&self, dst: &impl AsRef<Path>) -> Result<()> {
        ensure_abs_path(dst)?;
        ensure_is_file(dst)?;

        let dst = dst.as_ref();
        fs::write(dst, self.data.to_string())
            .with_context(|| format!("failed to write {}", dst.display()))?;
        Ok(())
    }

    pub(crate) fn add_member(&mut self, path: impl AsRef<Path>) -> Result<()> {
        ensure_abs_path(&path)?;

        let path = path.as_ref();
        let path = path
            .to_str()
            .ok_or_else(|| anyhow!("fail to convert to UTF-8 string {}", path.display()))?;
        let members = self.get_workspace_members_section()?;
        let members_array_mut = members.as_array_mut().unwrap();
        if members_array_mut
            .iter()
            .all(|v| v.as_str().unwrap() != path)
        {
            members_array_mut
                .push(toml_edit::Value::from(path.to_string()))
                .unwrap();
        }
        Ok(())
    }

    fn get_workspace_section(&mut self) -> Result<&mut toml_edit::Item> {
        let workspace = self.data[WORKSPACE_KEY].or_insert(toml_edit::table());
        ensure!(
            workspace.is_table(),
            InvalidManifestFile(WORKSPACE_KEY.into(), "table".into())
        );
        Ok(workspace)
    }

    fn get_workspace_members_section(&mut self) -> Result<&mut toml_edit::Item> {
        let workspace = self.get_workspace_section()?;
        let members =
            workspace[MEMBERS_KEY].or_insert(toml_edit::value(toml_edit::Array::default()));
        ensure!(
            members.is_array(),
            InvalidManifestFile(format!("{}.{}", WORKSPACE_KEY, MEMBERS_KEY), "array".into())
        );
        Ok(members)
    }

    pub(crate) fn add_patch(&mut self, crates_name: &str, path: impl AsRef<Path>) -> Result<()> {
        ensure!(!crates_name.is_empty(), "crate name should not be empty");
        ensure_abs_path(&path)?;

        let path = path.as_ref();
        let path = path
            .to_str()
            .ok_or_else(|| anyhow!("fail to convert to UTF-8 string {}", path.display()))?;
        let crates_io = self.get_patch_crates_io_section()?;
        let crates_io_table_mut = crates_io.as_table_mut().unwrap();
        let mut table = toml_edit::InlineTable::default();
        table.get_or_insert("path", toml_edit::Value::from(path.to_string()));
        crates_io_table_mut[crates_name] = toml_edit::Item::Value(toml_edit::Value::from(table));
        Ok(())
    }

    fn get_patch_section(&mut self) -> Result<&mut toml_edit::Item> {
        let patch = self.data[PATCH_KEY].or_insert(toml_edit::table());
        ensure!(
            patch.is_table(),
            InvalidManifestFile(PATCH_KEY.into(), "table".into())
        );
        patch.as_table_mut().unwrap().set_implicit(true);
        Ok(patch)
    }

    fn get_patch_crates_io_section(&mut self) -> Result<&mut toml_edit::Item> {
        let patch = self.get_patch_section()?;
        let crates_io = patch[CRATES_IO_KEY].or_insert(toml_edit::table());
        ensure!(
            crates_io.is_table(),
            InvalidManifestFile(format!("{}.{}", PATCH_KEY, CRATES_IO_KEY), "table".into())
        );

        Ok(crates_io)
    }

    #[cfg(test)]
    fn init() -> Self {
        Self {
            data: Document::new(),
        }
    }
}

fn ensure_abs_path(path: &impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    ensure!(path.is_absolute(), "not absolute path {}", path.display());
    Ok(())
}

fn ensure_is_file(path: &impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    ensure!(!path.is_dir(), "not file {}", path.display());
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    use std::path::PathBuf;

    use tempfile::TempDir;

    const MANIFEST_FILENAME: &str = "Cargo.toml";

    #[test]
    fn test_manifest_write_to_error() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file = temp_dir.path().join(MANIFEST_FILENAME);
        fs::write(&file, "")?;
        let dummy_file = temp_dir.path().join("dummy.txt");
        fs::write(&dummy_file, "")?;

        let manifest = Manifest::init();
        // relative path
        assert!(manifest.write_to(&PathBuf::from("./relative")).is_err());
        assert!(manifest.write_to(&PathBuf::from("../relative")).is_err());
        // directory
        assert!(manifest.write_to(&temp_dir.path()).is_err());

        Ok(())
    }

    #[test]
    fn test_manifest_add_member_from_empty() -> Result<()> {
        static ANS: &str = r#"
[workspace]
members = ["/test1", "/test2/test2"]
"#;

        let temp_dir = TempDir::new()?;
        let file = temp_dir.path().join(MANIFEST_FILENAME);
        fs::write(&file, "")?;

        let mut manifest = Manifest::init();
        manifest.add_member(PathBuf::from("/test1"))?;
        manifest.add_member(PathBuf::from("/test2/test2"))?;
        manifest.write_to(&file)?;

        let data = fs::read_to_string(file)?;
        assert_eq!(data, ANS);

        Ok(())
    }

    #[test]
    fn test_manifest_add_member_with_content() -> Result<()> {
        static CONTENT: &str = r#"
[workspace]
members = ["/test1"]

[patch.crate-io]
hoge = { path = "/hoge/hoge" }
"#;

        static ANS: &str = r#"
[workspace]
members = ["/test1", "/test2/test2"]

[patch.crate-io]
hoge = { path = "/hoge/hoge" }
"#;

        let temp_dir = TempDir::new()?;
        let file = temp_dir.path().join(MANIFEST_FILENAME);
        fs::write(&file, CONTENT)?;

        let mut manifest = Manifest::read_from(&file)?;
        manifest.add_member(PathBuf::from("/test2/test2"))?;
        manifest.write_to(&file)?;

        let data = fs::read_to_string(file)?;
        assert_eq!(data, ANS);

        Ok(())
    }

    #[test]
    fn test_manifest_add_member_with_duplicate() -> Result<()> {
        static ANS: &str = r#"
[workspace]
members = ["/test1", "/test2/test2"]
"#;

        let temp_dir = TempDir::new()?;
        let file = temp_dir.path().join(MANIFEST_FILENAME);
        fs::write(&file, "")?;

        let mut manifest = Manifest::read_from(&file)?;
        manifest.add_member(PathBuf::from("/test1"))?;
        manifest.add_member(PathBuf::from("/test2/test2"))?;
        manifest.add_member(PathBuf::from("/test1"))?;
        manifest.add_member(PathBuf::from("/test2/test2"))?;
        manifest.write_to(&file)?;

        let data = fs::read_to_string(file)?;
        assert_eq!(data, ANS);

        Ok(())
    }

    #[test]
    fn test_manifest_add_member_error() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file = temp_dir.path().join(MANIFEST_FILENAME);
        fs::write(&file, "")?;

        let mut manifest = Manifest::read_from(&file)?;
        // relative path
        assert!(manifest.add_member(PathBuf::from("test")).is_err());

        Ok(())
    }

    #[test]
    fn test_manifest_add_patch_from_empty() -> Result<()> {
        static ANS: &str = r#"
[patch.crates-io]
hoge ={path = "/hoge"}
fuga ={path = "/fuga/fuga"}
"#;

        let temp_dir = TempDir::new()?;
        let file = temp_dir.path().join(MANIFEST_FILENAME);
        fs::write(&file, "")?;

        let mut manifest = Manifest::read_from(&file)?;
        manifest.add_patch("hoge", PathBuf::from("/hoge"))?;
        manifest.add_patch("fuga", PathBuf::from("/fuga/fuga"))?;
        manifest.write_to(&file)?;

        let data = fs::read_to_string(file)?;
        assert_eq!(data, ANS);

        Ok(())
    }

    #[test]
    fn test_manifest_add_patch_with_content() -> Result<()> {
        static CONTENT: &str = r#"
[workspace]
members = ["/test1"]

[patch.crates-io]
hoge = { path = "/hoge/hoge" }
"#;

        static ANS: &str = r#"
[workspace]
members = ["/test1"]

[patch.crates-io]
hoge = { path = "/hoge/hoge" }
fuga ={path = "/fuga/fuga"}
"#;

        let temp_dir = TempDir::new()?;
        let file = temp_dir.path().join(MANIFEST_FILENAME);
        fs::write(&file, CONTENT)?;

        let mut manifest = Manifest::read_from(&file)?;
        manifest.add_patch("fuga", PathBuf::from("/fuga/fuga"))?;
        manifest.write_to(&file)?;

        let data = fs::read_to_string(file)?;
        assert_eq!(data, ANS);

        Ok(())
    }

    #[test]
    fn test_manifest_add_patch_with_override() -> Result<()> {
        static ANS: &str = r#"
[patch.crates-io]
hoge ={path = "/piyo"}
"#;

        let temp_dir = TempDir::new()?;
        let file = temp_dir.path().join(MANIFEST_FILENAME);
        fs::write(&file, "")?;

        let mut manifest = Manifest::read_from(&file)?;
        manifest.add_patch("hoge", PathBuf::from("/hoge"))?;
        manifest.add_patch("hoge", PathBuf::from("/fuga"))?;
        manifest.add_patch("hoge", PathBuf::from("/piyo"))?;
        manifest.write_to(&file)?;

        let data = fs::read_to_string(file)?;
        assert_eq!(data, ANS);

        Ok(())
    }

    #[test]
    fn test_manifest_add_patch_error() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file = temp_dir.path().join(MANIFEST_FILENAME);
        fs::write(&file, "")?;

        let mut manifest = Manifest::read_from(&file)?;
        // empty crate name
        assert!(manifest.add_patch("", PathBuf::from("/test")).is_err());
        // relative path
        assert!(manifest.add_patch("hoge", PathBuf::from("hoge")).is_err());
        assert!(manifest.add_patch("hoge", PathBuf::from("./hoge")).is_err());
        assert!(manifest
            .add_patch("hoge", PathBuf::from("../hoge"))
            .is_err());

        Ok(())
    }
}
